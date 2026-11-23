import { useCallback, useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import { isImeComposing } from '../lib/ime';
import type { Asset, Kind, Project } from '../lib/types';
import { KINDS, KIND_MAP } from '../lib/types';
import { AssetCard } from '../components/AssetCard';
import { AssistantPanel } from '../components/AssistantPanel';
import { AssetEditor } from '../components/AssetEditor';
import { AiCardStudio } from '../components/AiCardStudio';
import { GenerateDialog } from '../components/GenerateDialog';
import { ProjectSettings } from '../components/ProjectSettings';
import { Icon } from '../components/Icon';
import { Badge, Button, ConfirmDialog, EmptyState, Modal, Skeleton } from '../components/ui';
import { navigate, useQueryParam } from '../lib/router';
import { copyText, downloadUrl, formatFullDate, kindLabel, projectInitials, relativeTime, statusLabel, statusTone } from '../lib/format';

type Tab = 'overview' | Kind;

interface VscodeFallback {
  projectPath: string;
  manualCommand: string;
  filesWritten: string[];
}

export function ProjectPage({ projectId }: { projectId: string }) {
  const app = useApp();
  const focusAssetId = useQueryParam('asset');
  const [project, setProject] = useState<Project | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState('');
  const [tab, setTab] = useState<Tab>('overview');
  const [editorOpen, setEditorOpen] = useState(false);
  const [editorAsset, setEditorAsset] = useState<Asset | null>(null);
  const [createKind, setCreateKind] = useState<Kind | undefined>();
  const [generateOpen, setGenerateOpen] = useState(false);
  const [assistantOpen, setAssistantOpen] = useState(false);
  const [generateKind, setGenerateKind] = useState<Kind>('article');
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [aiCardOpen, setAiCardOpen] = useState(false);
  const [selectionMode, setSelectionMode] = useState(false);
  const [selectedIds, setSelectedIds] = useState<Set<string>>(new Set());
  const [deleteTargets, setDeleteTargets] = useState<Asset[]>([]);
  const [deleting, setDeleting] = useState(false);
  const [vscodeLoading, setVscodeLoading] = useState(false);
  const [vscodeFallback, setVscodeFallback] = useState<VscodeFallback | null>(null);
  const [uploadKind, setUploadKind] = useState<Kind | null>(null);
  const [uploading, setUploading] = useState(false);
  const [dragActive, setDragActive] = useState(false);
  const [lightbox, setLightbox] = useState<Asset | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  const pendingUploadKind = useRef<Kind>('illustration');

  const load = useCallback(async () => {
    setLoading(true);
    setError('');
    try {
      const data = await api.getProject(projectId);
      setProject(data);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '无法加载项目');
    } finally {
      setLoading(false);
    }
  }, [projectId]);

  useEffect(() => {
    load();
  }, [load]);

  useEffect(() => {
    if (!project || !focusAssetId) return;
    for (const kind of KINDS) {
      const found = project.assets[kind.id]?.find((asset) => asset.id === focusAssetId);
      if (found) {
        setTab(kind.id);
        setEditorAsset(found);
        setCreateKind(undefined);
        setEditorOpen(true);
        navigate(`/projects/${project.id}`, { replace: true });
        break;
      }
    }
  }, [project, focusAssetId]);

  const refresh = useCallback(async () => {
    try {
      const data = await api.getProject(projectId);
      setProject(data);
      return data;
    } catch (err) {
      app.toast({
        message: err instanceof ApiError ? err.message : '刷新项目失败',
        tone: 'error',
      });
      return null;
    }
  }, [projectId, app]);

  const openAsset = (asset: Asset) => {
    setEditorAsset(asset);
    setCreateKind(undefined);
    setEditorOpen(true);
  };

  const openCreate = (kind: Kind) => {
    setCreateKind(kind);
    setEditorAsset(null);
    setEditorOpen(true);
  };

  const openGenerate = (kind?: Kind) => {
    setGenerateKind(kind ?? (tab !== 'overview' ? tab : 'article'));
    setGenerateOpen(true);
  };

  const openVscode = async () => {
    if (!project) return;
    setVscodeLoading(true);
    try {
      const result = await api.openVscode(project.id);
      app.toast({ message: result.message, tone: 'success' });
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      if (apiErr?.code === 'VSCODE_NOT_FOUND' && apiErr.details) {
        const details = apiErr.details as Partial<VscodeFallback>;
        setVscodeFallback({
          projectPath: details.projectPath ?? '',
          manualCommand: details.manualCommand ?? '',
          filesWritten: details.filesWritten ?? [],
        });
        app.toast({ message: '上下文文件已生成，但未找到 VS Code 命令', tone: 'error' });
      } else {
        app.toast({ message: apiErr?.message ?? '打开 VSCode 失败', tone: 'error' });
      }
    } finally {
      setVscodeLoading(false);
    }
  };

  const triggerUpload = (kind: Kind) => {
    pendingUploadKind.current = kind;
    const input = fileInputRef.current;
    if (!input) return;
    input.accept = KIND_MAP[kind].accept ?? '';
    input.value = '';
    input.click();
  };

  const uploadFiles = async (kind: Kind, files: FileList | File[]) => {
    const list = Array.from(files);
    if (!list.length) return;
    setUploading(true);
    setUploadKind(kind);
    let succeeded = 0;
    try {
      for (const file of list) {
        await api.uploadAsset(projectId, kind, file);
        succeeded += 1;
      }
      await refresh();
      await Promise.all([app.refreshProjects(), app.refreshWorkspace()]);
      app.toast({ message: `已上传 ${succeeded} 个${kindLabel(kind)}素材`, tone: 'success' });
    } catch (err) {
      app.toast({
        message: err instanceof ApiError ? err.message : '上传失败',
        tone: 'error',
      });
      if (succeeded) await refresh();
    } finally {
      setUploading(false);
      setUploadKind(null);
      setDragActive(false);
    }
  };

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (isImeComposing(event)) return;
      if (
        (event.metaKey || event.ctrlKey) &&
        event.shiftKey &&
        event.key.toLowerCase() === 'a'
      ) {
        event.preventDefault();
        setAssistantOpen((value) => !value);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  const counts = project?.counts;
  const activeKind = tab !== 'overview' ? KIND_MAP[tab] : null;
  const activeAssets = tab !== 'overview' ? project?.assets[tab] ?? [] : [];

  useEffect(() => {
    setSelectionMode(false);
    setSelectedIds(new Set());
  }, [tab, projectId]);

  const toggleSelected = (id: string) => {
    setSelectedIds((current) => {
      const next = new Set(current);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const selectAll = () => {
    setSelectedIds(new Set(activeAssets.map((asset) => asset.id)));
  };

  const deleteOne = (asset: Asset) => {
    setDeleteTargets([asset]);
  };

  const deleteSelected = () => {
    setDeleteTargets(activeAssets.filter((asset) => selectedIds.has(asset.id)));
  };

  const confirmDelete = async () => {
    if (!deleteTargets.length) return;
    setDeleting(true);
    let succeeded = 0;
    let failed = 0;
    for (const asset of deleteTargets) {
      try {
        await api.deleteAsset(projectId, asset.id);
        succeeded += 1;
      } catch {
        failed += 1;
      }
    }
    await refresh();
    await Promise.all([app.refreshProjects(), app.refreshWorkspace()]);
    setDeleting(false);
    setDeleteTargets([]);
    setSelectedIds((current) => {
      const next = new Set(current);
      deleteTargets.forEach((asset) => next.delete(asset.id));
      return next;
    });
    if (succeeded) {
      app.toast({
        message: failed ? `已删除 ${succeeded} 个，${failed} 个失败` : `已删除 ${succeeded} 个素材`,
        tone: failed ? 'error' : 'success',
      });
    } else {
      app.toast({ message: '删除失败，请重试', tone: 'error' });
    }
  };
  const totalAssets = useMemo(
    () => (counts ? Object.values(counts).reduce((a, b) => a + b, 0) : 0),
    [counts],
  );

  if (loading) {
    return (
      <div class="page">
        <div class="project-loading">
          <Skeleton class="skeleton-title" />
          <Skeleton class="skeleton-line" />
          <Skeleton class="skeleton-block" />
        </div>
      </div>
    );
  }

  if (error || !project) {
    return (
      <div class="page">
        <button class="back-link" onClick={() => navigate('/projects')}>
          <Icon name="arrow-left" size={15} /> 返回项目
        </button>
        <EmptyState
          icon="alert"
          title="项目没有找到"
          description={error || '它可能已经被删除或移动到了回收站。'}
          action={
            <Button variant="primary" onClick={() => navigate('/projects')}>
              回到项目列表
            </Button>
          }
        />
      </div>
    );
  }

  const linkedIdeas = app.data.ideas.filter((idea) => project.ideaIds.includes(idea.id));

  return (
    <div class={`page project-page ${assistantOpen ? 'has-assistant' : ''}`}>
      <div class="project-content">
      <button class="back-link" onClick={() => navigate('/projects')}>
        <Icon name="arrow-left" size={15} /> 所有项目
      </button>

      <header class="project-header">
        <div class="project-header-main">
          <div class="project-header-text">
            <p class="eyebrow">
              PROJECT · {formatFullDate(project.createdAt)}
            </p>
            <div class="project-title-row">
              <h1>{project.title}</h1>
              <Badge tone={statusTone(project.status, 'project')}>{statusLabel(project.status, 'project')}</Badge>
            </div>
            <p class="project-brief">{project.brief || '这个项目还没有简介。可以在项目设置里补充，它会成为 AI 生成时的重要上下文。'}</p>
            {project.tags.length ? (
              <div class="project-tags">
                {project.tags.map((tag) => <span class="tag" key={tag}>{tag}</span>)}
              </div>
            ) : null}
            <div class="project-header-meta">
              <span><Icon name="layers" size={14} /> {totalAssets} 个素材</span>
              <span><Icon name="bulb" size={14} /> {linkedIdeas.length} 条灵感</span>
              <span><Icon name="clock" size={14} /> 更新于 {relativeTime(project.updatedAt)}</span>
            </div>
          </div>
          <div class="project-header-actions">
            <Button variant="primary" icon="wand" onClick={() => openGenerate()}>
              生成素材
            </Button>
            <Button
              variant={assistantOpen ? 'primary' : 'secondary'}
              icon="spark"
              onClick={() => setAssistantOpen((value) => !value)}
            >
              {assistantOpen ? '收起助手' : '助手'}
            </Button>
            <Button variant="secondary" icon="code" loading={vscodeLoading} onClick={openVscode}>
              在 VSCode 打开
            </Button>
            <button class="icon-btn icon-btn-soft" title="项目设置" aria-label="项目设置" onClick={() => setSettingsOpen(true)}>
              <Icon name="settings" size={18} />
            </button>
          </div>
        </div>
        {project.coverUrl ? (
          <button class="project-header-cover" onClick={() => {
            const cover = project.assets.illustration?.find((a) => a.fileUrl) ?? project.assets.card?.find((a) => a.fileUrl);
            if (cover) setLightbox(cover);
          }}>
            <img src={project.coverUrl} alt="" />
          </button>
        ) : (
          <div class="project-header-cover project-header-cover-empty" aria-hidden="true">
            <strong>{projectInitials(project.title)}</strong>
            <small>PROJECT · {project.assets && Object.values(project.counts).reduce((a, b) => a + b, 0)} PIECES</small>
          </div>
        )}
      </header>

      <div class="project-tabs" role="tablist">
        <button class={`project-tab ${tab === 'overview' ? 'is-active' : ''}`} onClick={() => setTab('overview')}>
          概览
        </button>
        {KINDS.map((kind) => (
          <button
            key={kind.id}
            class={`project-tab ${tab === kind.id ? 'is-active' : ''}`}
            onClick={() => setTab(kind.id)}
          >
            {kind.label}
            <span>{project.counts[kind.id] ?? 0}</span>
          </button>
        ))}
        <div class="project-tab-spacer" />
        <Button size="sm" variant="ghost" icon="refresh" onClick={async () => {
          const data = await api.rescanProject(project.id).catch(() => null);
          if (data) {
            setProject(data);
            await Promise.all([app.refreshProjects(), app.refreshWorkspace()]);
            app.toast({ message: '已扫描项目目录', tone: 'success' });
          }
        }}>
          刷新文件
        </Button>
      </div>

      <input
        ref={fileInputRef}
        type="file"
        hidden
        multiple
        onChange={(e) => {
          const files = (e.target as HTMLInputElement).files;
          if (files?.length) uploadFiles(pendingUploadKind.current, files);
          (e.target as HTMLInputElement).value = '';
        }}
      />

      {tab === 'overview' ? (
        <div class="project-overview">
          <div class="overview-grid">
            {KINDS.map((kind) => {
              const list = project.assets[kind.id] ?? [];
              return (
                <section class="kind-overview-card" key={kind.id}>
                  <div class="kind-overview-head">
                    <span class="kind-icon" style={{ background: `${kind.accent}14`, color: kind.accent }}>
                      <Icon
                        name={kind.id === 'video' ? 'video' : kind.id === 'illustration' ? 'image' : kind.id === 'card' ? 'cards' : kind.id === 'script' ? 'file-text' : 'file-text'}
                        size={20}
                      />
                    </span>
                    <div>
                      <h3>{kind.label}</h3>
                      <p>{kind.description}</p>
                    </div>
                    <span class="kind-count">{list.length}</span>
                  </div>
                  {list.length ? (
                    <div class="kind-latest">
                      {list.slice(0, 3).map((asset) => (
                        <button key={asset.id} onClick={() => openAsset(asset)}>
                          <span>{asset.title}</span>
                          <small>{relativeTime(asset.updatedAt)}</small>
                        </button>
                      ))}
                    </div>
                  ) : (
                    <p class="kind-empty">还没有{kind.label}素材。</p>
                  )}
                  <div class="kind-overview-actions">
                    <button class="text-link" onClick={() => setTab(kind.id)}>
                      进入管理 <Icon name="arrow-right" size={14} />
                    </button>
                    <button class="text-link" onClick={() => openGenerate(kind.id)}>
                      <Icon name="wand" size={14} /> 生成
                    </button>
                  </div>
                </section>
              );
            })}
          </div>

          <div class="overview-columns">
            <section class="panel">
              <div class="section-head">
                <div>
                  <p class="eyebrow">SOURCES</p>
                  <h2>立项来源</h2>
                </div>
                <span class="muted-text">{linkedIdeas.length} 条</span>
              </div>
              {linkedIdeas.length ? (
                <div class="linked-ideas">
                  {linkedIdeas.map((idea) => (
                    <button class="linked-idea" key={idea.id} onClick={() => navigate(`/ideas?focus=${idea.id}`)}>
                      <span class="linked-idea-mark"><Icon name="bulb" size={15} /></span>
                      <span>
                        <strong>{idea.title}</strong>
                        <small>{idea.excerpt}</small>
                      </span>
                      <Icon name="chevron-right" size={14} />
                    </button>
                  ))}
                </div>
              ) : (
                <p class="muted-text">还没有关联想法，可以在项目设置里添加。</p>
              )}
            </section>

            <section class="panel vscode-panel">
              <div class="section-head">
                <div>
                  <p class="eyebrow">AI WORKSPACE</p>
                  <h2>带上下文进 VSCode</h2>
                </div>
              </div>
              <p class="panel-text">
                自动生成 <code>AGENTS.md</code>、<code>CLAUDE.md</code>、技能与预设副本、提示词快照目录和
                VSCode 配置，外部 AI 打开即可读懂项目。
              </p>
              <div class="vscode-file-list">
                {['AGENTS.md', 'CLAUDE.md', '.vscode/', 'skills/', 'presets/', 'prompts/'].map((item) => (
                  <span key={item}><Icon name="file-text" size={13} /> {item}</span>
                ))}
              </div>
              <div class="panel-actions">
                <Button variant="secondary" icon="code" loading={vscodeLoading} onClick={openVscode}>
                  打开 VSCode
                </Button>
                <Button
                  variant="ghost"
                  icon="folder-open"
                  onClick={async () => {
                    const result = await api.prepareContext(project.id).catch((err) => err as ApiError);
                    if (result instanceof ApiError) {
                      app.toast({ message: result.message, tone: 'error' });
                    } else {
                      app.toast({ message: '上下文文件已刷新', tone: 'success' });
                    }
                  }}
                >
                  仅刷新文件
                </Button>
              </div>
            </section>
          </div>
        </div>
      ) : (
        <section
          class={`kind-view ${dragActive ? 'is-dragging' : ''}`}
          onDragOver={(e) => {
            if (!activeKind?.upload) return;
            e.preventDefault();
            setDragActive(true);
          }}
          onDragLeave={(e) => {
            if (e.currentTarget === e.target) setDragActive(false);
          }}
          onDrop={(e) => {
            if (!activeKind?.upload) return;
            e.preventDefault();
            uploadFiles(activeKind.id, e.dataTransfer?.files ?? []);
          }}
        >
          <div class="kind-toolbar">
            <div class="kind-toolbar-title">
              <span class="kind-icon" style={{ background: `${activeKind!.accent}14`, color: activeKind!.accent }}>
                <Icon name="layers" size={19} />
              </span>
              <div>
                <h2>{activeKind!.label}</h2>
                <p>{activeKind!.description} · {activeAssets.length} 个素材</p>
              </div>
            </div>
            <div class="kind-toolbar-actions">
              {activeAssets.length ? (
                <Button
                  variant={selectionMode ? 'primary' : 'ghost'}
                  icon="check"
                  onClick={() => {
                    setSelectionMode(!selectionMode);
                    if (selectionMode) setSelectedIds(new Set());
                  }}
                >
                  {selectionMode ? '退出批量' : '批量选择'}
                </Button>
              ) : null}
              <Button variant="ghost" icon="refresh" onClick={() => refresh()}>
                刷新
              </Button>
              {activeKind!.id === 'article' || activeKind!.id === 'script' || activeKind!.id === 'video' || activeKind!.id === 'illustration' ? (
                <Button variant="secondary" icon="plus" onClick={() => openCreate(activeKind!.id)}>
                  {activeKind!.id === 'video' ? '新建笔记' : activeKind!.id === 'illustration' ? '新建提示词' : activeKind!.action}
                </Button>
              ) : null}
              {activeKind!.id === 'card' ? (
                <Button variant="primary" icon="spark" onClick={() => setAiCardOpen(true)}>
                  AI 文卡图
                </Button>
              ) : null}
              {activeKind!.upload ? (
                <Button variant="secondary" icon="upload" loading={uploading && uploadKind === activeKind!.id} onClick={() => triggerUpload(activeKind!.id)}>
                  {activeKind!.id === 'video' ? '上传视频' : '上传图片'}
                </Button>
              ) : null}
              <Button variant="primary" icon="wand" onClick={() => openGenerate(activeKind!.id)}>
                生成
              </Button>
            </div>
          </div>

          {selectionMode ? (
            <div class="asset-batch-bar">
              <span>已选 {selectedIds.size} 个</span>
              <button type="button" class="text-link" onClick={selectAll}>全选本页</button>
              {selectedIds.size ? (
                <button type="button" class="text-link" onClick={() => setSelectedIds(new Set())}>清空选择</button>
              ) : null}
              <span class="asset-batch-spacer" />
              <Button
                size="sm"
                variant="danger"
                icon="trash"
                disabled={!selectedIds.size}
                onClick={deleteSelected}
              >
                删除所选
              </Button>
            </div>
          ) : null}

          {activeAssetGrid(
            activeAssets,
            openAsset,
            (asset) => setLightbox(asset),
            {
              selectable: selectionMode,
              selectedIds,
              onSelect: toggleSelected,
              onDelete: deleteOne,
            },
          )}

          {activeKind!.upload ? (
            <div class="drop-hint">
              <Icon name="upload" size={16} />
              也可以直接把{activeKind!.id === 'video' ? '视频' : '图片'}拖到这里上传
            </div>
          ) : null}
          {dragActive ? (
            <div class="drop-overlay">
              <Icon name="upload" size={28} />
              <strong>松开上传{activeKind!.label}</strong>
            </div>
          ) : null}
        </section>
      )}

      <GenerateDialog
        project={project}
        open={generateOpen}
        initialKind={generateKind}
        onClose={() => setGenerateOpen(false)}
        onSaved={async (assetId) => {
          const data = await refresh();
          if (data) {
            const found = KINDS.flatMap((kind) => data.assets[kind.id] ?? []).find((asset) => asset.id === assetId);
            setTab(generateKind);
            if (found) openAsset(found);
          }
          await app.refreshProjects();
        }}
      />

      <AssetEditor
        project={project}
        asset={editorAsset}
        createKind={createKind}
        open={editorOpen}
        onClose={() => setEditorOpen(false)}
        onSaved={async (assetId) => {
          const data = await refresh();
          if (data && assetId) {
            const found = KINDS.flatMap((kind) => data.assets[kind.id] ?? []).find((asset) => asset.id === assetId);
            if (found) setEditorAsset(found);
          }
        }}
      />

      <ProjectSettings
        project={project}
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        onSaved={async (updated) => {
          setProject(updated);
          await app.refreshProjects();
        }}
      />

      <AiCardStudio
        project={project}
        open={aiCardOpen}
        onClose={() => setAiCardOpen(false)}
        onSaved={async (assetId) => {
          const data = await refresh();
          setTab('card');
          if (data) {
            const found = (data.assets.card ?? []).find((asset) => asset.id === assetId);
            if (found) openAsset(found);
          }
        }}
      />

      <ConfirmDialog
        open={deleteTargets.length > 0}
        title={deleteTargets.length > 1 ? `删除 ${deleteTargets.length} 个素材？` : '删除这个素材？'}
        message={
          deleteTargets.length > 1
            ? `${deleteTargets.length} 个素材及其媒体文件会移入 .mediadesk/trash，可在文件系统中手动恢复。`
            : `「${deleteTargets[0]?.title ?? ''}」会移入 .mediadesk/trash，媒体文件一并移动。`
        }
        confirmLabel="移入回收站"
        loading={deleting}
        onClose={() => {
          if (!deleting) setDeleteTargets([]);
        }}
        onConfirm={confirmDelete}
      />

      <Modal
        open={!!vscodeFallback}
        onClose={() => setVscodeFallback(null)}
        title="没有找到 VS Code 命令行"
        size="md"
        footer={
          <Button variant="primary" onClick={() => setVscodeFallback(null)}>
            知道了
          </Button>
        }
      >
        <div class="stack">
          <p>上下文文件已经生成在项目目录里。你可以手动用 VS Code 打开该文件夹，或安装 <code>code</code> 命令后再试。</p>
          {vscodeFallback ? (
            <>
              <div class="copy-box">
                <code>{vscodeFallback.manualCommand}</code>
                <button
                  class="copy-box-btn"
                  onClick={async () => {
                    await copyText(vscodeFallback.manualCommand);
                    app.toast({ message: '命令已复制', tone: 'success' });
                  }}
                >
                  <Icon name="copy" size={14} /> 复制
                </button>
              </div>
              <p class="muted-text">已生成 {vscodeFallback.filesWritten.length} 个上下文文件，项目路径：{vscodeFallback.projectPath}</p>
            </>
          ) : null}
        </div>
      </Modal>

      {lightbox ? (
        <div class="lightbox" onClick={() => setLightbox(null)}>
          <button class="lightbox-close" aria-label="关闭预览" onClick={() => setLightbox(null)}>
            <Icon name="close" size={22} />
          </button>
          <img src={lightbox.fileUrl} alt={lightbox.title} onClick={(e) => e.stopPropagation()} />
          <div class="lightbox-caption" onClick={(e) => e.stopPropagation()}>
            <span>{lightbox.title}</span>
            <button
              class="text-link"
              onClick={() => {
                downloadUrl(lightbox.fileUrl!, lightbox.file ?? `${lightbox.title}.png`);
              }}
            >
              <Icon name="download" size={14} /> 下载
            </button>
          </div>
        </div>
      ) : null}
      </div>

      {!assistantOpen ? (
        <button
          type="button"
          class="assistant-fab"
          aria-label="打开项目助手"
          title="打开项目助手（⌘⇧A / Ctrl+Shift+A）"
          onClick={() => setAssistantOpen(true)}
        >
          <Icon name="spark" size={17} />
          <span>助手</span>
        </button>
      ) : null}

      {assistantOpen ? (
        <AssistantPanel
          project={project}
          open={assistantOpen}
          onClose={() => setAssistantOpen(false)}
          onChanged={async () => {
            await refresh();
            await app.refreshProjects();
          }}
        />
      ) : null}
    </div>
  );
}

function activeAssetGrid(
  assets: Asset[],
  onOpen: (asset: Asset) => void,
  onPreview: (asset: Asset) => void,
  selection?: {
    selectable: boolean;
    selectedIds: Set<string>;
    onSelect: (id: string) => void;
    onDelete: (asset: Asset) => void;
  },
) {
  if (!assets.length) {
    return (
      <EmptyState
        icon="layers"
        title="这一类还没有素材"
        description="新建、上传，或让大模型根据项目上下文生成一份初稿。"
      />
    );
  }
  return (
    <div class="asset-grid">
      {assets.map((asset) => (
        <AssetCard
          key={asset.id}
          asset={asset}
          onOpen={() => onOpen(asset)}
          onPreview={asset.fileUrl ? () => onPreview(asset) : undefined}
          selectable={selection?.selectable}
          selected={selection?.selectedIds.has(asset.id)}
          onSelect={() => selection?.onSelect(asset.id)}
          onDelete={() => selection?.onDelete(asset)}
        />
      ))}
    </div>
  );
}
