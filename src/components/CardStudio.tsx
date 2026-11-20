import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import type { Project } from '../lib/types';
import { Button, Field, Modal, Segmented } from './ui';
import { Icon } from './Icon';
import {
  CARD_RATIOS,
  CARD_THEMES,
  drawCard,
  extractCardSubtext,
  extractCardText,
  type CardRatio,
} from '../lib/card-renderer';

function canvasToBlob(canvas: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (blob) resolve(blob);
      else reject(new Error('导出 PNG 失败'));
    }, 'image/png');
  });
}

export function CardStudio({
  project,
  open,
  initialText = '',
  initialTitle = '',
  onClose,
  onSaved,
}: {
  project: Project;
  open: boolean;
  initialText?: string;
  initialTitle?: string;
  onClose: () => void;
  onSaved: (assetId: string) => void;
}) {
  const app = useApp();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [text, setText] = useState('');
  const [subtext, setSubtext] = useState('');
  const [title, setTitle] = useState('');
  const [themeId, setThemeId] = useState(CARD_THEMES[0].id);
  const [ratio, setRatio] = useState<CardRatio>('3:4');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  const theme = CARD_THEMES.find((item) => item.id === themeId) ?? CARD_THEMES[0];

  useLayoutEffect(() => {
    if (!open) return;
    const source = initialText || '';
    setText(extractCardText(source));
    setSubtext(extractCardSubtext(source));
    setTitle(initialTitle || extractCardText(source) || '文卡图');
    setThemeId(CARD_THEMES[0].id);
    setRatio('3:4');
    setError('');
  }, [open, initialText, initialTitle]);

  useLayoutEffect(() => {
    if (!open || !canvasRef.current) return;
    drawCard(canvasRef.current, {
      text,
      subtext,
      theme,
      ratio,
      topLabel: 'A NOTE FROM 媒材工坊',
    });
  }, [open, text, subtext, theme, ratio]);

  const exportBlob = async () => {
    if (!canvasRef.current) throw new Error('画布尚未准备好');
    return canvasToBlob(canvasRef.current);
  };

  const download = async () => {
    try {
      const blob = await exportBlob();
      const url = URL.createObjectURL(blob);
      const link = document.createElement('a');
      link.href = url;
      link.download = `${(title || '文卡图').replace(/[\\/:*?"<>|]/g, '-')}.png`;
      link.click();
      URL.revokeObjectURL(url);
      app.toast({ message: 'PNG 已导出', tone: 'success' });
    } catch (err) {
      setError(err instanceof Error ? err.message : '导出失败');
    }
  };

  const save = async () => {
    if (!text.trim()) {
      setError('先写下一句主张。');
      return;
    }
    setSaving(true);
    setError('');
    try {
      const blob = await exportBlob();
      const filename = `${(title || text).replace(/[\\/:*?"<>|]/g, '-').slice(0, 40)}.png`;
      const file = new File([blob], filename, { type: 'image/png' });
      const asset = await api.uploadAsset(project.id, 'card', file, {
        title: title.trim() || text.trim().slice(0, 40),
      });
      await Promise.all([app.refreshProjects(), app.refreshWorkspace()]);
      app.toast({ message: '文卡图已存入项目', tone: 'success' });
      onSaved(asset.id);
      onClose();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : err instanceof Error ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="文卡图 Studio"
      subtitle="把一句值得记住的话，排成可以保存的卡片。导出为 1080px PNG。"
      size="full"
      class="card-studio-modal"
      footer={
        <>
          <span class="shortcut-hint">输出 PNG · 最高 1080 × 1440</span>
          <div class="modal-actions">
            <Button variant="secondary" icon="download" onClick={download}>
              下载 PNG
            </Button>
            <Button variant="primary" icon="image" loading={saving} onClick={save}>
              存入项目
            </Button>
          </div>
        </>
      }
    >
      <div class="card-studio">
        <div class="card-stage">
          <div class="card-canvas-frame">
            <canvas ref={canvasRef} class="card-canvas" aria-label="文卡图预览" />
          </div>
          <div class="card-theme-row">
            {CARD_THEMES.map((item) => (
              <button
                key={item.id}
                class={`theme-swatch ${theme.id === item.id ? 'is-active' : ''}`}
                title={item.name}
                aria-label={`主题：${item.name}`}
                onClick={() => setThemeId(item.id)}
                style={{
                  background: `linear-gradient(135deg, ${item.colors[0]}, ${item.colors[1]})`,
                }}
              >
                <span style={{ color: item.accent }}>媒</span>
              </button>
            ))}
          </div>
        </div>
        <div class="card-controls">
          <Field label="主张" hint="卡片上的核心句，建议 8–18 字">
            <textarea
              class="textarea"
              rows={3}
              maxLength={80}
              value={text}
              onInput={(e) => setText((e.target as HTMLTextAreaElement).value)}
            />
          </Field>
          <Field label="补充" hint="一句解释或出处，没有出处不要编">
            <textarea
              class="textarea"
              rows={2}
              maxLength={100}
              value={subtext}
              onInput={(e) => setSubtext((e.target as HTMLTextAreaElement).value)}
            />
          </Field>
          <Field label="素材标题">
            <input
              class="input"
              value={title}
              onInput={(e) => setTitle((e.target as HTMLInputElement).value)}
            />
          </Field>
          <div class="field">
            <span class="field-label">画幅</span>
            <Segmented
              options={(Object.keys(CARD_RATIOS) as CardRatio[]).map((key) => ({
                value: key,
                label: CARD_RATIOS[key].label,
              }))}
              value={ratio}
              onChange={setRatio}
            />
          </div>
          <div class="card-tips">
            <Icon name="bulb" size={16} />
            <p>建议提前在脑中想好发布场景：留白位置与字数会决定读者是否愿意保存。</p>
          </div>
          {error ? <p class="form-error">{error}</p> : null}
        </div>
      </div>
    </Modal>
  );
}
