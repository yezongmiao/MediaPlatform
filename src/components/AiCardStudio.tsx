import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import type { CardPlan, Project } from '../lib/types';
import { Button, Field, InlineError, Modal } from './ui';
import { Icon } from './Icon';
import { CARD_THEMES } from '../lib/card-renderer';
import { drawEditorialCard } from '../lib/editorial-card';

function canvasToBlob(canvas: HTMLCanvasElement): Promise<Blob> {
  return new Promise((resolve, reject) => {
    canvas.toBlob((blob) => {
      if (blob) resolve(blob);
      else reject(new Error('导出 PNG 失败'));
    }, 'image/png');
  });
}

function loadImage(url: string): Promise<HTMLImageElement> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = () => resolve(image);
    image.onerror = () => reject(new Error('配图加载失败'));
    image.src = url;
  });
}

export function AiCardStudio({
  project,
  open,
  onClose,
  onSaved,
}: {
  project: Project;
  open: boolean;
  onClose: () => void;
  onSaved: (assetId: string) => void;
}) {
  const app = useApp();
  const canvasRef = useRef<HTMLCanvasElement>(null);
  const [sources, setSources] = useState<Array<{ id: string; title: string }>>([]);
  const [sourceId, setSourceId] = useState('');
  const [instruction, setInstruction] = useState('');
  const [withImage, setWithImage] = useState(true);
  const [themeId, setThemeId] = useState(CARD_THEMES[0].id);
  const [plan, setPlan] = useState<CardPlan | null>(null);
  const [imageUrl, setImageUrl] = useState('');
  const [image, setImage] = useState<HTMLImageElement | null>(null);
  const [imageWarning, setImageWarning] = useState('');
  const [imageLoadError, setImageLoadError] = useState('');
  const [meta, setMeta] = useState<{ model: string; latencyMs: number } | null>(null);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  const theme = CARD_THEMES.find((item) => item.id === themeId) ?? CARD_THEMES[0];

  useEffect(() => {
    if (!open) return;
    const list = [...(project.assets.article ?? []), ...(project.assets.script ?? [])].map((asset) => ({
      id: asset.id,
      title: asset.title,
    }));
    setSources(list);
    setSourceId((current) => (current && list.some((item) => item.id === current) ? current : list[0]?.id ?? ''));
    setError('');
  }, [open, project.assets.article, project.assets.script]);

  useEffect(() => {
    if (!imageUrl) {
      setImage(null);
      setImageLoadError('');
      return;
    }
    let cancelled = false;
    loadImage(imageUrl)
      .then((loaded) => {
        if (!cancelled) {
          setImage(loaded);
          setImageLoadError('');
        }
      })
      .catch(() => {
        if (!cancelled) {
          setImage(null);
          setImageLoadError('配图加载失败，已退回纯文字版。');
        }
      });
    return () => {
      cancelled = true;
    };
  }, [imageUrl]);

  useLayoutEffect(() => {
    if (!open || !plan || !canvasRef.current) return;
    drawEditorialCard(canvasRef.current, { plan, theme, image });
  }, [open, plan, theme, image]);

  const generate = async () => {
    setLoading(true);
    setError('');
    setPlan(null);
    setImageUrl('');
    setImage(null);
    setImageWarning('');
    setImageLoadError('');
    setMeta(null);
    try {
      const result = await api.generateCardPlan(project.id, {
        sourceAssetId: sourceId || undefined,
        instruction,
        withImage,
      });
      setPlan(result.plan);
      setImageUrl(result.imageUrl ?? '');
      setImageWarning(result.imageWarning ?? '');
      setMeta({ model: result.model, latencyMs: result.latencyMs });
      app.toast({ message: '文卡图方案已生成', tone: 'success' });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '生成文卡图失败');
    } finally {
      setLoading(false);
    }
  };

  const download = async () => {
    if (!plan || !canvasRef.current) return;
    const blob = await canvasToBlob(canvasRef.current);
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `${plan.title.replace(/[\\/:*?"<>|]/g, '-').slice(0, 40) || '文卡图'}.png`;
    link.click();
    URL.revokeObjectURL(url);
  };

  const save = async () => {
    if (!plan || !canvasRef.current) return;
    setSaving(true);
    setError('');
    try {
      const blob = await canvasToBlob(canvasRef.current);
      const safeTitle = plan.title.replace(/[\\/:*?"<>|]/g, '-').slice(0, 40) || '文卡图';
      const file = new File([blob], `${safeTitle}.png`, { type: 'image/png' });
      const asset = await api.uploadAsset(project.id, 'card', file, { title: plan.title });
      await Promise.all([app.refreshProjects(), app.refreshWorkspace()]);
      app.toast({ message: '文卡图 PNG 已存入项目', tone: 'success' });
      onSaved(asset.id);
      onClose();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存文卡图失败');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="AI 文卡图"
      subtitle="从文章生成一张 1080×1440 的编辑式 PNG；可选 AI 配图。"
      size="full"
      class="ai-card-modal"
      footer={
        <>
          <span class="shortcut-hint">文本企划 + 可选媒体配图 · 输出 1080×1440 PNG</span>
          <div class="modal-actions">
            <Button variant="secondary" icon="download" disabled={!plan || saving} onClick={download}>
              下载 PNG
            </Button>
            <Button variant="primary" icon="image" loading={saving} disabled={!plan} onClick={save}>
              存入项目
            </Button>
          </div>
        </>
      }
    >
      <div class="ai-card-layout">
        <div class="ai-card-stage">
          <div class={`ai-card-frame ${plan ? '' : 'is-empty'}`}>
            <canvas ref={canvasRef} class="ai-card-canvas" aria-label="AI 文卡图预览" />
            {!plan && !loading ? (
              <div class="ai-card-placeholder">
                <Icon name="cards" size={34} />
                <h3>把一篇文章变成一张可保存的卡片</h3>
                <p>选一篇文章，写一句额外要求；大模型会提炼标题、正文、注脚与配图提示词。</p>
              </div>
            ) : null}
            {loading ? (
              <div class="ai-card-loading">
                <span class="spinner" />
                <strong>正在提炼文章并排版…</strong>
                <small>{withImage ? '文本方案完成后会尝试生成配图，可能需要更久。' : '只生成文本方案，不调用图像模型。'}</small>
              </div>
            ) : null}
          </div>
          <div class="ai-card-stage-foot">
            <span>{plan ? `模型：${meta?.model ?? '—'} · ${meta?.latencyMs ?? 0}ms` : '1080 × 1440 · 3:4'}</span>
            {image ? <span><Icon name="image" size={12} /> 已带入 AI 配图</span> : <span>纯文字排版</span>}
          </div>
        </div>

        <aside class="ai-card-controls">
          <Field label="取材文章" hint={sources.length ? '从项目文章或脚本中提取' : '项目还没有文章，将使用项目简介'}>
            <select class="select" value={sourceId} onChange={(e) => setSourceId((e.target as HTMLSelectElement).value)}>
              <option value="">不指定（使用项目简介）</option>
              {sources.map((source) => (
                <option value={source.id} key={source.id}>{source.title}</option>
              ))}
            </select>
          </Field>
          <Field label="额外要求" hint="可选：角度、时间线、画面风格">
            <textarea
              class="textarea"
              rows={4}
              value={instruction}
              placeholder="例如：突出秦汉之际的时间线；配图用古地图质感，但不要出现文字。"
              onInput={(e) => setInstruction((e.target as HTMLTextAreaElement).value)}
            />
          </Field>
          <label class="ai-card-toggle">
            <input
              type="checkbox"
              checked={withImage}
              onChange={(e) => setWithImage((e.target as HTMLInputElement).checked)}
            />
            <span>
              <strong>生成 AI 配图</strong>
              <small>需要先在「设置 → 媒体生成」配置图像模型；失败时自动退回纯文字版。</small>
            </span>
          </label>

          <div class="ai-card-themes">
            <span class="field-label">纸张主题</span>
            <div class="theme-swatch-row">
              {CARD_THEMES.map((item) => (
                <button
                  key={item.id}
                  type="button"
                  class={`theme-swatch ${item.id === theme.id ? 'is-active' : ''}`}
                  title={item.name}
                  aria-label={`主题：${item.name}`}
                  onClick={() => setThemeId(item.id)}
                  style={{ background: `linear-gradient(135deg, ${item.colors[0]}, ${item.colors[1]})` }}
                >
                  <span style={{ color: item.accent }}>媒</span>
                </button>
              ))}
            </div>
          </div>

          <Button variant="primary" icon="spark" loading={loading} onClick={generate}>
            {plan ? '重新生成' : '生成文卡图'}
          </Button>
          <p class="settings-note">
            助手会严格按原文事实提炼；标题、正文与来源仍建议在导出前自行核对。配图会作为一张插图素材保存到项目。
          </p>

          {imageWarning ? (
            <div class="ai-card-warning">
              <Icon name="alert" size={14} />
              <span>{imageWarning}</span>
            </div>
          ) : null}
          {imageLoadError ? (
            <div class="ai-card-warning">
              <Icon name="alert" size={14} />
              <span>{imageLoadError}</span>
            </div>
          ) : null}
          {error ? <InlineError message={error} /> : null}
        </aside>
      </div>
    </Modal>
  );
}
