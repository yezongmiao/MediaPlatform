import type { ProjectSummary } from '../lib/types';
import { KINDS } from '../lib/types';
import { Icon } from './Icon';
import { Badge } from './ui';
import { navigate } from '../lib/router';
import { projectInitials, relativeTime, statusLabel, statusTone } from '../lib/format';

export function ProjectCard({ project }: { project: ProjectSummary }) {
  return (
    <article
      class="project-card"
      tabIndex={0}
      role="link"
      onClick={() => navigate(`/projects/${project.id}`)}
      onKeyDown={(e) => {
        if (e.key === 'Enter' || e.key === ' ') navigate(`/projects/${project.id}`);
      }}
    >
      <div class="project-cover">
        {project.coverUrl ? (
          <img src={project.coverUrl} alt="" loading="lazy" />
        ) : (
          <div class="project-cover-fallback" aria-hidden="true">
            <span class="project-initial">{projectInitials(project.title)}</span>
            <span class="cover-grid" />
          </div>
        )}
        <div class="project-cover-top">
          <Badge tone={statusTone(project.status, 'project')}>{statusLabel(project.status, 'project')}</Badge>
        </div>
      </div>
      <div class="project-card-body">
        <div class="project-card-heading">
          <h3>{project.title}</h3>
          <span class="project-updated">{relativeTime(project.updatedAt)}</span>
        </div>
        <p class="project-card-brief">{project.briefExcerpt || '还没有项目简介'}</p>
        <div class="project-card-foot">
          <div class="project-kind-stats">
            {KINDS.map((kind) => (
              <span
                class={`kind-stat ${project.counts[kind.id] ? 'has-items' : ''}`}
                key={kind.id}
                title={`${kind.label} ${project.counts[kind.id] ?? 0}`}
              >
                <span class="kind-dot" style={{ background: kind.accent }} />
                {project.counts[kind.id] ?? 0}
              </span>
            ))}
          </div>
          <span class="project-open-hint">
            打开
            <Icon name="arrow-right" size={14} />
          </span>
        </div>
      </div>
    </article>
  );
}
