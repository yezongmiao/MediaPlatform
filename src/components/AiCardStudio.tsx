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
  const [presetId, setPresetId] = useState('');
  const [skillIds, setSkillIds] = useState<string[]>([]);
  const [cards, setCards] = useState<GeneratedCard[]>([]);
  const [activeIndex, setActiveIndex] = useState(0);
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

  const theme = CARD_THEMES[0];
  const kindPresets = app.data.presets.filter((preset) => preset.kind === 'card');
  const availableSkills = app.data.skills.filter(
    (skill) => skill.appliesTo.length === 0 || skill.appliesTo.includes('card'),
  );
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
    const preset = app.data.presets.find((item) => item.kind === 'card');
    setPresetId(preset?.id ?? '');
    setSkillIds(preset?.skillIds ?? []);
    setError('');
  }, [open, project.assets.article, project.assets.script, app.data.presets]);

  useLayoutEffect(() => {
    if (!open || !activePlan || !canvasRef.current) return;
    drawEditorialCard(canvasRef.current, { plan: activePlan, theme });
  }, [open, activePlan, activeIndex, theme]);

  const generate = async () => {
    setLoading(true);
    setError('');
    setCards([]);
    setActiveIndex(0);
    setMeta(null);
    setSaveProgress('');
    try {
      const result = await api.generateCardPlan(project.id, {
        sourceAssetId: sourceId || undefined,
        instruction,
        presetId: presetId || undefined,
        skillIds,
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
    const canvas = document.createElement('canvas');
    drawEditorialCard(canvas, { plan: card.plan, theme });
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
    const groupId = `card-group-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
    const groupTitle = cards.length > 1
      ? `${cards[0].plan.title} 等 ${cards.length} 张`
      : cards[0]?.plan.title || 'AI 文卡图';
    try {
      for (let index = 0; index < cards.length; index += 1) {
        setSaveProgress(`${index + 1} / ${cards.length}`);
        const card = cards[index];
        const blob = await renderCardBlob(card);
        const safeTitle = card.plan.title.replace(/[\\/:*?"<>|]/g, '-').slice(0, 40) || `文卡图-${index + 1}`;
        const file = new File([blob], `${safeTitle}.png`, { type: 'image/png' });
        const asset = await api.uploadAsset(project.id, 'card', file, {
          title: card.plan.title,
          groupId,
          groupTitle,
          groupIndex: index + 1,
          groupTotal: cards.length,
        });
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
      subtitle="由当前文本/多模态内核生成文案，本地排版为多张 1080×1440 PNG。"
      size="full"
      class="ai-card-modal"
      footer={
        <>
          <span class="shortcut-hint">
            {cards.length ? `${cards.length} 张 · 已按文章长度 ${meta?.sourceChars ?? 0} 字生成` : '文本企划 + 本地排版 · 1080×1440 PNG'}
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
                <p>文章越长，生成的卡片越多；模型会拆出不同观点，本地排版标题、正文、注脚与来源。</p>
              </div>
            ) : null}
            {loading ? (
              <div class="ai-card-loading">
                <span class="spinner" />
                <strong>正在拆解文章并排版…</strong>
                <small>只调用当前文本/多模态内核生成文案，不调用图像模型。</small>
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
            <span>纯文字排版 · 本地渲染</span>
          </div>
        </div>

        <aside class="ai-card-controls">
          <Field label="取材文章" hint={sources.length ? '文章越长，生成的卡片越多（每约 500 字 1 张，不设默认上限）' : '项目还没有文章，将使用项目简介'}>
            <select class="select" value={sourceId} onChange={(e) => setSourceId((e.target as HTMLSelectElement).value)}>
              <option value="">不指定（使用项目简介）</option>
              {sources.map((source) => (
                <option value={source.id} key={source.id}>{source.title}</option>
              ))}
            </select>
          </Field>
          <Field label="创作预设" hint="决定结构、语气与输出规范">
            <select
              class="select"
              value={presetId}
              onChange={(e) => {
                const id = (e.target as HTMLSelectElement).value;
                setPresetId(id);
                const preset = app.data.presets.find((item) => item.id === id);
                setSkillIds(preset?.skillIds ?? []);
              }}
            >
              <option value="">不套用预设（使用通用规范）</option>
              {kindPresets.map((preset) => (
                <option value={preset.id} key={preset.id}>{preset.name}</option>
              ))}
            </select>
          </Field>
          <Field label="已加载技能" hint={`${availableSkills.length} 个适用于文卡图`}>
            {availableSkills.length ? (
              <div class="skill-checks ai-card-skill-checks">
                {availableSkills.map((skill) => (
                  <label class={`skill-check ${skillIds.includes(skill.id) ? 'is-checked' : ''}`} key={skill.id}>
                    <input
                      type="checkbox"
                      checked={skillIds.includes(skill.id)}
                      onChange={() => setSkillIds((prev) => (
                        prev.includes(skill.id)
                          ? prev.filter((id) => id !== skill.id)
                          : [...prev, skill.id]
                      ))}
                    />
                    <span>
                      <strong>{skill.name}</strong>
                      <small>{skill.description}</small>
                    </span>
                  </label>
                ))}
              </div>
            ) : (
              <p class="muted-text">还没有适用于文卡图的技能。</p>
            )}
          </Field>
          <Field label="额外要求" hint="可选：角度、时间线、画面风格">
            <textarea
              class="textarea"
              rows={4}
              value={instruction}
              placeholder="例如：突出秦汉之际的时间线；语气克制，标题有杂志感。"
              onInput={(e) => setInstruction((e.target as HTMLTextAreaElement).value)}
            />
          </Field>
          <Button variant="primary" icon="spark" loading={loading} onClick={generate}>
            {cards.length ? '重新生成' : '生成文卡图'}
          </Button>
          {meta ? (
            <p class="ai-card-count">
              文章约 {meta.sourceChars} 字 → 建议 {meta.targetCount} 张，实际生成 {cards.length} 张。
            </p>
          ) : null}
          <p class="settings-note">
            正文与来源由文本模型按原文提炼，请在导出前核对；卡片只在本地排版保存，不调用图像模型。
          </p>

          {meta?.warning ? (
            <div class="ai-card-warning">
              <Icon name="alert" size={14} />
              <span>{meta.warning}</span>
            </div>
          ) : null}
          {error ? <InlineError message={error} /> : null}
        </aside>
      </div>
    </Modal>
  );
}
