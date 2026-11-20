import { useMemo, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import { Icon } from '../components/Icon';
import { ProjectCard } from '../components/ProjectCard';
import { Button, EmptyState } from '../components/ui';
import { navigate } from '../lib/router';
import { excerpt, formatBytes, greeting, relativeTime, todayLabel } from '../lib/format';

export function DashboardPage() {
  const app = useApp();
  const [draft, setDraft] = useState('');
  const [saving, setSaving] = useState(false);
  const [error, setError] = useState('');

  const recentProjects = app.data.projects.slice(0, 3);
  const recentIdeas = app.data.ideas
    .filter((idea) => idea.status !== 'archived')
    .slice(0, 5);
  const totalAssets = Object.values(app.data.projects).reduce(
    (sum, project) => sum + Object.values(project.counts).reduce((a, b) => a + b, 0),
    0,
  );

  const stats = useMemo(
    () => [
      { label: '灵感', value: app.data.workspace.ideas, icon: 'bulb' as const, path: '/ideas' },
      { label: '进行中的项目', value: app.data.projects.filter((p) => p.status === 'active').length, icon: 'folder' as const, path: '/projects' },
      { label: '素材', value: totalAssets, icon: 'layers' as const, path: '/projects' },
      { label: '本地占用', value: formatBytes(app.data.workspace.bytes), icon: 'sliders' as const, path: '/settings' },
    ],
    [app.data, totalAssets],
  );

  const saveDraft = async () => {
    const text = draft.trim();
    if (!text) return;
    const firstLine = text
      .split('\n')
      .map((line) => line.trim())
      .find(Boolean) ?? '无标题想法';
    setSaving(true);
    setError('');
    try {
      const idea = await api.createIdea({
        title: firstLine.replace(/^#+\s*/, '').slice(0, 80),
        content: text,
      });
      setDraft('');
      await Promise.all([app.refreshIdeas(), app.refreshWorkspace()]);
      app.toast({ message: '想法已收进灵感箱', tone: 'success' });
      navigate('/ideas');
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  return (
    <div class="page dashboard-page">
      <header class="page-heading dashboard-heading">
        <div>
          <p class="eyebrow">工作台 · {todayLabel()}</p>
          <h1>{greeting()}，创作者。</h1>
          <p class="page-subtitle">把一个模糊的念头，慢慢养成可以发布的作品。</p>
        </div>
        <div class="heading-actions">
          <Button variant="secondary" icon="search" onClick={app.openCommandPalette}>
            搜索
            <kbd>⌘K</kbd>
          </Button>
          <Button variant="primary" icon="pen" onClick={app.openQuickCapture}>
            速记想法
          </Button>
        </div>
      </header>

      <section class="capture-hero">
        <div class="capture-hero-head">
          <span class="capture-kicker"><Icon name="spark" size={15} /> 此刻的想法</span>
          <span class="capture-date">写下来，就不用一直记着</span>
        </div>
        <textarea
          class="capture-hero-textarea"
          value={draft}
          placeholder="今天想做什么内容？一句话、一个观点、一个画面都可以…"
          onInput={(e) => setDraft((e.target as HTMLTextAreaElement).value)}
          onKeyDown={(e) => {
            if ((e.metaKey || e.ctrlKey) && e.key === 'Enter') {
              e.preventDefault();
              saveDraft();
            }
          }}
        />
        <div class="capture-hero-foot">
          <span class="capture-tip">⌘ / Ctrl + Enter 保存 · 可稍后在灵感页补充标签</span>
          <div class="capture-hero-actions">
            {error ? <span class="form-error">{error}</span> : null}
            <Button variant="primary" icon="inbox" loading={saving} onClick={saveDraft}>
              收进灵感箱
            </Button>
          </div>
        </div>
      </section>

      <section class="stat-row">
        {stats.map((stat) => (
          <button class="stat-card" key={stat.label} onClick={() => navigate(stat.path)}>
            <span class="stat-icon"><Icon name={stat.icon} size={18} /></span>
            <span class="stat-value">{stat.value}</span>
            <span class="stat-label">{stat.label}</span>
          </button>
        ))}
      </section>

      <div class="dashboard-columns">
        <section class="panel">
          <div class="section-head">
            <div>
              <p class="eyebrow">PROJECTS</p>
              <h2>最近的项目</h2>
            </div>
            <button class="text-link" onClick={() => navigate('/projects')}>
              查看全部 <Icon name="arrow-right" size={14} />
            </button>
          </div>
          {recentProjects.length ? (
            <div class="project-grid project-grid-compact">
              {recentProjects.map((project) => (
                <ProjectCard project={project} key={project.id} />
              ))}
            </div>
          ) : (
            <EmptyState
              icon="folder"
              title="还没有项目"
              description="从一条想法开始，或者直接建立项目。项目会自动得到五类素材目录。"
              action={
                <Button variant="primary" icon="plus" onClick={() => navigate('/projects?new=1')}>
                  建立第一个项目
                </Button>
              }
            />
          )}
        </section>

        <aside class="dashboard-aside">
          <section class="panel">
            <div class="section-head">
              <div>
                <p class="eyebrow">INBOX</p>
                <h2>最新灵感</h2>
              </div>
              <button class="text-link" onClick={() => navigate('/ideas')}>
                全部
              </button>
            </div>
            {recentIdeas.length ? (
              <div class="idea-list-compact">
                {recentIdeas.map((idea) => (
                  <button
                    class="idea-row"
                    key={idea.id}
                    onClick={() => navigate(`/ideas?focus=${idea.id}`)}
                  >
                    <span class="idea-row-dot" />
                    <span class="idea-row-body">
                      <strong>{idea.title}</strong>
                      <small>{excerpt(idea.excerpt, 56)}</small>
                    </span>
                    <span class="idea-row-time">{relativeTime(idea.updatedAt)}</span>
                  </button>
                ))}
              </div>
            ) : (
              <div class="aside-empty">灵感箱是空的，先记下第一句话吧。</div>
            )}
          </section>

          <section class="panel guide-panel">
            <div class="section-head">
              <div>
                <p class="eyebrow">FLOW</p>
                <h2>创作动线</h2>
              </div>
            </div>
            <ol class="flow-list">
              <li>
                <span>01</span>
                <div><strong>随手记</strong><small>灵感不必完整，先收进来</small></div>
              </li>
              <li>
                <span>02</span>
                <div><strong>立项</strong><small>聚合成项目，拆出五类素材</small></div>
              </li>
              <li>
                <span>03</span>
                <div><strong>调用大模型</strong><small>预设与技能组合，生成初稿</small></div>
              </li>
              <li>
                <span>04</span>
                <div><strong>进 VSCode</strong><small>文件与上下文齐备，继续深写</small></div>
              </li>
            </ol>
          </section>
        </aside>
      </div>
    </div>
  );
}
