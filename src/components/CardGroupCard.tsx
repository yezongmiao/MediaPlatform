import type { Asset } from '../lib/types';
import { relativeTime } from '../lib/format';
import { Badge } from './ui';
import { Icon } from './Icon';

function groupTitle(assets: Asset[]): string {
  return assets[0]?.cardGroupTitle || assets[0]?.title || 'AI 文卡图组';
}

export function CardGroupCard({
  assets,
  onOpen,
  selected = false,
  selectable = false,
  onSelect,
  onDelete,
}: {
  assets: Asset[];
  onOpen: () => void;
  selected?: boolean;
  selectable?: boolean;
  onSelect?: () => void;
  onDelete?: () => void;
}) {
  const ordered = [...assets].sort(
    (a, b) => (a.cardGroupIndex ?? 0) - (b.cardGroupIndex ?? 0),
  );
  const images = ordered.filter((asset) => asset.fileUrl);
  const front = images[0];
  const first = ordered[0];
  const summary = ordered
    .slice(0, 3)
    .map((asset) => asset.title)
    .join(' · ');

  return (
    <article
      class={`asset-card card-group-card ${selected ? 'is-selected' : ''}`}
      onClick={onOpen}
      tabIndex={0}
      onKeyDown={(event) => {
        if (event.target !== event.currentTarget) return;
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          onOpen();
        }
      }}
    >
      <div class="asset-card-visual card-group-stack">
        {images.slice(0, 3).reverse().map((asset, index, all) => (
          <img
            class={`card-group-layer card-group-layer-${all.length - index}`}
            src={asset.fileUrl}
            alt=""
            loading="lazy"
            key={asset.id}
          />
        ))}
        {!front ? (
          <div class="card-group-empty">
            <Icon name="cards" size={26} />
          </div>
        ) : null}
        <div class="asset-card-tools">
          {selectable ? (
            <label class="asset-select" onClick={(event) => event.stopPropagation()}>
              <input
                type="checkbox"
                checked={selected}
                aria-label={`选择文卡图组：${groupTitle(ordered)}`}
                onChange={(event) => {
                  event.stopPropagation();
                  onSelect?.();
                }}
              />
            </label>
          ) : null}
          {onDelete ? (
            <button
              type="button"
              class="asset-delete"
              aria-label={`删除文卡图组：${groupTitle(ordered)}`}
              title="删除整组"
              onClick={(event) => {
                event.stopPropagation();
                onDelete();
              }}
            >
              <Icon name="trash" size={14} />
            </button>
          ) : null}
        </div>
        <span class="card-group-count">
          <Icon name="layers" size={12} /> {ordered.length} 张
        </span>
        <div class="asset-card-badges">
          {first ? (
            <Badge tone="neutral">文卡图组</Badge>
          ) : null}
        </div>
      </div>
      <div class="asset-card-body">
        <div class="asset-card-meta">
          <span class="asset-kind-label" style={{ color: 'var(--kind-card)' }}>AI 文卡图</span>
          <span>{first ? relativeTime(first.updatedAt) : ''}</span>
        </div>
        <h3>{groupTitle(ordered)}</h3>
        <p>{summary || '一次生成的文卡图组'}</p>
      </div>
    </article>
  );
}
