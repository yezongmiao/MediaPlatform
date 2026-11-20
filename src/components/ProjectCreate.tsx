import { useEffect, useLayoutEffect, useMemo, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import type { Idea } from '../lib/types';
import { Button, Field, Modal } from './ui';
import { navigate } from '../lib/router';
import { excerpt, relativeTime } from '../lib/format';

export function ProjectCreate({
  open,
  initialIdeas = [],
  onClose,
  onCreated,
}: {
  open: boolean;
  initialIdeas?: Idea[];
  onClose: () => void;
  onCreated?: (projectId: string) => void;
}) {
  const app = useApp();
  const [title, setTitle] = useState('');
  const [brief, setBrief] = useState('');
  const [selected, setSelected] = useState<string[]>([]);
  const [search, setSearch] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  useLayoutEffect(() => {
    if (!open) return;
    const first = initialIdeas[0];
    setTitle(first?.title ?? '');
    setBrief(first?.content ?? '');
    setSelected(initialIdeas.map((idea) => idea.id));
    setSearch('');
    setError('');
  }, [open, initialIdeas]);

  const ideas = useMemo(() => {
    const needle = search.trim().toLowerCase();
    const eligible = app.data.ideas.filter(
      (idea) =>
        !idea.projectId ||
        initialIdeas.some((initial) => initial.id === idea.id),
    );
    const list = needle
      ? eligible.filter(
          (idea) =>
            idea.title.toLowerCase().includes(needle) ||
            idea.content.toLowerCase().includes(needle) ||
            idea.tags.some((tag) => tag.toLowerCase().includes(needle)),
        )
      : eligible;
    return list.slice(0, 40);
  }, [app.data.ideas, search, initialIdeas]);

  const toggle = (id: string) => {
    setSelected((prev) => (prev.includes(id) ? prev.filter((x) => x !== id) : [...prev, id]));
  };

  const create = async () => {
    if (!title.trim()) {
      setError('项目需要一个标题');
      return;
    }
    setSaving(true);
    setError('');
    try {
      const project = await api.createProject({
        title: title.trim(),
        brief,
        ideaIds: selected,
      });
      await Promise.all([
        app.refreshProjects(),
        app.refreshIdeas(),
        app.refreshWorkspace(),
      ]);
      app.toast({ message: '项目已建立', tone: 'success' });
      onCreated?.(project.id);
      onClose();
      navigate(`/projects/${project.id}`);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '创建项目失败');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="建立新项目"
      subtitle="把想法升级成一条可执行的内容线，五类素材各归其位。"
      size="lg"
      footer={
        <>
          <span class="shortcut-hint">关联 {selected.length} 条想法</span>
          <div class="modal-actions">
            <Button variant="ghost" onClick={onClose}>
              取消
            </Button>
            <Button variant="primary" icon="plus" loading={saving} onClick={create}>
              建立项目
            </Button>
          </div>
        </>
      }
    >
      <div class="form-stack">
        <Field label="项目标题" required>
          <input
            class="input"
            data-autofocus
            value={title}
            placeholder="例如：AI 时代的个人创作方法论"
            onInput={(e) => setTitle((e.target as HTMLInputElement).value)}
          />
        </Field>
        <Field label="项目简介" hint="可先写一句，之后在项目设置里补充">
          <textarea
            class="textarea"
            value={brief}
            placeholder="这条内容想解决什么问题？给谁看？"
            onInput={(e) => setBrief((e.target as HTMLTextAreaElement).value)}
          />
        </Field>
        <Field label="从灵感立项" hint="可多选；立项后想法会自动标记为已立项">
          <div class="idea-picker">
            <input
              class="input idea-picker-search"
              placeholder="筛选想法…"
              value={search}
              onInput={(e) => setSearch((e.target as HTMLInputElement).value)}
            />
            <div class="idea-picker-list">
              {ideas.length === 0 ? (
                <p class="picker-empty">没有可关联的未立项想法，也可以直接创建空项目。</p>
              ) : (
                ideas.map((idea) => (
                  <label class={`picker-row ${selected.includes(idea.id) ? 'is-checked' : ''}`} key={idea.id}>
                    <input
                      type="checkbox"
                      checked={selected.includes(idea.id)}
                      onChange={() => toggle(idea.id)}
                    />
                    <span class="picker-row-main">
                      <strong>{idea.title}</strong>
                      <small>{excerpt(idea.excerpt, 80)}</small>
                    </span>
                    <span class="picker-meta">{relativeTime(idea.updatedAt)}</span>
                  </label>
                ))
              )}
            </div>
          </div>
        </Field>
        {error ? <p class="form-error">{error}</p> : null}
      </div>
    </Modal>
  );
}
