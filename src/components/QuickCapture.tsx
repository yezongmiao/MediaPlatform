import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import { Button, Field, Modal, TagInput } from './ui';
import { navigate } from '../lib/router';

export function QuickCapture({ open, onClose }: { open: boolean; onClose: () => void }) {
  const app = useApp();
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [tags, setTags] = useState<string[]>([]);
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useLayoutEffect(() => {
    if (open) {
      setTitle('');
      setContent('');
      setTags([]);
      setError('');
      window.setTimeout(() => textareaRef.current?.focus(), 80);
    }
  }, [open]);

  const save = async () => {
    if (!content.trim() && !title.trim()) {
      setError('先写点什么吧。');
      textareaRef.current?.focus();
      return;
    }
    const firstLine = content
      .split('\n')
      .map((line) => line.trim())
      .find(Boolean);
    const resolvedTitle = (title.trim() || firstLine || '无标题想法')
      .replace(/^#+\s*/, '')
      .slice(0, 80);
    setSaving(true);
    setError('');
    try {
      const idea = await api.createIdea({
        title: resolvedTitle,
        content: content.trim(),
        tags,
      });
      await Promise.all([app.refreshIdeas(), app.refreshWorkspace()]);
      app.toast({ message: '想法已收进灵感箱', tone: 'success' });
      onClose();
      if (window.location.pathname !== '/ideas') {
        navigate('/ideas');
      }
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存失败，请重试');
    } finally {
      setSaving(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="速记一个想法"
      subtitle="不用整理，先写下来。之后再立项、找素材。"
      size="lg"
      class="quick-capture-modal"
      footer={
        <>
          <span class="shortcut-hint">⌘ / Ctrl + Enter 保存</span>
          <div class="modal-actions">
            <Button variant="ghost" onClick={onClose}>
              取消
            </Button>
            <Button variant="primary" icon="pen" loading={saving} onClick={save}>
              收进灵感箱
            </Button>
          </div>
        </>
      }
    >
      <div class="quick-capture-grid">
        <Field label="标题" hint="留空则自动取正文第一行">
          <input
            class="input"
            value={title}
            placeholder="给这个想法一个名字"
            onInput={(e) => setTitle((e.target as HTMLInputElement).value)}
          />
        </Field>
        <Field label="想法" required>
          <textarea
            ref={textareaRef}
            class="textarea quick-textarea"
            value={content}
            placeholder={'此刻在想什么？\n\n可以是一句话、一个观点、一个画面、一个还没想清楚的念头……'}
            onInput={(e) => setContent((e.target as HTMLTextAreaElement).value)}
            onKeyDown={(e) => {
              if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
                e.preventDefault();
                save();
              }
            }}
          />
        </Field>
        <Field label="标签" hint="回车添加，最多 8 个">
          <TagInput value={tags} onChange={setTags} />
        </Field>
        {error ? <p class="form-error">{error}</p> : null}
      </div>
    </Modal>
  );
}
