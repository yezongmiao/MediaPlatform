import { useLayoutEffect, useRef, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import type { BrowseInfo } from '../lib/types';
import { Button, InlineError, Modal } from './ui';
import { Icon } from './Icon';

export function FolderBrowser({
  open,
  initialPath,
  onSelect,
  onClose,
}: {
  open: boolean;
  initialPath?: string;
  onSelect: (path: string) => Promise<void> | void;
  onClose: () => void;
}) {
  const [info, setInfo] = useState<BrowseInfo | null>(null);
  const [loading, setLoading] = useState(false);
  const [selecting, setSelecting] = useState(false);
  const [error, setError] = useState('');
  const pathEdited = useRef(false);
  const requestId = useRef(0);
  const inputRef = useRef<HTMLInputElement>(null);

  const load = async (
    path?: string,
    options: { preserveEditedPath?: boolean; updateInput?: boolean } = {},
  ) => {
    const id = ++requestId.current;
    setLoading(true);
    setError('');
    try {
      const data = await api.browse(path);
      if (id !== requestId.current) return;
      setInfo(data);
      if (
        options.updateInput !== false &&
        inputRef.current &&
        (!options.preserveEditedPath || !pathEdited.current)
      ) {
        inputRef.current.value = data.path;
        pathEdited.current = false;
      }
    } catch (err) {
      if (id !== requestId.current) return;
      setError(err instanceof ApiError ? err.message : '无法读取目录');
    } finally {
      if (id === requestId.current) setLoading(false);
    }
  };

  useLayoutEffect(() => {
    if (!open) return;
    pathEdited.current = false;
    if (inputRef.current) inputRef.current.value = initialPath ?? '';
    load(initialPath, { preserveEditedPath: true, updateInput: false });
  }, [open, initialPath]);

  // Read the live DOM value so a click that lands between keystroke and render
  // still navigates to exactly what the user typed.
  const loadTypedPath = () => {
    const value = inputRef.current?.value.trim();
    if (!value) return;
    pathEdited.current = false;
    load(value);
  };

  const choose = async () => {
    if (!info) return;
    setSelecting(true);
    try {
      await onSelect(info.path);
      onClose();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '切换工作区失败');
    } finally {
      setSelecting(false);
    }
  };

  return (
    <Modal
      open={open}
      onClose={onClose}
      title="选择工作区文件夹"
      subtitle="所有想法、项目与素材都会保存在这个目录。可以直接选择已有工作区。"
      size="lg"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            取消
          </Button>
          <Button variant="primary" icon="folder-open" onClick={choose} loading={selecting} disabled={!info || loading}>
            使用此文件夹
          </Button>
        </>
      }
    >
      <div class="folder-browser">
        <div class="folder-pathbar">
          <Button size="sm" variant="secondary" icon="arrow-left" onClick={() => { pathEdited.current = false; if (info?.parent) load(info.parent); }} disabled={!info?.parent}>
            上一级
          </Button>
          <input
            ref={inputRef}
            class="input folder-path-input"
            defaultValue=""
            aria-label="目录路径"
            placeholder={loading ? '正在读取目录…' : '/path/to/workspace'}
            onInput={() => {
              pathEdited.current = true;
            }}
            onKeyDown={(e) => {
              if (e.key === 'Enter') loadTypedPath();
            }}
          />
          <Button size="sm" variant="secondary" icon="refresh" onClick={loadTypedPath}>
            前往
          </Button>
        </div>
        <div class="folder-hint-row">
          {info?.isWorkspace ? (
            <span class="folder-workspace-badge">已有工作区</span>
          ) : (
            <span class="muted-text">新目录会自动创建 MediaDesk 结构</span>
          )}
          <span class="muted-text">{info?.dirs.length ?? 0} 个子文件夹</span>
        </div>
        {error ? <InlineError message={error} onRetry={loadTypedPath} /> : null}
        <div class="folder-list">
          {loading ? (
            <div class="folder-loading">
              <span class="spinner" />
              正在读取目录…
            </div>
          ) : info?.dirs.length ? (
            info.dirs.map((dir) => (
              <button key={dir.path} class="folder-row" onClick={() => { pathEdited.current = false; load(dir.path); }}>
                <Icon name="folder" size={17} />
                <span>{dir.name}</span>
                <Icon name="chevron-right" size={14} />
              </button>
            ))
          ) : (
            <div class="folder-loading">这里没有子文件夹</div>
          )}
        </div>
      </div>
    </Modal>
  );
}
