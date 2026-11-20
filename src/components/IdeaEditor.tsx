import { useEffect, useLayoutEffect, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import type { Idea } from '../lib/types';
import { Button, ConfirmDialog, Field, Modal, TagInput } from './ui';

export function IdeaEditor({
  idea,
  open,
  onClose,
  onSaved,
}: {
  idea: Idea | null;
  open: boolean;
  onClose: () => void;
  onSaved?: (idea: Idea) => void;
}) {
  const app = useApp();
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [tags, setTags] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [error, setError] = useState('');

  useLayoutEffect(() => {
    if (open) {
      setTitle(idea?.title ?? '');
      setContent(idea?.content ?? '');
      setTags(idea?.tags ?? []);
      setError('');
    }
  }, [open, idea]);

  const save = async () => {
    if (!title.trim()) {
      setError('标题不能为空');
      return;
    }
    setSaving(true);
    setError('');
    try {
      const saved = idea
        ? await api.updateIdea(idea.id, { title: title.trim(), content, tags })
        : await api.createIdea({ title: title.trim(), content, tags });
      await Promise.all([app.refreshIdeas(), app.refreshWorkspace()]);
      app.toast({ message: idea ? '想法已更新' : '想法已创建', tone: 'success' });
      onSaved?.(saved);
      onClose();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const remove = async () => {
    if (!idea) return;
    setDeleting(true);
    try {
      await api.deleteIdea(idea.id);
      await Promise.all([app.refreshIdeas(), app.refreshProjects(), app.refreshWorkspace()]);
      app.toast({ message: '想法已移入回收站', tone: 'success' });
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
        title={idea ? '编辑想法' : '新建想法'}
        subtitle={idea ? `创建于 ${new Date(idea.createdAt).toLocaleDateString('zh-CN')}` : '先写下来，再慢慢想清楚。'}
        size="lg"
        footer={
          <>
            {idea ? (
              <Button variant="ghost" icon="trash" class="footer-left" onClick={() => setConfirmDelete(true)}>
                删除
              </Button>
            ) : null}
            <div class="modal-actions">
              <Button variant="ghost" onClick={onClose}>
                取消
              </Button>
              <Button variant="primary" onClick={save} loading={saving}>
                保存
              </Button>
            </div>
          </>
        }
      >
        <div class="form-stack">
          <Field label="标题" required>
            <input
              class="input"
              data-autofocus
              value={title}
              onInput={(e) => setTitle((e.target as HTMLInputElement).value)}
            />
          </Field>
          <Field label="正文">
            <textarea
              class="textarea idea-textarea"
              value={content}
              placeholder="展开写，或者只留一句话。"
              onInput={(e) => setContent((e.target as HTMLTextAreaElement).value)}
            />
          </Field>
          <Field label="标签">
            <TagInput value={tags} onChange={setTags} />
          </Field>
          {idea?.projectId ? (
            <p class="form-hint-line">这条想法已经关联到一个项目。</p>
          ) : null}
          {error ? <p class="form-error">{error}</p> : null}
        </div>
      </Modal>
      <ConfirmDialog
        open={confirmDelete}
        title="删除这个想法？"
        message="文件会移入工作区的 .mediadesk/trash，可在文件管理器中找回。"
        loading={deleting}
        onClose={() => setConfirmDelete(false)}
        onConfirm={remove}
      />
    </>
  );
}
