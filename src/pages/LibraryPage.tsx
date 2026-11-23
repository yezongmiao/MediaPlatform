import { useEffect, useLayoutEffect, useMemo, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import { Icon } from '../components/Icon';
import { MarkdownView } from '../components/MarkdownView';
import { LibraryAssistantPanel } from '../components/LibraryAssistantPanel';
import { Badge, Button, ConfirmDialog, EmptyState, Field, Modal } from '../components/ui';
import type { Kind, Preset, Skill } from '../lib/types';
import { KINDS, KIND_MAP } from '../lib/types';
import { excerpt, kindLabel } from '../lib/format';

type LibraryTab = 'skills' | 'presets';

export function LibraryPage() {
  const app = useApp();
  const [tab, setTab] = useState<LibraryTab>('skills');
  const [editingSkill, setEditingSkill] = useState<Skill | 'new' | null>(null);
  const [editingPreset, setEditingPreset] = useState<Preset | 'new' | null>(null);
  const [assistantOpen, setAssistantOpen] = useState(false);

  const kindCount = useMemo(() => {
    const map = new Map<Kind, number>();
    for (const preset of app.data.presets) {
      map.set(preset.kind, (map.get(preset.kind) ?? 0) + 1);
    }
    return map;
  }, [app.data.presets]);

  return (
    <div class="page library-page">
      <header class="page-heading">
        <div>
          <p class="eyebrow">LIBRARY · 技能与预设</p>
          <h1>把创作方法，沉淀成可复用的文件。</h1>
          <p class="page-subtitle">
            技能是方法论，预设是某一类素材的创作规范。它们会参与生成，也会被复制进 VSCode 工程。
          </p>
        </div>
        <div class="heading-actions">
          <Button
            variant="secondary"
            icon="spark"
            onClick={() => setAssistantOpen(true)}
          >
            助手
          </Button>
          <Button
            variant="primary"
            icon="plus"
            onClick={() => (tab === 'skills' ? setEditingSkill('new') : setEditingPreset('new'))}
          >
            新建{tab === 'skills' ? '技能' : '预设'}
          </Button>
        </div>
      </header>

      <div class="library-tabs">
        <button class={`library-tab ${tab === 'skills' ? 'is-active' : ''}`} onClick={() => setTab('skills')}>
          <Icon name="bulb" size={17} />
          创作技能
          <span>{app.data.skills.length}</span>
        </button>
        <button class={`library-tab ${tab === 'presets' ? 'is-active' : ''}`} onClick={() => setTab('presets')}>
          <Icon name="layers" size={17} />
          创作预设
          <span>{app.data.presets.length}</span>
        </button>
      </div>

      <div class="library-info-strip">
        <Icon name="code" size={17} />
        <p>
          一次生成会按「项目上下文 + 预设 + 技能 + 本次要求」编译提示词。项目打开 VSCode 时，选中的技能与预设会复制到项目目录，外部 AI 可直接读取。
        </p>
      </div>

      {tab === 'skills' ? (
        app.data.skills.length ? (
          <div class="skill-grid">
            {app.data.skills.map((skill) => (
              <article class="skill-card" key={skill.id}>
                <div class="skill-card-head">
                  <span class="skill-mark"><Icon name="bulb" size={17} /></span>
                  <span class="skill-applies">
                    {skill.appliesTo.length
                      ? skill.appliesTo.map((kind) => <Badge key={kind} tone="neutral">{KIND_MAP[kind]?.label ?? kind}</Badge>)
                      : <Badge tone="neutral">通用</Badge>}
                  </span>
                </div>
                <h3>{skill.name}</h3>
                <p class="skill-description">{skill.description || '还没有描述'}</p>
                <p class="skill-excerpt">{excerpt(skill.content, 130)}</p>
                <div class="skill-card-foot">
                  <span class="mono-mini">{skill.id}</span>
                  <Button size="sm" variant="ghost" icon="edit" onClick={() => setEditingSkill(skill)}>
                    编辑
                  </Button>
                </div>
              </article>
            ))}
          </div>
        ) : (
          <EmptyState
            icon="bulb"
            title="还没有技能"
            description="把常用写法总结成方法论，生成时按需勾选。"
            action={<Button variant="primary" icon="plus" onClick={() => setEditingSkill('new')}>新建技能</Button>}
          />
        )
      ) : app.data.presets.length ? (
        <div class="preset-sections">
          {KINDS.map((kind) => {
            const presets = app.data.presets.filter((preset) => preset.kind === kind.id);
            if (!presets.length) return null;
            return (
              <section class="preset-section" key={kind.id}>
                <div class="preset-section-head">
                  <span class="kind-icon" style={{ background: `${kind.accent}14`, color: kind.accent }}>
                    <Icon name="layers" size={17} />
                  </span>
                  <h2>{kind.label}预设</h2>
                  <span class="muted-text">{presets.length} 个 · {kindCount.get(kind.id) ?? 0}</span>
                </div>
                <div class="preset-grid">
                  {presets.map((preset) => (
                    <article class="preset-card" key={preset.id}>
                      <div class="preset-card-top">
                        <span class="preset-kind-label" style={{ color: kind.accent }}>{kind.label}</span>
                        <span class="muted-text">{preset.skillIds.length} 个技能</span>
                      </div>
                      <h3>{preset.name}</h3>
                      <p>{preset.description || '还没有描述'}</p>
                      <p class="preset-excerpt">{excerpt(preset.content, 150)}</p>
                      <div class="preset-card-foot">
                        <div class="preset-skill-dots">
                          {preset.skillIds.slice(0, 5).map((id) => {
                            const skill = app.data.skills.find((item) => item.id === id);
                            return <span key={id} title={skill?.name ?? id} />;
                          })}
                        </div>
                        <Button size="sm" variant="ghost" icon="edit" onClick={() => setEditingPreset(preset)}>
                          编辑
                        </Button>
                      </div>
                    </article>
                  ))}
                </div>
              </section>
            );
          })}
        </div>
      ) : (
        <EmptyState
          icon="layers"
          title="还没有预设"
          description="为文章、脚本、插图、文卡图、视频分别定义生成规范。"
          action={<Button variant="primary" icon="plus" onClick={() => setEditingPreset('new')}>新建预设</Button>}
        />
      )}

      <SkillEditor
        skill={editingSkill}
        open={editingSkill !== null}
        onClose={() => setEditingSkill(null)}
      />
      <PresetEditor
        preset={editingPreset}
        open={editingPreset !== null}
        onClose={() => setEditingPreset(null)}
      />

      {!assistantOpen ? (
        <button
          type="button"
          class="assistant-fab library-assistant-fab"
          aria-label="打开技能与预设助手"
          title="打开技能与预设助手"
          onClick={() => setAssistantOpen(true)}
        >
          <Icon name="spark" size={17} />
          <span>技能助手</span>
        </button>
      ) : null}

      {assistantOpen ? (
        <LibraryAssistantPanel
          open={assistantOpen}
          onClose={() => setAssistantOpen(false)}
          onChanged={app.refreshLibrary}
        />
      ) : null}
    </div>
  );
}

function SkillEditor({
  skill,
  open,
  onClose,
}: {
  skill: Skill | 'new' | null;
  open: boolean;
  onClose: () => void;
}) {
  const app = useApp();
  const isNew = skill === 'new';
  const existing = skill && skill !== 'new' ? skill : null;
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [appliesTo, setAppliesTo] = useState<string[]>([]);
  const [content, setContent] = useState('');
  const [preview, setPreview] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [error, setError] = useState('');

  useLayoutEffect(() => {
    if (!open) return;
    setName(existing?.name ?? '');
    setDescription(existing?.description ?? '');
    setAppliesTo(existing?.appliesTo ?? []);
    setContent(existing?.content ?? '');
    setPreview(false);
    setError('');
  }, [open, existing]);

  const toggleKind = (kind: Kind) => {
    setAppliesTo((prev) => (prev.includes(kind) ? prev.filter((x) => x !== kind) : [...prev, kind]));
  };

  const save = async () => {
    if (!name.trim()) {
      setError('技能需要名称');
      return;
    }
    setSaving(true);
    setError('');
    try {
      const payload = { name: name.trim(), description, appliesTo, content };
      if (existing) await api.updateSkill(existing.id, payload);
      else await api.createSkill(payload);
      await app.refreshLibrary();
      app.toast({ message: existing ? '技能已更新' : '技能已创建', tone: 'success' });
      onClose();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    if (!existing) return;
    setDeleting(true);
    try {
      await api.deleteSkill(existing.id);
      await app.refreshLibrary();
      app.toast({ message: '技能已移入回收站', tone: 'success' });
      setConfirmDelete(false);
      onClose();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '删除失败');
    } finally {
      setDeleting(false);
    }
  };

  return (
    <>
      <Modal
        open={open}
        onClose={onClose}
        title={existing ? '编辑技能' : '新建技能'}
        subtitle="技能会作为创作方法注入提示词，并在 VSCode 工程中留下副本。"
        size="xl"
        footer={
          <>
            {existing ? (
              <Button variant="ghost" icon="trash" class="footer-left" onClick={() => setConfirmDelete(true)}>
                删除
              </Button>
            ) : <span />}
            <div class="modal-actions">
              <Button variant="ghost" onClick={onClose}>取消</Button>
              <Button variant="primary" onClick={save} loading={saving}>保存技能</Button>
            </div>
          </>
        }
      >
        <div class="editor-grid">
          <div class="form-stack">
            <Field label="技能名称" required>
              <input class="input" data-autofocus value={name} onInput={(e) => setName((e.target as HTMLInputElement).value)} />
            </Field>
            <Field label="一句话说明">
              <input class="input" value={description} placeholder="它解决什么问题？" onInput={(e) => setDescription((e.target as HTMLInputElement).value)} />
            </Field>
            <Field label="适用素材类型" hint="不选表示通用">
              <div class="chip-checks">
                {KINDS.map((kind) => (
                  <button
                    type="button"
                    key={kind.id}
                    class={`chip-check ${appliesTo.includes(kind.id) ? 'is-checked' : ''}`}
                    onClick={() => toggleKind(kind.id)}
                  >
                    <span class="kind-dot" style={{ background: kind.accent }} />
                    {kind.label}
                  </button>
                ))}
              </div>
            </Field>
            <div class="side-tip">
              <Icon name="bulb" size={15} />
              <p>写清「什么时候用、具体怎么做、不要做什么」，比抽象原则更有用。</p>
            </div>
          </div>
          <div class="editor-main">
            <div class="editor-main-head">
              <span class="field-label">方法正文（Markdown）</span>
              <div class="asset-tab-switch">
                <button class={!preview ? 'is-active' : ''} onClick={() => setPreview(false)}>编辑</button>
                <button class={preview ? 'is-active' : ''} onClick={() => setPreview(true)}>预览</button>
              </div>
            </div>
            {preview ? (
              <div class="editor-preview-scroll">
                <MarkdownView source={content} />
              </div>
            ) : (
              <textarea
                class="textarea editor-textarea"
                value={content}
                placeholder="例如：\n# 开头钩子\n1. 反常识：…\n2. 损失感：…\n\n约束：\n- 不虚构数据\n- 首段不超过 80 字"
                onInput={(e) => setContent((e.target as HTMLTextAreaElement).value)}
              />
            )}
          </div>
          {error ? <p class="form-error editor-error">{error}</p> : null}
        </div>
      </Modal>
      <ConfirmDialog
        open={confirmDelete}
        title="删除这个技能？"
        message="技能文件会移入回收站。已被项目引用的副本仍会保留在项目目录里。"
        loading={deleting}
        onClose={() => setConfirmDelete(false)}
        onConfirm={remove}
      />
    </>
  );
}

function PresetEditor({
  preset,
  open,
  onClose,
}: {
  preset: Preset | 'new' | null;
  open: boolean;
  onClose: () => void;
}) {
  const app = useApp();
  const existing = preset && preset !== 'new' ? preset : null;
  const [name, setName] = useState('');
  const [description, setDescription] = useState('');
  const [kind, setKind] = useState<Kind>('article');
  const [skillIds, setSkillIds] = useState<string[]>([]);
  const [content, setContent] = useState('');
  const [preview, setPreview] = useState(false);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [error, setError] = useState('');

  useLayoutEffect(() => {
    if (!open) return;
    setName(existing?.name ?? '');
    setDescription(existing?.description ?? '');
    setKind(existing?.kind ?? 'article');
    setSkillIds(existing?.skillIds ?? []);
    setContent(existing?.content ?? '');
    setPreview(false);
    setError('');
  }, [open, existing]);

  const applicableSkills = app.data.skills.filter(
    (skill) => skill.appliesTo.length === 0 || skill.appliesTo.includes(kind),
  );

  const changeKind = (next: Kind) => {
    setKind(next);
    setSkillIds((prev) =>
      prev.filter((id) => {
        const skill = app.data.skills.find((item) => item.id === id);
        return !!skill && (skill.appliesTo.length === 0 || skill.appliesTo.includes(next));
      }),
    );
  };

  const save = async () => {
    if (!name.trim()) {
      setError('预设需要名称');
      return;
    }
    setSaving(true);
    setError('');
    try {
      const payload = { name: name.trim(), description, kind, skillIds, content };
      if (existing) await api.updatePreset(existing.id, payload);
      else await api.createPreset(payload);
      await app.refreshLibrary();
      app.toast({ message: existing ? '预设已更新' : '预设已创建', tone: 'success' });
      onClose();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    if (!existing) return;
    setDeleting(true);
    try {
      await api.deletePreset(existing.id);
      await app.refreshLibrary();
      app.toast({ message: '预设已移入回收站', tone: 'success' });
      setConfirmDelete(false);
      onClose();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '删除失败');
    } finally {
      setDeleting(false);
    }
  };

  return (
    <>
      <Modal
        open={open}
        onClose={onClose}
        title={existing ? '编辑预设' : '新建预设'}
        subtitle="预设决定某一类素材的结构、语气与输出格式。"
        size="xl"
        footer={
          <>
            {existing ? (
              <Button variant="ghost" icon="trash" class="footer-left" onClick={() => setConfirmDelete(true)}>
                删除
              </Button>
            ) : <span />}
            <div class="modal-actions">
              <Button variant="ghost" onClick={onClose}>取消</Button>
              <Button variant="primary" onClick={save} loading={saving}>保存预设</Button>
            </div>
          </>
        }
      >
        <div class="editor-grid">
          <div class="form-stack">
            <Field label="预设名称" required>
              <input class="input" data-autofocus value={name} onInput={(e) => setName((e.target as HTMLInputElement).value)} />
            </Field>
            <Field label="适用素材类型" required>
              <select class="select" value={kind} onChange={(e) => changeKind((e.target as HTMLSelectElement).value as Kind)}>
                {KINDS.map((item) => <option value={item.id} key={item.id}>{item.label}</option>)}
              </select>
            </Field>
            <Field label="一句话说明">
              <input class="input" value={description} onInput={(e) => setDescription((e.target as HTMLInputElement).value)} />
            </Field>
            <Field label="默认加载技能" hint="生成时会自动勾选，可临时调整">
              <div class="check-list compact">
                {applicableSkills.map((skill) => (
                  <label class={`check-row ${skillIds.includes(skill.id) ? 'is-checked' : ''}`} key={skill.id}>
                    <input
                      type="checkbox"
                      checked={skillIds.includes(skill.id)}
                      onChange={() =>
                        setSkillIds((prev) => prev.includes(skill.id) ? prev.filter((x) => x !== skill.id) : [...prev, skill.id])
                      }
                    />
                    <span>
                      <strong>{skill.name}</strong>
                      <small>{skill.description || `${kindLabel(kind)}可用`}</small>
                    </span>
                  </label>
                ))}
              </div>
            </Field>
          </div>
          <div class="editor-main">
            <div class="editor-main-head">
              <span class="field-label">预设正文（Markdown）</span>
              <div class="asset-tab-switch">
                <button class={!preview ? 'is-active' : ''} onClick={() => setPreview(false)}>编辑</button>
                <button class={preview ? 'is-active' : ''} onClick={() => setPreview(true)}>预览</button>
              </div>
            </div>
            {preview ? (
              <div class="editor-preview-scroll">
                <MarkdownView source={content} />
              </div>
            ) : (
              <textarea
                class="textarea editor-textarea"
                value={content}
                placeholder={'# 创作目标\n…\n\n# 结构\n…\n\n# 输出格式\n…\n\n# 质量约束\n…'}
                onInput={(e) => setContent((e.target as HTMLTextAreaElement).value)}
              />
            )}
          </div>
          {error ? <p class="form-error editor-error">{error}</p> : null}
        </div>
      </Modal>
      <ConfirmDialog
        open={confirmDelete}
        title="删除这个预设？"
        message="预设文件会移入回收站。已有的项目副本不受影响。"
        loading={deleting}
        onClose={() => setConfirmDelete(false)}
        onConfirm={remove}
      />
    </>
  );
}
