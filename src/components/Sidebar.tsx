import { Icon } from './Icon';
import type { IconName } from './Icon';
import type { RouteName } from '../lib/router';
import { navigate } from '../lib/router';

interface NavItem {
  name: RouteName;
  label: string;
  icon: IconName;
  count?: number;
}

export function Sidebar({
  active,
  ideaCount,
  projectCount,
  workspacePath,
  mobileOpen,
  onClose,
  onQuickCapture,
  onCommandPalette,
}: {
  active: RouteName;
  ideaCount: number;
  projectCount: number;
  workspacePath: string;
  mobileOpen: boolean;
  onClose: () => void;
  onQuickCapture: () => void;
  onCommandPalette: () => void;
}) {
  const items: NavItem[] = [
    { name: 'dashboard', label: '工作台', icon: 'spark' },
    { name: 'ideas', label: '灵感', icon: 'inbox', count: ideaCount },
    { name: 'projects', label: '项目', icon: 'folder', count: projectCount },
    { name: 'library', label: '技能与预设', icon: 'layers' },
    { name: 'settings', label: '设置', icon: 'settings' },
  ];

  const go = (name: RouteName) => {
    navigate(name === 'dashboard' ? '/' : `/${name}`);
    onClose();
  };

  const workspaceName = workspacePath.split(/[\\/]/).filter(Boolean).pop() ?? workspacePath;

  return (
    <>
      <div class={`sidebar-scrim ${mobileOpen ? 'is-open' : ''}`} onClick={onClose} />
      <aside class={`sidebar ${mobileOpen ? 'is-open' : ''}`}>
        <div class="sidebar-brand">
          <button class="brand" onClick={() => go('dashboard')} aria-label="返回工作台">
            <span class="brand-mark">媒</span>
            <span class="brand-text">
              <strong>媒材工坊</strong>
              <small>MEDIA DESK</small>
            </span>
          </button>
          <button class="sidebar-close-mobile" aria-label="关闭菜单" onClick={onClose}>
            <Icon name="close" size={18} />
          </button>
        </div>

        <button class="quick-capture-trigger" onClick={onQuickCapture}>
          <Icon name="pen" size={17} />
          <span>速记一个想法</span>
          <kbd>⌘N</kbd>
        </button>

        <button class="command-trigger" onClick={onCommandPalette}>
          <span class="command-trigger-left">
            <Icon name="search" size={15} />
            搜索全部
          </span>
          <kbd>⌘K</kbd>
        </button>

        <nav class="sidebar-nav" aria-label="主导航">
          <span class="nav-caption">工作空间</span>
          {items.map((item) => (
            <button
              key={item.name}
              class={`nav-item ${active === item.name || (item.name === 'projects' && active === 'project') ? 'is-active' : ''}`}
              onClick={() => go(item.name)}
            >
              <Icon name={item.icon} size={18} />
              <span class="nav-label">{item.label}</span>
              {typeof item.count === 'number' && item.count > 0 ? (
                <span class="nav-count">{item.count}</span>
              ) : null}
            </button>
          ))}
        </nav>

        <div class="sidebar-spacer" />

        <div class="sidebar-foot">
          <button class="workspace-card" onClick={() => go('settings')} title={workspacePath}>
            <span class="workspace-card-icon">
              <Icon name="folder-open" size={16} />
            </span>
            <span class="workspace-card-body">
              <small>当前工作区</small>
              <strong>{workspaceName || '未选择'}</strong>
            </span>
            <Icon name="chevron-right" size={14} class="workspace-chevron" />
          </button>
          <p class="sidebar-note">所有内容保存在本地 Markdown 文件中。</p>
        </div>
      </aside>
    </>
  );
}
