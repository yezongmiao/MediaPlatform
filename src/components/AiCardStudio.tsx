import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import type { GeneratedCard, Project } from '../lib/types';
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
  const imageCache = useRef(new Map<string, HTMLImageElement>());
  const [sources, setSources] = useState<Array<{ id: string; title: string }>>([]);
  const [sourceId, setSourceId] = useState('');
  const [instruction, setInstruction] = useState('');
  const [withImage, setWithImage] = useState(true);
  const [themeId, setThemeId] = useState(CARD_THEMES[0].id);
  const [cards, setCards] = useState<GeneratedCard[]>([]);
  const [activeIndex, setActiveIndex] = useState(0);
  const [activeImage, setActiveImage] = useState<HTMLImageElement | null>(null);
  const [imageLoadError, setImageLoadError] = useState('');
  const [meta, setMeta] = useState<{
    model: string;
    latencyMs: number;
    sourceChars: number;
    targetCount: number;
    warning?: string;
  } | null>(null);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saveProgress, setSaveProgress] = useState('');
  const [error, setError] = useState('');

  const theme = CARD_THEMES.find((item) => item.id === themeId) ?? CARD_THEMES[0];
  const activeCard = cards[activeIndex] ?? null;
  const activePlan = activeCard?.plan ?? null;

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
    if (!activeCard?.imageUrl) {
      setActiveImage(null);
      setImageLoadError('');
      return;
    }
    let cancelled = false;
    const cached = imageCache.current.get(activeCard.imageUrl);
    if (cached) {
      setActiveImage(cached);
      setImageLoadError('');
      return;
    }
    loadImage(activeCard.imageUrl)
      .then((loaded) => {
        imageCache.current.set(activeCard.imageUrl!, loaded);
        if (!cancelled) {
          setActiveImage(loaded);
          setImageLoadError('');
        }
      })
      .catch(() => {
        if (!cancelled) {
          setActiveImage(null);
          setImageLoadError('配图加载失败，已退回纯文字版。');
        }
      });
    return () => {
      cancelled = true;
    };
  }, [activeCard?.imageUrl, activeCard]);

  useLayoutEffect(() => {
    if (!open || !activePlan || !canvasRef.current) return;
    drawEditorialCard(canvasRef.current, { plan: activePlan, theme, image: activeImage });
  }, [open, activePlan, activeIndex, theme, activeImage]);

  const generate = async () => {
    setLoading(true);
    setError('');
    setCards([]);
    setActiveIndex(0);
    setActiveImage(null);
    setImageLoadError('');
    setMeta(null);
    setSaveProgress('');
    try {
      const result = await api.generateCardPlan(project.id, {
        sourceAssetId: sourceId || undefined,
        instruction,
        withImage,
      });
      setCards(result.cards);
      setActiveIndex(0);
      setMeta({
        model: result.model,
        latencyMs: result.latencyMs,
        sourceChars: result.sourceChars,
        targetCount: result.targetCount,
        warning: result.warning,
      });
      app.toast({
        message: `已生成 ${result.cards.length} 张文卡图方案（文章约 ${result.sourceChars} 字）`,
        tone: 'success',
      });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '生成文卡图失败');
    } finally {
      setLoading(false);
    }
  };

  const renderCardBlob = async (card: GeneratedCard): Promise<Blob> => {
    let image: HTMLImageElement | null = null;
    if (card.imageUrl) {
      image = imageCache.current.get(card.imageUrl) ?? null;
      if (!image) {
        try {
          image = await loadImage(card.imageUrl);
          imageCache.current.set(card.imageUrl, image);
        } catch {
          image = null;
        }
      }
    }
    const canvas = document.createElement('canvas');
    drawEditorialCard(canvas, { plan: card.plan, theme, image });
    return canvasToBlob(canvas);
  };

  const download = async () => {
    if (!activePlan || !canvasRef.current) return;
    const blob = await canvasToBlob(canvasRef.current);
    const url = URL.createObjectURL(blob);
    const link = document.createElement('a');
    link.href = url;
    link.download = `${activePlan.title.replace(/[\\/:*?"<>|]/g, '-').slice(0, 40) || '文卡图'}.png`;
    link.click();
    URL.revokeObjectURL(url);
  };

  const save = async () => {
    if (!cards.length) return;
    setSaving(true);
    setError('');
    const savedIds: string[] = [];
    try {
      for (let index = 0; index < cards.length; index += 1) {
        setSaveProgress(`${index + 1} / ${cards.length}`);
        const card = cards[index];
        const blob = await renderCardBlob(card);
        const safeTitle = card.plan.title.replace(/[\\/:*?"<>|]/g, '-').slice(0, 40) || `文卡图-${index + 1}`;
        const file = new File([blob], `${safeTitle}.png`, { type: 'image/png' });
        const asset = await api.uploadAsset(project.id, 'card', file, { title: card.plan.title });
        savedIds.push(asset.id);
      }
      await Promise.all([app.refreshProjects(), app.refreshWorkspace()]);
      app.toast({ message: `已存入 ${cards.length} 张文卡图 PNG`, tone: 'success' });
      if (savedIds[0]) onSaved(savedIds[0]);
      onClose();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存文卡图失败');
    } finally {
      setSaving(false);
      setSaveProgress('');
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="AI 文卡图"
      subtitle="根据文章长度生成多张 1080×1440 编辑式 PNG；可选 AI 配图。"
      size="full"
      class="ai-card-modal"
      footer={
        <>
          <span class="shortcut-hint">
            {cards.length ? `${cards.length} 张 · 已按文章长度 ${meta?.sourceChars ?? 0} 字生成` : '文本企划 + 可选媒体配图 · 1080×1440 PNG'}
          </span>
          <div class="modal-actions">
            <Button variant="secondary" icon="download" disabled={!activePlan || saving} onClick={download}>
              下载当前
            </Button>
            <Button variant="primary" icon="image" loading={saving} disabled={!cards.length} onClick={save}>
              {saving ? `保存中 ${saveProgress}` : cards.length > 1 ? `保存全部 ${cards.length} 张` : '存入项目'}
            </Button>
          </div>
        </>
      }
    >
      <div class="ai-card-layout">
        <div class="ai-card-stage">
          <div class={`ai-card-frame ${activePlan ? '' : 'is-empty'}`}>
            <canvas ref={canvasRef} class="ai-card-canvas" aria-label="AI 文卡图预览" />
            {!activePlan && !loading ? (
              <div class="ai-card-placeholder">
                <Icon name="cards" size={34} />
                <h3>把一篇文章变成一组可保存的卡片</h3>
                <p>文章越长，生成的卡片越多；大模型会拆出不同观点，分别排版标题、正文、注脚与配图。</p>
              </div>
            ) : null}
            {loading ? (
              <div class="ai-card-loading">
                <span class="spinner" />
                <strong>正在拆解文章并排版…</strong>
                <small>{withImage ? '会为每张卡片逐张生成配图，可能需要较久。' : '只生成文本方案，不调用图像模型。'}</small>
              </div>
            ) : null}
          </div>
          {cards.length > 1 ? (
            <div class="ai-card-thumbs" aria-label="文卡图列表">
              {cards.map((card, index) => (
                <button
                  key={`${card.plan.title}-${index}`}
                  class={`ai-card-thumb ${index === activeIndex ? 'is-active' : ''}`}
                  onClick={() => setActiveIndex(index)}
                  aria-label={`查看第 ${index + 1} 张：${card.plan.title}`}
                >
                  <span>{String(index + 1).padStart(2, '0')}</span>
                  <strong>{card.plan.title}</strong>
                </button>
              ))}
            </div>
          ) : null}
          <div class="ai-card-stage-foot">
            <span>
              {activePlan
                ? `第 ${activeIndex + 1} / ${cards.length} 张 · 模型 ${meta?.model ?? '—'} · ${meta?.latencyMs ?? 0}ms`
                : '1080 × 1440 · 3:4'}
            </span>
            <span>
              {activeImage ? (
                <><Icon name="image" size={12} /> 已带入 AI 配图</>
              ) : (
                '纯文字排版'
              )}
            </span>
          </div>
        </div>

        <aside class="ai-card-controls">
          <Field label="取材文章" hint={sources.length ? '文章越长，生成的卡片越多（每约 500 字 1 张，最多 6 张）' : '项目还没有文章，将使用项目简介'}>
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
              <small>会为每张卡片逐张调用图像模型；未配置或失败时自动退回纯文字版。</small>
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
            {cards.length ? '重新生成' : '生成文卡图'}
          </Button>
          {meta ? (
            <p class="ai-card-count">
              文章约 {meta.sourceChars} 字 → 建议 {meta.targetCount} 张，实际生成 {cards.length} 张。
            </p>
          ) : null}
          <p class="settings-note">
            正文与来源由文本模型按原文提炼，请在导出前核对。配图会作为独立插图素材保存到项目。
          </p>

          {meta?.warning ? (
            <div class="ai-card-warning">
              <Icon name="alert" size={14} />
              <span>{meta.warning}</span>
            </div>
          ) : null}
          {activeCard?.imageWarning ? (
            <div class="ai-card-warning">
              <Icon name="alert" size={14} />
              <span>{activeCard.imageWarning}</span>
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
