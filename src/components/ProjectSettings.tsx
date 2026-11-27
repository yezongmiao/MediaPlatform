import { useEffect, useLayoutEffect, useMemo, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import type { Project, ProjectStatus } from '../lib/types';
import { PROJECT_STATUS } from '../lib/types';
import { Button, ConfirmDialog, Field, Modal, TagInput } from './ui';
import { Icon } from './Icon';
import { navigate } from '../lib/router';
import { excerpt, relativeTime } from '../lib/format';

export function ProjectSettings({
  project,
  open,
  onClose,
  onSaved,
}: {
  project: Project;
  open: boolean;
  onClose: () => void;
  onSaved: (project: Project) => void | Promise<void>;
}) {
  const app = useApp();
  const [title, setTitle] = useState('');
  const [brief, setBrief] = useState('');
  const [status, setStatus] = useState<ProjectStatus>('active');
  const [tags, setTags] = useState<string[]>([]);
  const [ideaIds, setIdeaIds] = useState<string[]>([]);
  const [skillIds, setSkillIds] = useState<string[]>([]);
  const [presetIds, setPresetIds] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [error, setError] = useState('');
  const [ideaSearch, setIdeaSearch] = useState('');

  useLayoutEffect(() => {
    if (!open) return;
    setTitle(project.title);
    setBrief(project.brief);
    setStatus(project.status);
    setTags(project.tags);
    setIdeaIds(project.ideaIds);
    setSkillIds(project.skillIds);
    setPresetIds(project.presetIds);
    setError('');
    setIdeaSearch('');
  }, [open, project]);

  const ideas = useMemo(() => {
    const needle = ideaSearch.trim().toLowerCase();
    return app.data.ideas
      .filter(
        (idea) =>
          !needle ||
          idea.title.toLowerCase().includes(needle) ||
          idea.content.toLowerCase().includes(needle),
      )
      .slice(0, 30);
  }, [app.data.ideas, ideaSearch]);

  const toggle = (list: string[], id: string) =>
    list.includes(id) ? list.filter((x) => x !== id) : [...list, id];

  const save = async () => {
    if (!title.trim()) {
      setError('项目标题不能为空');
      return;
    }
    setSaving(true);
    setError('');
    try {
      const updated = await api.updateProject(project.id, {
        title: title.trim(),
        brief,
        status,
        tags,
        ideaIds,
        skillIds,
        presetIds,
      });
      await Promise.all([
        app.refreshProjects(),
        app.refreshIdeas(),
        app.refreshWorkspace(),
      ]);
      app.toast({ message: '项目设置已保存', tone: 'success' });
      await onSaved(updated);
      onClose();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    setDeleting(true);
    try {
      await api.deleteProject(project.id);
      await Promise.all([
        app.refreshProjects(),
        app.refreshIdeas(),
        app.refreshWorkspace(),
      ]);
      app.toast({ message: '项目已移入回收站', tone: 'success' });
      setConfirmDelete(false);
      onClose();
      navigate('/projects');
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
        title="项目设置"
        subtitle="素材会随项目目录一起管理；技能与预设会复制进 VSCode 工程。"
        size="xl"
        footer={
          <>
            <Button variant="ghost" icon="trash" class="footer-left" onClick={() => setConfirmDelete(true)}>
              删除项目
            </Button>
            <div class="modal-actions">
              <Button variant="ghost" onClick={onClose}>
                取消
              </Button>
              <Button variant="primary" onClick={save} loading={saving}>
                保存设置
              </Button>
            </div>
          </>
        }
      >
        <div class="settings-grid">
          <section class="settings-section">
            <h3>基本信息</h3>
            <Field label="项目标题" required>
              <input
                class="input"
                data-autofocus
                value={title}
                onInput={(e) => setTitle((e.target as HTMLInputElement).value)}
              />
            </Field>
            <Field label="项目简介">
              <textarea
                class="textarea"
                rows={4}
                value={brief}
                onInput={(e) => setBrief((e.target as HTMLTextAreaElement).value)}
              />
            </Field>
            <div class="form-row">
              <Field label="状态">
                <select
                  class="select"
                  value={status}
                  onChange={(e) => setStatus((e.target as HTMLSelectElement).value as ProjectStatus)}
                >
                  {Object.entries(PROJECT_STATUS).map(([value, item]) => (
                    <option value={value} key={value}>
                      {item.label}
                    </option>
                  ))}
                </select>
              </Field>
              <Field label="标签">
                <TagInput value={tags} onChange={setTags} />
              </Field>
            </div>
            <div class="project-meta-line">
              <span>创建于 {new Date(project.createdAt).toLocaleDateString('zh-CN')}</span>
              <span>更新于 {relativeTime(project.updatedAt)}</span>
              <span>目录 <code>{project.dirName}</code></span>
            </div>
          </section>

          <section class="settings-section">
            <h3>关联灵感</h3>
            <p class="section-note">取消关联不会删除想法，只会把它从项目上下文中移除。</p>
            <input
              class="input"
              placeholder="筛选想法…"
              value={ideaSearch}
              onInput={(e) => setIdeaSearch((e.target as HTMLInputElement).value)}
            />
            <div class="check-list">
              {ideas.map((idea) => {
                const belongsElsewhere = !!idea.projectId && idea.projectId !== project.id;
                return (
                <label class={`check-row ${ideaIds.includes(idea.id) ? 'is-checked' : ''} ${belongsElsewhere ? 'is-disabled' : ''}`} key={idea.id} title={belongsElsewhere ? '该想法已属于其他项目' : undefined}>
                  <input
                    type="checkbox"
                    checked={ideaIds.includes(idea.id)}
                    disabled={belongsElsewhere}
                    onChange={() => setIdeaIds(toggle(ideaIds, idea.id))}
                  />
                  <span>
                    <strong>{idea.title}</strong>
                    <small>{excerpt(idea.excerpt, 60)}</small>
                  </span>
                  <em>{belongsElsewhere ? '已属于其他项目' : relativeTime(idea.updatedAt)}</em>
                </label>
                );
              })}
              {ideas.length === 0 ? <p class="muted-text">没有匹配的想法。</p> : null}
            </div>
          </section>

          <section class="settings-section">
            <h3>技能</h3>
            <p class="section-note">勾选后会复制到项目 <code>skills/</code>，供外部 AI 读取；不选则默认带全部技能。</p>
            <div class="check-grid">
              {app.data.skills.map((skill) => (
                <label class={`check-tile ${skillIds.includes(skill.id) ? 'is-checked' : ''}`} key={skill.id}>
                  <input
                    type="checkbox"
                    checked={skillIds.includes(skill.id)}
                    onChange={() => setSkillIds(toggle(skillIds, skill.id))}
                  />
                  <span>
                    <strong>{skill.name}</strong>
                    <small>{skill.description}</small>
                  </span>
                </label>
              ))}
            </div>
          </section>

          <section class="settings-section">
            <h3>预设</h3>
            <p class="section-note">预设会复制到项目 <code>presets/</code>。生成素材时仍可在生成弹层里临时切换。</p>
            <div class="check-grid">
              {app.data.presets.map((preset) => (
                <label class={`check-tile ${presetIds.includes(preset.id) ? 'is-checked' : ''}`} key={preset.id}>
                  <input
                    type="checkbox"
                    checked={presetIds.includes(preset.id)}
                    onChange={() => setPresetIds(toggle(presetIds, preset.id))}
                  />
                  <span>
                    <strong>{preset.name}</strong>
                    <small>{preset.description}</small>
                  </span>
                </label>
              ))}
            </div>
          </section>

          <section class="settings-section settings-note-block">
            <Icon name="code" size={18} />
            <div>
              <h3>外部工作区说明</h3>
              <p>
                项目页的「打开」菜单可以进入 VS Code、DeepSeek Harness 或文件管理器。进入 VS Code 时会生成
                <code>AGENTS.md</code>、<code>CLAUDE.md</code>、<code>.vscode/</code>、<code>skills/</code>、
                <code>presets/</code> 与 <code>.mediadesk/context.json</code>，然后调用本机 <code>code</code> 命令。
              </p>
            </div>
          </section>

          {error ? <p class="form-error settings-error">{error}</p> : null}
        </div>
      </Modal>

      <ConfirmDialog
        open={confirmDelete}
        title="删除整个项目？"
        message={`「${project.title}」及全部素材会移入 .mediadesk/trash。关联的想法会回到待立项状态。`}
        loading={deleting}
        onClose={() => setConfirmDelete(false)}
        onConfirm={remove}
      />
    </>
  );
}
