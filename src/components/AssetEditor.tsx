import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import type { Asset, AssetStatus, Kind, Project } from '../lib/types';
import { ASSET_STATUS, KIND_MAP } from '../lib/types';
import { Button, ConfirmDialog, Field, IconButton, Modal, TagInput } from './ui';
import { Icon } from './Icon';
import { MarkdownView } from './MarkdownView';
import { copyText, downloadUrl, formatDate, kindLabel } from '../lib/format';

const CONTENT_LABEL: Record<Kind, string> = {
  article: '正文',
  script: '脚本',
  illustration: '图像提示词与备注',
  card: '卡片文案',
  video: '制作笔记',
};

export function AssetEditor({
  project,
  asset,
  createKind,
  open,
  onClose,
  onSaved,
  onOpenCardStudio,
}: {
  project: Project;
  asset: Asset | null;
  createKind?: Kind;
  open: boolean;
  onClose: () => void;
  onSaved: (assetId?: string) => void | Promise<void>;
  onOpenCardStudio?: (text: string) => void;
}) {
  const app = useApp();
  const [current, setCurrent] = useState<Asset | null>(asset);
  const [title, setTitle] = useState('');
  const [content, setContent] = useState('');
  const [status, setStatus] = useState<AssetStatus>('draft');
  const [tags, setTags] = useState<string[]>([]);
  const [tab, setTab] = useState<'write' | 'preview'>('write');
  const [saving, setSaving] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [deleting, setDeleting] = useState(false);
  const [confirmDelete, setConfirmDelete] = useState(false);
  const [confirmDiscard, setConfirmDiscard] = useState(false);
  const [error, setError] = useState('');
  const [initial, setInitial] = useState('');
  const fileInputRef = useRef<HTMLInputElement>(null);

  const kind = (current?.kind ?? createKind ?? 'article') as Kind;
  const meta = KIND_MAP[kind];
  const isCreating = !asset;

  useLayoutEffect(() => {
    if (!open) return;
    setCurrent(asset);
    setTitle(asset?.title ?? '');
    setContent(asset?.content ?? '');
    setStatus(asset?.status ?? 'draft');
    setTags(asset?.tags ?? []);
    setTab(asset && asset.content ? 'preview' : 'write');
    setError('');
    setInitial(
      JSON.stringify({
        title: asset?.title ?? '',
        content: asset?.content ?? '',
        status: asset?.status ?? 'draft',
        tags: asset?.tags ?? [],
      }),
    );
  }, [open, asset]);

  const dirty =
    JSON.stringify({ title, content, status, tags }) !== initial;

  const close = () => {
    if (dirty) {
      setConfirmDiscard(true);
      return;
    }
    onClose();
  };

  const save = async () => {
    if (!title.trim()) {
      setError('标题不能为空');
      return;
    }
    setSaving(true);
    setError('');
    try {
      let saved: Asset;
      if (current) {
        saved = await api.updateAsset(project.id, current.id, {
          title: title.trim(),
          content,
          status,
          tags,
        });
      } else {
        saved = await api.createAsset(project.id, {
          kind,
          title: title.trim(),
          content,
          status,
          tags,
        });
      }
      setCurrent(saved);
      setInitial(JSON.stringify({ title: saved.title, content: saved.content, status: saved.status, tags: saved.tags }));
      await Promise.all([app.refreshWorkspace(), app.refreshProjects()]);
      app.toast({ message: isCreating ? '素材已创建' : '素材已保存', tone: 'success' });
      await onSaved(saved.id);
      onClose();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const upload = async (file: File) => {
    if (!current) return;
    setUploading(true);
    setError('');
    try {
      const updated = await api.uploadAsset(project.id, kind, file, {
        assetId: current.id,
        title: title.trim() || undefined,
      });
      setCurrent(updated);
      setTitle(updated.title);
      await Promise.all([app.refreshProjects(), app.refreshWorkspace()]);
      app.toast({ message: '媒体文件已替换', tone: 'success' });
      await onSaved(updated.id);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '上传失败');
    } finally {
      setUploading(false);
      if (fileInputRef.current) fileInputRef.current.value = '';
    }
  };

  const remove = async () => {
    if (!current) return;
    setDeleting(true);
    try {
      await api.deleteAsset(project.id, current.id);
      await Promise.all([app.refreshProjects(), app.refreshWorkspace()]);
      app.toast({ message: '素材已移入回收站', tone: 'success' });
      setConfirmDelete(false);
      onClose();
      await onSaved();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '删除失败');
    } finally {
      setDeleting(false);
    }
  };

  useEffect(() => {
    if (!open) return;
    const onKey = (event: KeyboardEvent) => {
      if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === 's') {
        event.preventDefault();
        save();
      }
    };
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  });

  const hasMedia = !!current?.fileUrl;
  const isVideo = kind === 'video';

  return (
    <>
      <Modal
        open={open}
        onClose={close}
        title={
          <span class="asset-editor-title">
            <span class="kind-indicator" style={{ background: meta.accent }} />
            {isCreating ? `新建${kindLabel(kind)}` : current?.title || kindLabel(kind)}
          </span>
        }
        subtitle={`${project.title} · ${kindLabel(kind)}`}
        size="full"
        class="asset-editor-modal"
        footer={
          <>
            {current ? (
              <Button variant="ghost" icon="trash" class="footer-left" onClick={() => setConfirmDelete(true)}>
                删除
              </Button>
            ) : <span />}
            <div class="modal-actions">
              {hasMedia && current?.fileUrl ? (
                <Button
                  variant="ghost"
                  icon="download"
                  onClick={() => downloadUrl(current.fileUrl!, current.file ?? `${current.title}.${isVideo ? 'mp4' : 'png'}`)}
                >
                  下载
                </Button>
              ) : null}
              {content ? (
                <Button
                  variant="ghost"
                  icon="copy"
                  onClick={async () => {
                    await copyText(content);
                    app.toast({ message: '内容已复制', tone: 'success' });
                  }}
                >
                  复制
                </Button>
              ) : null}
              <Button variant="ghost" onClick={close}>
                取消
              </Button>
              <Button variant="primary" loading={saving} onClick={save}>
                保存
              </Button>
            </div>
          </>
        }
      >
        <div class="asset-editor">
          <div class="asset-editor-main">
            {hasMedia ? (
              <div class={`asset-media-preview ${isVideo ? 'is-video' : ''}`}>
                {isVideo ? (
                  <video src={current!.fileUrl} controls preload="metadata" />
                ) : (
                  <img src={current!.fileUrl} alt={current!.title} />
                )}
                <div class="asset-media-actions">
                  <input
                    ref={fileInputRef}
                    type="file"
                    hidden
                    accept={meta.accept}
                    onChange={(e) => {
                      const file = (e.target as HTMLInputElement).files?.[0];
                      if (file) upload(file);
                    }}
                  />
                  <Button
                    size="sm"
                    variant="secondary"
                    icon="upload"
                    loading={uploading}
                    onClick={() => fileInputRef.current?.click()}
                  >
                    替换文件
                  </Button>
                </div>
              </div>
            ) : kind === 'card' && current ? (
              <div class="asset-media-placeholder card-placeholder">
                <Icon name="cards" size={28} />
                <h3>还没有画面</h3>
                <p>用文卡图 Studio 把这段文案排成一张可以发布的图片。</p>
                <Button
                  variant="primary"
                  icon="wand"
                  onClick={() => {
                    onOpenCardStudio?.(content || current.title);
                    onClose();
                  }}
                >
                  去排版
                </Button>
              </div>
            ) : kind === 'illustration' && current ? (
              <div class="asset-media-placeholder">
                <Icon name="image" size={28} />
                <h3>提示词草稿</h3>
                <p>把右侧提示词复制到图像模型生成图片，再回到这里上传。</p>
                <input
                  ref={fileInputRef}
                  type="file"
                  hidden
                  accept={meta.accept}
                  onChange={(e) => {
                    const file = (e.target as HTMLInputElement).files?.[0];
                    if (file) upload(file);
                  }}
                />
                <Button variant="primary" icon="upload" loading={uploading} onClick={() => fileInputRef.current?.click()}>
                  上传图片
                </Button>
              </div>
            ) : null}

            <div class="asset-markdown-panel">
              <div class="asset-panel-head">
                <span class="asset-panel-title">{CONTENT_LABEL[kind]}</span>
                <div class="asset-tab-switch">
                  <button class={tab === 'write' ? 'is-active' : ''} onClick={() => setTab('write')}>
                    编辑
                  </button>
                  <button class={tab === 'preview' ? 'is-active' : ''} onClick={() => setTab('preview')}>
                    预览
                  </button>
                </div>
              </div>
              {tab === 'write' ? (
                <textarea
                  class="textarea asset-textarea"
                  value={content}
                  placeholder={
                    kind === 'article'
                      ? '从大纲开始，或直接写正文…'
                      : kind === 'script'
                        ? '写下口播、分镜或镜头提示…'
                        : kind === 'illustration'
                          ? '记录可复用的图像提示词、负面词与比例…'
                          : kind === 'card'
                            ? '写下卡片的短文案…'
                            : '记录素材来源、剪辑思路与待办…'
                  }
                  onInput={(e) => setContent((e.target as HTMLTextAreaElement).value)}
                />
              ) : (
                <div class="asset-preview-scroll">
                  {content.trim() ? (
                    <MarkdownView source={content} />
                  ) : (
                    <p class="muted-text">还没有内容。切回「编辑」开始写吧。</p>
                  )}
                </div>
              )}
            </div>
          </div>

          <aside class="asset-side">
            <Field label="标题" required>
              <input
                class="input"
                data-autofocus
                value={title}
                onInput={(e) => setTitle((e.target as HTMLInputElement).value)}
              />
            </Field>
            <Field label="状态">
              <select
                class="select"
                value={status}
                onChange={(e) => setStatus((e.target as HTMLSelectElement).value as AssetStatus)}
              >
                {Object.entries(ASSET_STATUS).map(([value, item]) => (
                  <option value={value} key={value}>
                    {item.label}
                  </option>
                ))}
              </select>
            </Field>
            <Field label="标签">
              <TagInput value={tags} onChange={setTags} />
            </Field>
            <div class="asset-info">
              <span class="asset-info-row">
                <small>类型</small>
                <strong>{kindLabel(kind)}</strong>
              </span>
              <span class="asset-info-row">
                <small>创建</small>
                <strong>{current ? formatDate(current.createdAt, true) : '现在'}</strong>
              </span>
              <span class="asset-info-row">
                <small>来源</small>
                <strong>{current?.source === 'llm' ? '大模型生成' : '手动创作'}</strong>
              </span>
              {current?.file ? (
                <span class="asset-info-row">
                  <small>文件</small>
                  <strong class="asset-file-name" title={current.file}>{current.file}</strong>
                </span>
              ) : null}
            </div>
            {current?.prompt ? (
              <div class="asset-prompt-box">
                <span class="field-label">保存的提示词</span>
                <p>{current.prompt}</p>
              </div>
            ) : null}
            <div class="side-tip">
              <Icon name="code" size={15} />
              <p>所有改动都会写入项目目录中的 Markdown 文件，可直接被 VSCode 和外部 AI 读取。</p>
            </div>
            {error ? <p class="form-error">{error}</p> : null}
          </aside>
        </div>
      </Modal>

      <ConfirmDialog
        open={confirmDelete}
        title="删除这个素材？"
        message={`「${current?.title ?? ''}」会移入 .mediadesk/trash，媒体文件一并移动。`}
        loading={deleting}
        onClose={() => setConfirmDelete(false)}
        onConfirm={remove}
      />
      <ConfirmDialog
        open={confirmDiscard}
        title="放弃未保存的修改？"
        message="当前编辑内容还没有保存。"
        confirmLabel="放弃"
        danger={false}
        onClose={() => setConfirmDiscard(false)}
        onConfirm={() => {
          setConfirmDiscard(false);
          onClose();
        }}
      />
    </>
  );
}
