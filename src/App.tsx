import { useCallback, useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api, ApiError } from './lib/api';
import { AppContext, type AppData, type ToastInput } from './lib/app-context';
import type { ThemeMode } from './lib/types';
import { useRoute } from './lib/router';
import { applyTheme, persistTheme, readStoredTheme } from './lib/theme';
import { Sidebar } from './components/Sidebar';
import { ToastHost, type ToastItem } from './components/ToastHost';
import { QuickCapture } from './components/QuickCapture';
import { CommandPalette } from './components/CommandPalette';
import { Button, EmptyState } from './components/ui';
import { Icon } from './components/Icon';
import { DashboardPage } from './pages/DashboardPage';
import { IdeasPage } from './pages/IdeasPage';
import { ProjectsPage } from './pages/ProjectsPage';
import { ProjectPage } from './pages/ProjectPage';
import { LibraryPage } from './pages/LibraryPage';
import { SettingsPage } from './pages/SettingsPage';
import { navigate } from './lib/router';
import { isImeComposing } from './lib/ime';

export function App() {
  const route = useRoute();
  const [data, setData] = useState<AppData | null>(null);
  const [loading, setLoading] = useState(true);
  const [loadError, setLoadError] = useState('');
  const [theme, setThemeState] = useState<ThemeMode>(readStoredTheme);
  const [toasts, setToasts] = useState<ToastItem[]>([]);
  const [quickOpen, setQuickOpen] = useState(false);
  const [paletteOpen, setPaletteOpen] = useState(false);
  const [mobileSidebar, setMobileSidebar] = useState(false);
  const toastId = useRef(0);

  const loadAll = useCallback(async () => {
    setLoadError('');
    try {
      const [workspace, config, ideas, projects, skills, presets] = await Promise.all([
        api.getWorkspace(),
        api.getConfig(),
        api.listIdeas(),
        api.listProjects(),
        api.listSkills(),
        api.listPresets(),
      ]);
      setData({ workspace, config, ideas, projects, skills, presets });
    } catch (err) {
      setLoadError(err instanceof ApiError ? err.message : '无法连接本地服务');
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    loadAll();
  }, [loadAll]);

  useEffect(() => {
    applyTheme(theme);
    if (theme !== 'system') return;
    const media = window.matchMedia('(prefers-color-scheme: dark)');
    const listener = () => applyTheme('system');
    media.addEventListener('change', listener);
    return () => media.removeEventListener('change', listener);
  }, [theme]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (isImeComposing(event)) return;
      const meta = event.metaKey || event.ctrlKey;
      if (!meta) return;
      if (event.key.toLowerCase() === 'k') {
        event.preventDefault();
        setPaletteOpen(true);
      } else if (event.key.toLowerCase() === 'n') {
        event.preventDefault();
        setQuickOpen(true);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, []);

  useEffect(() => {
    setMobileSidebar(false);
    window.scrollTo({ top: 0, behavior: 'instant' as ScrollBehavior });
  }, [route.path]);

  const toast = useCallback((input: ToastInput | string) => {
    const value = typeof input === 'string' ? { message: input } : input;
    const id = ++toastId.current;
    const item: ToastItem = {
      id,
      message: value.message,
      tone: value.tone ?? 'default',
    };
    setToasts((prev) => [...prev.slice(-2), item]);
    window.setTimeout(() => {
      setToasts((prev) => prev.filter((t) => t.id !== id));
    }, value.duration ?? 3400);
  }, []);

  const refreshIdeas = useCallback(async () => {
    const ideas = await api.listIdeas();
    setData((prev) => (prev ? { ...prev, ideas } : prev));
  }, []);

  const refreshProjects = useCallback(async () => {
    const projects = await api.listProjects();
    setData((prev) => (prev ? { ...prev, projects } : prev));
  }, []);

  const refreshLibrary = useCallback(async () => {
    const [skills, presets] = await Promise.all([api.listSkills(), api.listPresets()]);
    setData((prev) => (prev ? { ...prev, skills, presets } : prev));
  }, []);

  const refreshWorkspace = useCallback(async () => {
    const workspace = await api.getWorkspace();
    setData((prev) => (prev ? { ...prev, workspace } : prev));
  }, []);

  const refreshConfig = useCallback(async () => {
    const config = await api.getConfig();
    setData((prev) => (prev ? { ...prev, config } : prev));
  }, []);

  const refreshAll = useCallback(async () => {
    const [workspace, config, ideas, projects, skills, presets] = await Promise.all([
      api.getWorkspace(),
      api.getConfig(),
      api.listIdeas(),
      api.listProjects(),
      api.listSkills(),
      api.listPresets(),
    ]);
    setData({ workspace, config, ideas, projects, skills, presets });
  }, []);

  const setTheme = useCallback((next: ThemeMode) => {
    setThemeState(next);
    persistTheme(next);
  }, []);

  const contextValue = useMemo(() => {
    if (!data) return null;
    return {
      data,
      loading,
      refreshAll,
      refreshIdeas,
      refreshProjects,
      refreshLibrary,
      refreshWorkspace,
      refreshConfig,
      toast,
      theme,
      setTheme,
      openQuickCapture: () => setQuickOpen(true),
      openCommandPalette: () => setPaletteOpen(true),
    };
  }, [
    data,
    loading,
    refreshAll,
    refreshIdeas,
    refreshProjects,
    refreshLibrary,
    refreshWorkspace,
    refreshConfig,
    toast,
    theme,
    setTheme,
  ]);

  if (loading) {
    return (
      <div class="boot-screen">
        <div class="boot-mark">媒</div>
        <h1>媒材工坊</h1>
        <p>正在整理你的工作台…</p>
        <span class="loading-bar"><span /></span>
      </div>
    );
  }

  if (loadError || !data || !contextValue) {
    return (
      <div class="boot-screen boot-error">
        <div class="boot-mark">媒</div>
        <h1>无法打开工作台</h1>
        <p>{loadError || '本地服务暂不可用。'}</p>
        <div class="boot-actions">
          <Button variant="primary" icon="refresh" onClick={() => {
            setLoading(true);
            loadAll();
          }}>
            重试
          </Button>
          <Button variant="ghost" onClick={() => window.location.reload()}>
            刷新页面
          </Button>
        </div>
        <small>如果是桌面版，请确认应用已完整启动；浏览器版请确认本地服务仍在运行。</small>
      </div>
    );
  }

  return (
    <AppContext.Provider value={contextValue}>
      <div class="app-shell">
        <Sidebar
          active={route.name}
          ideaCount={data.ideas.length}
          projectCount={data.projects.length}
          workspacePath={data.workspace.path}
          mobileOpen={mobileSidebar}
          onClose={() => setMobileSidebar(false)}
          onQuickCapture={() => setQuickOpen(true)}
          onCommandPalette={() => setPaletteOpen(true)}
        />
        <div class="mobile-topbar">
          <button class="icon-btn icon-btn-soft" aria-label="打开菜单" onClick={() => setMobileSidebar(true)}>
            <Icon name="menu" size={19} />
          </button>
          <button class="mobile-brand" onClick={() => navigate('/')}>
            <span>媒</span>
            媒材工坊
          </button>
          <button class="icon-btn icon-btn-soft" aria-label="快速搜索" onClick={() => setPaletteOpen(true)}>
            <Icon name="search" size={18} />
          </button>
        </div>
        <main class="app-main">
          {route.name === 'dashboard' ? <DashboardPage /> : null}
          {route.name === 'ideas' ? <IdeasPage /> : null}
          {route.name === 'projects' ? <ProjectsPage /> : null}
          {route.name === 'project' && route.id ? <ProjectPage projectId={route.id} /> : null}
          {route.name === 'library' ? <LibraryPage /> : null}
          {route.name === 'settings' ? <SettingsPage /> : null}
          {route.name === 'notfound' ? (
            <div class="page">
              <EmptyState
                icon="alert"
                title="页面走丢了"
                description="这个地址没有对应的页面。"
                action={<Button variant="primary" onClick={() => navigate('/')}>回到工作台</Button>}
              />
            </div>
          ) : null}
        </main>
      </div>
      <QuickCapture open={quickOpen} onClose={() => setQuickOpen(false)} />
      <CommandPalette open={paletteOpen} onClose={() => setPaletteOpen(false)} />
      <ToastHost
        toasts={toasts}
        onDismiss={(id) => setToasts((prev) => prev.filter((toastItem) => toastItem.id !== id))}
      />
    </AppContext.Provider>
  );
}
