import { useEffect, useMemo, useState } from 'preact/hooks';
import { useApp } from '../lib/app-context';
import { IdeaEditor } from '../components/IdeaEditor';
import { ProjectCreate } from '../components/ProjectCreate';
import { Icon } from '../components/Icon';
import { Badge, Button, EmptyState } from '../components/ui';
import { navigate, useQueryParam } from '../lib/router';
import type { Idea, IdeaStatus } from '../lib/types';
import { IDEA_STATUS } from '../lib/types';
import { relativeTime, statusLabel, statusTone } from '../lib/format';

type Filter = 'all' | IdeaStatus;

export function IdeasPage() {
  const app = useApp();
  const focus = useQueryParam('focus');
  const [filter, setFilter] = useState<Filter>('all');
  const [query, setQuery] = useState('');
  const [tag, setTag] = useState<string | null>(null);
  const [editing, setEditing] = useState<Idea | null>(null);
  const [editorOpen, setEditorOpen] = useState(false);
  const [promoting, setPromoting] = useState<Idea | null>(null);

  useEffect(() => {
    if (!focus) return;
    const idea = app.data.ideas.find((item) => item.id === focus);
    if (idea) {
      setEditing(idea);
      setEditorOpen(true);
    }
  }, [focus, app.data.ideas]);

  const closeEditor = () => {
    setEditorOpen(false);
    setEditing(null);
    if (focus) navigate('/ideas', { replace: true });
  };

  const closePromote = () => {
    setPromoting(null);
  };

  const tags = useMemo(() => {
    const set = new Set<string>();
    app.data.ideas.forEach((idea) => idea.tags.forEach((t) => set.add(t)));
    return [...set].slice(0, 18);
  }, [app.data.ideas]);

  const filtered = useMemo(() => {
    const needle = query.trim().toLowerCase();
    return app.data.ideas.filter((idea) => {
      if (filter !== 'all' && idea.status !== filter) return false;
      if (tag && !idea.tags.includes(tag)) return false;
      if (!needle) return true;
      return (
        idea.title.toLowerCase().includes(needle) ||
        idea.content.toLowerCase().includes(needle) ||
        idea.tags.some((t) => t.toLowerCase().includes(needle))
      );
    });
  }, [app.data.ideas, filter, query, tag]);

  const counts = useMemo(
    () => ({
      all: app.data.ideas.length,
      inbox: app.data.ideas.filter((i) => i.status === 'inbox').length,
      promoted: app.data.ideas.filter((i) => i.status === 'promoted').length,
      archived: app.data.ideas.filter((i) => i.status === 'archived').length,
    }),
    [app.data.ideas],
  );

  return (
    <div class="page ideas-page">
      <header class="page-heading">
        <div>
          <p class="eyebrow">INBOX · 灵感收件箱</p>
          <h1>所有想法，先收进来。</h1>
          <p class="page-subtitle">不用分类，不用完整。记录下来，再决定哪些值得立项。</p>
        </div>
        <div class="heading-actions">
          <Button variant="primary" icon="pen" onClick={app.openQuickCapture}>
            速记想法
          </Button>
        </div>
      </header>

      <div class="toolbar ideas-toolbar">
        <div class="filter-tabs">
          {([
            ['all', '全部'],
            ['inbox', '待立项'],
            ['promoted', '已立项'],
            ['archived', '已归档'],
          ] as [Filter, string][]).map(([value, label]) => (
            <button
              key={value}
              class={`filter-tab ${filter === value ? 'is-active' : ''}`}
              onClick={() => setFilter(value)}
            >
              {label}
              <span>{counts[value]}</span>
            </button>
          ))}
        </div>
        <div class="search-field">
          <Icon name="search" size={16} />
          <input
            value={query}
            placeholder="搜索想法…"
            onInput={(e) => setQuery((e.target as HTMLInputElement).value)}
          />
          {query ? (
            <button class="search-clear" aria-label="清除搜索" onClick={() => setQuery('')}>
              <Icon name="close" size={13} />
            </button>
          ) : null}
        </div>
      </div>

      {tags.length ? (
        <div class="tag-filter-row">
          <button class={`tag-filter ${tag === null ? 'is-active' : ''}`} onClick={() => setTag(null)}>
            全部标签
          </button>
          {tags.map((t) => (
            <button
              key={t}
              class={`tag-filter ${tag === t ? 'is-active' : ''}`}
              onClick={() => setTag(tag === t ? null : t)}
            >
              #{t}
            </button>
          ))}
        </div>
      ) : null}

      {filtered.length ? (
        <div class="ideas-masonry">
          {filtered.map((idea) => (
            <article class="idea-card" key={idea.id}>
              <div class="idea-card-top">
                <Badge tone={statusTone(idea.status, 'idea')}>{statusLabel(idea.status, 'idea')}</Badge>
                <span class="idea-time">{relativeTime(idea.updatedAt)}</span>
              </div>
              <button class="idea-card-main" onClick={() => navigate(`/ideas?focus=${idea.id}`)}>
                <h3>{idea.title}</h3>
                <p>{idea.excerpt || '（还没有正文）'}</p>
              </button>
              {idea.tags.length ? (
                <div class="idea-tags">
                  {idea.tags.map((t) => (
                    <button key={t} onClick={() => setTag(t)}>
                      #{t}
                    </button>
                  ))}
                </div>
              ) : null}
              <div class="idea-card-actions">
                <button class="text-link" onClick={() => navigate(`/ideas?focus=${idea.id}`)}>
                  <Icon name="edit" size={14} /> 编辑
                </button>
                {idea.projectId ? (
                  <button class="text-link" onClick={() => navigate(`/projects/${idea.projectId}`)}>
                    <Icon name="arrow-right" size={14} /> 查看项目
                  </button>
                ) : (
                  <button class="text-link accent-link" onClick={() => setPromoting(idea)}>
                    <Icon name="spark" size={14} /> 立项
                  </button>
                )}
              </div>
            </article>
          ))}
        </div>
      ) : (
        <EmptyState
          icon="inbox"
          title={query || tag || filter !== 'all' ? '没有匹配的想法' : '灵感箱还是空的'}
          description={
            query || tag || filter !== 'all'
              ? '换个筛选条件，或者清空搜索。'
              : '先记下此刻脑海里的那一句话。'
          }
          action={
            query || tag || filter !== 'all' ? (
              <Button
                variant="secondary"
                onClick={() => {
                  setQuery('');
                  setTag(null);
                  setFilter('all');
                }}
              >
                清除筛选
              </Button>
            ) : (
              <Button variant="primary" icon="pen" onClick={app.openQuickCapture}>
                速记第一条想法
              </Button>
            )
          }
        />
      )}

      <IdeaEditor idea={editing} open={editorOpen} onClose={closeEditor} />
      <ProjectCreate
        open={!!promoting}
        initialIdeas={promoting ? [promoting] : []}
        onClose={closePromote}
      />
    </div>
  );
}
