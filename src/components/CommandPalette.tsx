import { useEffect, useMemo, useRef, useState } from 'preact/hooks';
import { api } from '../lib/api';
import { useApp } from '../lib/app-context';
import { Icon } from './Icon';
import type { IconName } from './Icon';
import { Modal } from './ui';
import { navigate } from '../lib/router';
import type { SearchResult } from '../lib/types';
import { kindLabel } from '../lib/format';

interface PaletteItem {
  id: string;
  label: string;
  hint?: string;
  icon: IconName;
  group: string;
  run: () => void;
}

export function CommandPalette({ open, onClose }: { open: boolean; onClose: () => void }) {
  const app = useApp();
  const [query, setQuery] = useState('');
  const [result, setResult] = useState<SearchResult | null>(null);
  const [loading, setLoading] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const inputRef = useRef<HTMLInputElement>(null);
  const requestId = useRef(0);

  useEffect(() => {
    if (open) {
      setQuery('');
      setResult(null);
      setActiveIndex(0);
      window.setTimeout(() => inputRef.current?.focus(), 30);
    }
  }, [open]);

  useEffect(() => {
    if (!open) return;
    const q = query.trim();
    if (!q) {
      setResult(null);
      setLoading(false);
      return;
    }
    const id = ++requestId.current;
    setLoading(true);
    const timer = window.setTimeout(async () => {
      try {
        const data = await api.search(q);
        if (id === requestId.current) setResult(data);
      } catch {
        if (id === requestId.current) setResult(null);
      } finally {
        if (id === requestId.current) setLoading(false);
      }
    }, 220);
    return () => window.clearTimeout(timer);
  }, [query, open]);

  const items = useMemo<PaletteItem[]>(() => {
    const actions: PaletteItem[] = [
      {
        id: 'action-capture',
        label: '速记一个想法',
        hint: '⌘N',
        icon: 'pen',
        group: '快捷操作',
        run: () => {
          onClose();
          app.openQuickCapture();
        },
      },
      {
        id: 'action-project',
        label: '新建项目',
        icon: 'plus',
        group: '快捷操作',
        run: () => {
          onClose();
          navigate('/projects?new=1');
        },
      },
      {
        id: 'action-library',
        label: '管理技能与预设',
        icon: 'layers',
        group: '前往',
        run: () => {
          onClose();
          navigate('/library');
        },
      },
      {
        id: 'action-theme',
        label: app.theme === 'dark' ? '切换到浅色主题' : '切换到深色主题',
        icon: app.theme === 'dark' ? 'sun' : 'moon',
        group: '外观',
        run: () => {
          app.setTheme(app.theme === 'dark' ? 'light' : 'dark');
          onClose();
        },
      },
    ];
    const needle = query.trim().toLowerCase();
    const filteredActions = needle
      ? actions.filter((a) => a.label.toLowerCase().includes(needle))
      : actions;
    const resultItems: PaletteItem[] = [];
    if (result) {
      for (const idea of result.ideas) {
        resultItems.push({
          id: `idea-${idea.id}`,
          label: idea.title,
          hint: idea.excerpt,
          icon: 'bulb',
          group: '灵感',
          run: () => {
            onClose();
            navigate(`/ideas?focus=${idea.id}`);
          },
        });
      }
      for (const project of result.projects) {
        resultItems.push({
          id: `project-${project.id}`,
          label: project.title,
          hint: project.briefExcerpt,
          icon: 'folder',
          group: '项目',
          run: () => {
            onClose();
            navigate(`/projects/${project.id}`);
          },
        });
      }
      for (const item of result.assets) {
        resultItems.push({
          id: `asset-${item.asset.id}`,
          label: item.asset.title,
          hint: `${item.projectTitle} · ${kindLabel(item.asset.kind)}`,
          icon: item.asset.kind === 'video' ? 'video' : item.asset.kind === 'illustration' || item.asset.kind === 'card' ? 'image' : 'file-text',
          group: '素材',
          run: () => {
            onClose();
            navigate(`/projects/${item.projectId}?asset=${item.asset.id}`);
          },
        });
      }
    }
    return [...filteredActions, ...resultItems];
  }, [query, result, app.theme, app, onClose]);

  useEffect(() => {
    setActiveIndex(0);
  }, [query, result]);

  const groups = useMemo(() => {
    const map = new Map<string, { item: PaletteItem; index: number }[]>();
    items.forEach((item, index) => {
      const list = map.get(item.group) ?? [];
      list.push({ item, index });
      map.set(item.group, list);
    });
    return [...map.entries()];
  }, [items]);

  const runIndex = (index: number) => {
    const item = items[index];
    if (item) item.run();
  };

  return (
    <Modal open={open} onClose={onClose} size="lg" hideClose class="palette-modal">
      <div class="palette-search">
        <Icon name="search" size={19} />
        <input
          ref={inputRef}
          value={query}
          placeholder="搜索想法、项目、素材，或输入命令…"
          onInput={(e) => setQuery((e.target as HTMLInputElement).value)}
          onKeyDown={(e) => {
            if (e.key === 'ArrowDown') {
              e.preventDefault();
              setActiveIndex((i) => (items.length ? (i + 1) % items.length : 0));
            } else if (e.key === 'ArrowUp') {
              e.preventDefault();
              setActiveIndex((i) => (items.length ? (i - 1 + items.length) % items.length : 0));
            } else if (e.key === 'Enter') {
              e.preventDefault();
              runIndex(activeIndex);
            }
          }}
        />
        {loading ? <span class="spinner" /> : <kbd class="palette-esc">ESC</kbd>}
      </div>
      <div class="palette-results">
        {groups.length === 0 ? (
          <div class="palette-empty">
            <Icon name="search" size={22} />
            <p>{query ? '没有找到匹配内容' : '输入关键词开始搜索'}</p>
          </div>
        ) : (
          groups.map(([group, entries]) => (
            <div class="palette-group" key={group}>
              <span class="palette-group-label">{group}</span>
              {entries.map(({ item, index }) => (
                <button
                  key={item.id}
                  class={`palette-item ${index === activeIndex ? 'is-active' : ''}`}
                  onMouseEnter={() => setActiveIndex(index)}
                  onClick={() => runIndex(index)}
                >
                  <span class="palette-item-icon">
                    <Icon name={item.icon} size={17} />
                  </span>
                  <span class="palette-item-text">
                    <strong>{item.label}</strong>
                    {item.hint ? <small>{item.hint}</small> : null}
                  </span>
                  {item.hint === '⌘N' ? <kbd>⌘N</kbd> : null}
                </button>
              ))}
            </div>
          ))
        )}
      </div>
      <div class="palette-foot">
        <span><kbd>↑</kbd><kbd>↓</kbd> 选择</span>
        <span><kbd>↵</kbd> 打开</span>
        <span><kbd>ESC</kbd> 关闭</span>
      </div>
    </Modal>
  );
}
