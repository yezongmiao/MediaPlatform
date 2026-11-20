import { useEffect, useMemo, useState } from 'preact/hooks';
import { useApp } from '../lib/app-context';
import { ProjectCard } from '../components/ProjectCard';
import { ProjectCreate } from '../components/ProjectCreate';
import { Button, EmptyState } from '../components/ui';
import { Icon } from '../components/Icon';
import { navigate, useQueryParam } from '../lib/router';
import type { ProjectStatus } from '../lib/types';

export function ProjectsPage() {
  const app = useApp();
  const createParam = useQueryParam('new');
  const [creating, setCreating] = useState(false);
  const [query, setQuery] = useState('');
  const [status, setStatus] = useState<'all' | ProjectStatus>('all');

  useEffect(() => {
    if (createParam === '1') setCreating(true);
  }, [createParam]);

  const closeCreate = () => {
    setCreating(false);
    if (createParam === '1') navigate('/projects', { replace: true });
  };

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return app.data.projects.filter((project) => {
      if (status !== 'all' && project.status !== status) return false;
      if (!needle) return true;
      return (
        project.title.toLowerCase().includes(needle) ||
        project.briefExcerpt.toLowerCase().includes(needle) ||
        project.tags.some((t) => t.toLowerCase().includes(needle))
      );
    });
  }, [app.data.projects, query, status]);

  return (
    <div class="page projects-page">
      <header class="page-heading">
        <div>
          <p class="eyebrow">PROJECTS · 项目</p>
          <h1>一条内容线，一个项目。</h1>
          <p class="page-subtitle">文章、脚本、插图、文卡图、视频，各自归档，又彼此关联。</p>
        </div>
        <div class="heading-actions">
          <Button variant="primary" icon="plus" onClick={() => setCreating(true)}>
            新建项目
          </Button>
        </div>
      </header>

      <div class="toolbar">
        <div class="filter-tabs">
          {([
            ['all', '全部'],
            ['active', '进行中'],
            ['paused', '已暂停'],
            ['done', '已完成'],
            ['archived', '已归档'],
          ] as ['all' | ProjectStatus, string][]).map(([value, label]) => (
            <button
              key={value}
              class={`filter-tab ${status === value ? 'is-active' : ''}`}
              onClick={() => setStatus(value)}
            >
              {label}
            </button>
          ))}
        </div>
        <div class="search-field">
          <Icon name="search" size={16} />
          <input
            value={query}
            placeholder="搜索项目…"
            onInput={(e) => setQuery((e.target as HTMLInputElement).value)}
          />
          {query ? (
            <button class="search-clear" aria-label="清除搜索" onClick={() => setQuery('')}>
              <Icon name="close" size={13} />
            </button>
          ) : null}
        </div>
      </div>

      {filtered.length ? (
        <div class="project-grid">
          {filtered.map((project) => (
            <ProjectCard project={project} key={project.id} />
          ))}
        </div>
      ) : (
        <EmptyState
          icon="folder"
          title={query || status !== 'all' ? '没有匹配的项目' : '还没有项目'}
          description={
            query || status !== 'all'
              ? '换个关键词或状态试试。'
              : '从灵感立项，或直接建立一个空项目。'
          }
          action={
            query || status !== 'all' ? (
              <Button
                variant="secondary"
                onClick={() => {
                  setQuery('');
                  setStatus('all');
                }}
              >
                清除筛选
              </Button>
            ) : (
              <Button variant="primary" icon="plus" onClick={() => setCreating(true)}>
                新建项目
              </Button>
            )
          }
        />
      )}

      <ProjectCreate open={creating} onClose={closeCreate} />
    </div>
  );
}
