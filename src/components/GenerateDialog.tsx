import { useEffect, useLayoutEffect, useMemo, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import type { GenerateResult, Kind, Project, PromptResult } from '../lib/types';
import { KINDS, KIND_MAP } from '../lib/types';
import { Button, Field, Modal } from './ui';
import { Icon } from './Icon';
import { MarkdownView } from './MarkdownView';
import { navigate } from '../lib/router';
import { copyText, extractTitle, kindLabel, relativeTime } from '../lib/format';

const INSTRUCTION_PLACEHOLDER: Record<Kind, string> = {
  article: '例如：面向刚开始做自媒体的上班族，写一篇关于「如何持续找到选题」的长文，语气克制，有具体方法。',
  script: '例如：做一条 60 秒口播，讲清楚为什么「先完成再完美」，开头用一个真实困境切入。',
  illustration: '例如：为这篇文章做一组编辑插画，暖纸色、克制，给标题留出上方空白。',
  card: '例如：围绕「创作是长期主义」，写 5 组适合 3:4 文卡的短文案，克制、不鸡汤。',
  video: '例如：做一条 4 分钟中视频，讲「个人创作者如何建立素材系统」，需要分镜表和素材清单。',
};

export function GenerateDialog({
  project,
  open,
  initialKind,
  onClose,
  onSaved,
}: {
  project: Project;
  open: boolean;
  initialKind?: Kind;
  onClose: () => void;
  onSaved: (assetId: string) => void | Promise<void>;
}) {
  const app = useApp();
  const [kind, setKind] = useState<Kind>('article');
  const [presetId, setPresetId] = useState('');
  const [skillIds, setSkillIds] = useState<string[]>([]);
  const [instruction, setInstruction] = useState('');
  const [advanced, setAdvanced] = useState(false);
  const [loading, setLoading] = useState(false);
  const [saving, setSaving] = useState(false);
  const [result, setResult] = useState<GenerateResult | null>(null);
  const [title, setTitle] = useState('');
  const [error, setError] = useState('');
  const [errorCode, setErrorCode] = useState('');
  const [preview, setPreview] = useState<PromptResult | null>(null);
  const [showPrompt, setShowPrompt] = useState(false);
  const [compiling, setCompiling] = useState(false);

  const isMedia = kind === 'illustration' || kind === 'video';
  const kindPresets = useMemo(
    () => app.data.presets.filter((preset) => preset.kind === kind),
    [app.data.presets, kind],
  );
  const availableSkills = useMemo(
    () =>
      app.data.skills.filter(
        (skill) => skill.appliesTo.length === 0 || skill.appliesTo.includes(kind),
      ),
    [app.data.skills, kind],
  );

  const selectKind = (next: Kind, presetList = app.data.presets) => {
    setKind(next);
    const preset = presetList.find((item) => item.kind === next);
    setPresetId(preset?.id ?? '');
    setSkillIds(preset?.skillIds ?? []);
    setResult(null);
    setPreview(null);
    setError('');
    setErrorCode('');
  };

  useLayoutEffect(() => {
    if (!open) return;
    const startKind = initialKind ?? 'article';
    setKind(startKind);
    const preset = app.data.presets.find((item) => item.kind === startKind);
    setPresetId(preset?.id ?? '');
    setSkillIds(preset?.skillIds ?? []);
    setInstruction('');
    setResult(null);
    setPreview(null);
    setError('');
    setErrorCode('');
    setShowPrompt(false);
    setAdvanced(false);
  }, [open, initialKind, app.data.presets]);

  const selectPreset = (id: string) => {
    setPresetId(id);
    const preset = app.data.presets.find((item) => item.id === id);
    if (preset) setSkillIds(preset.skillIds);
    setResult(null);
    setPreview(null);
  };

  const toggleSkill = (id: string) => {
    setSkillIds((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]));
    setPreview(null);
  };

  const payload = () => ({
    projectId: project.id,
    kind,
    presetId: isMedia ? null : presetId || null,
    skillIds: isMedia ? [] : skillIds,
    instruction,
  });

  const generate = async () => {
    setLoading(true);
    setError('');
    setErrorCode('');
    setResult(null);
    try {
      const data = await api.generate(payload());
      setResult(data);
      setTitle(
        data.asset?.title ??
          extractTitle(
            data.content,
            `${kindLabel(kind)} · ${new Date().toLocaleDateString('zh-CN')}`,
          ),
      );
      setPreview(null);
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setError(apiErr?.message ?? '生成失败，请重试');
      setErrorCode(apiErr?.code ?? '');
    } finally {
      setLoading(false);
    }
  };

  const compilePrompt = async () => {
    setCompiling(true);
    setError('');
    try {
      const data = await api.previewPrompt(payload());
      setPreview(data);
      setShowPrompt(true);
      return data;
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '编译提示词失败');
      return null;
    } finally {
      setCompiling(false);
    }
  };

  const copyPrompt = async () => {
    const data = preview ?? (await compilePrompt());
    if (!data) return;
    await copyText(data.prompt);
    app.toast({ message: '完整提示词已复制', tone: 'success' });
  };

  const copyResult = async () => {
    if (!result) return;
    await copyText(result.content);
    app.toast({ message: '生成内容已复制', tone: 'success' });
  };

  const save = async () => {
    if (!result) return;
    if (!title.trim()) {
      setError('素材需要一个标题');
      return;
    }
    setSaving(true);
    setError('');
    try {
      const asset = await api.createAsset(project.id, {
        kind,
        title: title.trim(),
        content: result.content,
        status: 'draft',
        source: 'llm',
      });
      await Promise.all([
        app.refreshProjects(),
        app.refreshWorkspace(),
      ]);
      app.toast({
        message: result.snapshotPath
          ? '已保存素材，提示词快照同时归档'
          : '已保存为项目素材',
        tone: 'success',
      });
      await onSaved(asset.id);
      onClose();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const linkedIdeas = app.data.ideas.filter((idea) => project.ideaIds.includes(idea.id));
  const textKernel = app.data.config.textKernel ?? 'api';
  const mediaProfile =
    kind === 'illustration'
      ? app.data.config.mediaGen?.illustration
      : app.data.config.mediaGen?.video;
  const mediaConfigured = Boolean(
    mediaProfile?.baseUrl.trim() &&
      mediaProfile?.model.trim() &&
      (mediaProfile?.hasApiKey || /^http:\/\/(localhost|127\.0\.0\.1)/.test(mediaProfile.baseUrl)),
  );
  const configured =
    isMedia ? mediaConfigured : textKernel === 'api' ? app.data.config.llm.hasApiKey : true;
  const kernelLabel = isMedia
    ? '媒体生成模型'
    : textKernel === 'api'
      ? 'API 大模型'
      : textKernel === 'claude'
        ? 'Claude Code CLI'
        : textKernel === 'opencode'
          ? 'OpenCode CLI'
          : 'DeepSeek CLI';

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="生成素材"
      subtitle={
        isMedia
          ? `${project.title} · 调用媒体生成模型，成功后直接写入素材库`
          : `${project.title} · 手动调用文本内核，结果确认后才写入素材库`
      }
      size="full"
      class="generate-modal"
      footer={
        result ? (
          <>
            <Button variant="ghost" icon="refresh" onClick={generate} loading={loading}>
              重新生成
            </Button>
            <div class="modal-actions">
              {result.asset ? (
                <>
                  <Button variant="ghost" onClick={onClose}>
                    关闭
                  </Button>
                  <Button
                    variant="primary"
                    icon="arrow-right"
                    onClick={() => {
                      onSaved(result.asset!.id);
                      onClose();
                    }}
                  >
                    查看并编辑
                  </Button>
                </>
              ) : (
                <>
                  <Button variant="primary" icon="check" loading={saving} onClick={save}>
                    保存为素材
                  </Button>
                </>
              )}
            </div>
          </>
        ) : (
          <>
            <span class="shortcut-hint">
              {linkedIdeas.length
                ? `将带入 ${linkedIdeas.length} 条关联灵感`
                : '尚未关联灵感，将主要基于项目简介生成'}
            </span>
            <div class="modal-actions">
              <Button variant="ghost" icon="copy" onClick={copyPrompt} loading={compiling}>
                复制提示词
              </Button>
              <Button variant="primary" icon={isMedia ? 'image' : 'spark'} onClick={generate} loading={loading}>
                {isMedia ? '生成并保存' : '生成'}
              </Button>
            </div>
          </>
        )
      }
    >
      <div class="generate-layout">
        <div class="generate-form">
          <div class="generate-kind-tabs">
            {KINDS.filter((item) => item.id !== 'card').map((item) => (
              <button
                key={item.id}
                class={`generate-kind ${kind === item.id ? 'is-active' : ''}`}
                onClick={() => selectKind(item.id)}
                title={item.description}
              >
                <span class="kind-dot" style={{ background: item.accent }} />
                {item.label}
              </button>
            ))}
          </div>

          {!isMedia ? (
            <Field label="创作预设" hint="决定结构、语气与输出规范">
              <select
                class="select"
                value={presetId}
                onChange={(e) => selectPreset((e.target as HTMLSelectElement).value)}
              >
                <option value="">不套用预设（使用通用规范）</option>
                {kindPresets.map((preset) => (
                  <option value={preset.id} key={preset.id}>
                    {preset.name}
                  </option>
                ))}
              </select>
            </Field>
          ) : null}

          <Field label="本次要求" hint="可留空，默认围绕项目主题创作">
            <textarea
              class="textarea generate-instruction"
              rows={5}
              value={instruction}
              placeholder={INSTRUCTION_PLACEHOLDER[kind]}
              onInput={(e) => setInstruction((e.target as HTMLTextAreaElement).value)}
            />
          </Field>

          <div class="generate-context">
            <span class="generate-context-head">
              <Icon name="layers" size={15} />
              上下文
            </span>
            <p>
              项目简介、{linkedIdeas.length} 条关联灵感、已有素材标题都会发送给模型。
            </p>
            <span class={`generate-kernel-pill ${isMedia ? 'is-media' : ''}`}>
              <Icon name={isMedia ? 'image' : textKernel === 'api' ? 'spark' : 'code'} size={13} />
              {kernelLabel}
            </span>
          </div>

          <button class={`advanced-toggle ${advanced ? 'is-open' : ''}`} onClick={() => setAdvanced(!advanced)}>
            <span>
              <Icon name="sliders" size={15} />
              {isMedia ? '视觉提示词' : '技能与提示词'}
              {!isMedia ? <span class="advanced-count">{skillIds.length}</span> : null}
            </span>
            <Icon name="chevron-down" size={15} />
          </button>

          {advanced ? (
            <div class="generate-advanced">
              {!isMedia ? (
                <div class="advanced-block">
                  <div class="advanced-head">
                    <strong>已加载技能</strong>
                    <span>{availableSkills.length} 个可用</span>
                  </div>
                  {availableSkills.length ? (
                    <div class="skill-checks">
                      {availableSkills.map((skill) => (
                        <label class={`skill-check ${skillIds.includes(skill.id) ? 'is-checked' : ''}`} key={skill.id}>
                          <input
                            type="checkbox"
                            checked={skillIds.includes(skill.id)}
                            onChange={() => toggleSkill(skill.id)}
                          />
                          <span>
                            <strong>{skill.name}</strong>
                            <small>{skill.description}</small>
                          </span>
                        </label>
                      ))}
                    </div>
                  ) : (
                    <p class="muted-text">还没有适用于{kindLabel(kind)}的技能。</p>
                  )}
                </div>
              ) : null}
              <div class="advanced-block">
                <div class="advanced-head">
                  <strong>编译后的提示词</strong>
                  <Button size="sm" variant="ghost" icon="eye" onClick={compilePrompt} loading={compiling}>
                    {showPrompt ? '刷新' : '预览'}
                  </Button>
                </div>
                {showPrompt && preview ? (
                  <div class="prompt-preview">
                    <div class="prompt-meta">
                      <span>{preview.preset?.name ?? '通用规范'}</span>
                      <span>{preview.skills.length} 个技能</span>
                    </div>
                    <pre>{preview.prompt}</pre>
                    <Button size="sm" variant="secondary" icon="copy" onClick={copyPrompt}>
                      复制完整提示词
                    </Button>
                  </div>
                ) : (
                  <p class="muted-text">
                    {isMedia
                      ? '查看实际发给媒体模型的视觉提示词；不调用接口。'
                      : '编译后可复制到任意模型使用，不消耗 API。'}
                  </p>
                )}
              </div>
            </div>
          ) : null}

          {!configured ? (
            <div class="notice notice-warn">
              <Icon name="alert" size={16} />
              <p>
                {isMedia
                  ? `尚未配置${kindLabel(kind)}生成模型。请先到设置中接入图像/视频接口。`
                  : '尚未配置 API Key。你仍可以复制提示词，或前往设置接入 OpenAI 兼容服务。'}
              </p>
              <button class="text-link" onClick={() => { onClose(); navigate('/settings'); }}>
                去设置
              </button>
            </div>
          ) : null}

          {error ? (
            <div class="notice notice-error" role="alert">
              <Icon name="alert" size={16} />
              <p>{error}</p>
              {['LLM_NOT_CONFIGURED', 'LLM_REQUEST_FAILED', 'CLI_NOT_FOUND', 'CLI_FAILED', 'CLI_TIMEOUT', 'MEDIA_NOT_CONFIGURED', 'MEDIA_GEN_FAILED'].includes(errorCode) ? (
                <button class="text-link" onClick={() => { onClose(); navigate('/settings'); }}>
                  检查设置
                </button>
              ) : null}
            </div>
          ) : null}
        </div>

        <div class="generate-result">
          {loading ? (
            <div class="generate-loading">
              <span class="orb-loading"><Icon name={isMedia ? 'image' : 'spark'} size={26} /></span>
              <h3>{isMedia ? '正在生成并保存媒体…' : '正在组织素材…'}</h3>
              <p>
                {isMedia
                  ? '媒体模型正在生成文件；视频任务可能需要轮询，完成后会自动写入项目。'
                  : `正在调用${kernelLabel}读取项目上下文、预设与技能。通常需要 10–60 秒。`}
              </p>
              <span class="loading-bar"><span /></span>
            </div>
          ) : result ? (
            <div class={`result-sheet ${result.asset ? 'is-media-result' : ''}`}>
              <div class="result-head">
                <div>
                  <span class="eyebrow">
                    {result.asset ? 'MEDIA · 已保存' : 'DRAFT'} · {kindLabel(kind)}
                  </span>
                  {result.asset ? (
                    <h3 class="result-media-title">{result.asset.title}</h3>
                  ) : (
                    <input
                      class="result-title-input"
                      value={title}
                      aria-label="素材标题"
                      onInput={(e) => setTitle((e.target as HTMLInputElement).value)}
                    />
                  )}
                </div>
                <div class="result-meta">
                  <span>{result.model}</span>
                  <span>{(result.latencyMs / 1000).toFixed(1)}s</span>
                  {result.usage.totalTokens ? <span>{result.usage.totalTokens} tokens</span> : null}
                </div>
              </div>
              {result.asset ? (
                <div class="result-media">
                  {result.asset.kind === 'video' ? (
                    <video src={result.asset.fileUrl} controls playsinline />
                  ) : (
                    <img src={result.asset.fileUrl} alt={result.asset.title} />
                  )}
                  <div class="result-media-meta">
                    <span><Icon name="check" size={14} /> 已写入项目 {kindLabel(kind)} 目录</span>
                    {result.snapshotPath ? <span><Icon name="file-text" size={14} /> 提示词已快照</span> : null}
                  </div>
                </div>
              ) : (
                <div class="result-body">
                  <MarkdownView source={result.content} />
                </div>
              )}
              <div class="result-foot">
                <div class="result-actions">
                  {result.asset ? null : (
                    <Button size="sm" variant="ghost" icon="copy" onClick={copyResult}>
                      复制正文
                    </Button>
                  )}
                  <Button
                    size="sm"
                    variant="ghost"
                    icon="file-text"
                    onClick={() => setShowPrompt(!showPrompt)}
                  >
                    {showPrompt ? '收起提示词' : '查看提示词'}
                  </Button>
                </div>
                {result.snapshotPath ? (
                  <span class="result-snapshot" title={result.snapshotPath}>
                    提示词已快照
                  </span>
                ) : null}
              </div>
              {showPrompt ? <pre class="result-prompt">{result.prompt}</pre> : null}
            </div>
          ) : (
            <div class="generate-placeholder">
              <span class="placeholder-mark">「</span>
              <h3>先想清楚要什么，<br />再让模型动手。</h3>
              <p>生成结果会出现在这里，确认标题与内容后再保存。每次提示词都会自动快照到项目的 prompts 目录。</p>
              {linkedIdeas.length ? (
                <div class="linked-idea-preview">
                  <span class="eyebrow">本次关联灵感</span>
                  {linkedIdeas.slice(0, 4).map((idea) => (
                    <div key={idea.id}>
                      <strong>{idea.title}</strong>
                      <small>{relativeTime(idea.updatedAt)}</small>
                    </div>
                  ))}
                </div>
              ) : null}
            </div>
          )}
        </div>
      </div>
    </Modal>
  );
}
