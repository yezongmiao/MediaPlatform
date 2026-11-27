import { useEffect, useRef, useState } from 'preact/hooks';
import { Button } from './ui';
import { Icon } from './Icon';
import type { IconName } from './Icon';

interface OpenProjectMenuProps {
  onVscode: () => void;
  onDsh: () => void;
  onFolder: () => void;
  vscodeLoading?: boolean;
  dshLoading?: boolean;
  folderLoading?: boolean;
}

interface MenuAction {
  label: string;
  description: string;
  icon: IconName;
  run: () => void;
}

export function OpenProjectMenu({
  onVscode,
  onDsh,
  onFolder,
  vscodeLoading = false,
  dshLoading = false,
  folderLoading = false,
}: OpenProjectMenuProps) {
  const [open, setOpen] = useState(false);
  const rootRef = useRef<HTMLDivElement>(null);
  const menuRef = useRef<HTMLDivElement>(null);
  const busy = vscodeLoading || dshLoading || folderLoading;

  useEffect(() => {
    if (!open) return;
    const onPointerDown = (event: PointerEvent) => {
      if (!rootRef.current?.contains(event.target as Node)) setOpen(false);
    };
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      event.preventDefault();
      setOpen(false);
      rootRef.current?.querySelector<HTMLElement>('.open-project-menu-trigger')?.focus();
    };
    document.addEventListener('pointerdown', onPointerDown, true);
    document.addEventListener('keydown', onKeyDown);
    window.setTimeout(() => {
      menuRef.current?.querySelector<HTMLElement>('[role="menuitem"]')?.focus();
    }, 0);
    return () => {
      document.removeEventListener('pointerdown', onPointerDown, true);
      document.removeEventListener('keydown', onKeyDown);
    };
  }, [open]);

  const choose = (action: () => void) => {
    setOpen(false);
    action();
  };

  const actions: MenuAction[] = [
    {
      label: '在 VS Code 打开',
      description: '刷新 AI 上下文并启动编辑器',
      icon: 'code',
      run: onVscode,
    },
    {
      label: '在 DeepSeek Harness 打开',
      description: '刷新上下文并唤起桌面窗口',
      icon: 'spark',
      run: onDsh,
    },
    {
      label: '在文件管理器中显示',
      description: '打开项目所在文件夹',
      icon: 'folder-open',
      run: onFolder,
    },
  ];

  const onMenuKeyDown = (event: KeyboardEvent) => {
    if (!['ArrowDown', 'ArrowUp', 'Home', 'End'].includes(event.key)) return;
    const items = [...(menuRef.current?.querySelectorAll<HTMLElement>('[role="menuitem"]') ?? [])];
    if (!items.length) return;
    event.preventDefault();
    const current = items.indexOf(document.activeElement as HTMLElement);
    let next = 0;
    if (event.key === 'End') next = items.length - 1;
    else if (event.key === 'ArrowUp') next = current <= 0 ? items.length - 1 : current - 1;
    else if (event.key === 'ArrowDown') next = current < 0 || current >= items.length - 1 ? 0 : current + 1;
    items[next].focus();
  };

  return (
    <div class="open-project-menu" ref={rootRef}>
      <Button
        class={`open-project-menu-trigger ${open ? 'is-open' : ''}`}
        variant="secondary"
        icon="external"
        iconAfter="chevron-down"
        loading={busy}
        aria-haspopup="menu"
        aria-expanded={open}
        onClick={() => setOpen((value) => !value)}
      >
        打开
      </Button>
      {open ? (
        <div class="open-project-menu-popover" role="menu" ref={menuRef} onKeyDown={onMenuKeyDown}>
          {actions.map((action) => (
            <button
              type="button"
              class="open-project-menu-item"
              role="menuitem"
              key={action.label}
              onClick={() => choose(action.run)}
            >
              <span class="open-project-menu-icon"><Icon name={action.icon} size={17} /></span>
              <span class="open-project-menu-copy">
                <strong>{action.label}</strong>
                <small>{action.description}</small>
              </span>
            </button>
          ))}
        </div>
      ) : null}
    </div>
  );
}
