import type { Asset } from '../lib/types';
import { KIND_MAP, ASSET_STATUS } from '../lib/types';
import { Badge } from './ui';
import { Icon } from './Icon';
import { excerpt, relativeTime } from '../lib/format';

export function AssetCard({
  asset,
  onOpen,
  onPreview,
  selected = false,
  selectable = false,
  onSelect,
  onDelete,
}: {
  asset: Asset;
  onOpen: () => void;
  onPreview?: () => void;
  selected?: boolean;
  selectable?: boolean;
  onSelect?: () => void;
  onDelete?: () => void;
}) {
  const kind = KIND_MAP[asset.kind];
  const image = asset.fileUrl && (asset.kind === 'illustration' || asset.kind === 'card');
  const video = asset.kind === 'video';

  return (
    <article
      class={`asset-card asset-card-${asset.kind} ${selected ? 'is-selected' : ''}`}
      onClick={onOpen}
      tabIndex={0}
      onKeyDown={(e) => {
        if (e.target !== e.currentTarget) return;
        if (e.key === 'Enter' || e.key === ' ') {
          e.preventDefault();
          onOpen();
        }
      }}
    >
      <div class="asset-card-visual" style={{ '--asset-accent': kind.accent } as Record<string, string>}>
        {image ? (
          <>
            <img src={asset.fileUrl} alt={asset.title} loading="lazy" />
            {onPreview ? (
              <button
                class="asset-zoom"
                aria-label="预览大图"
                title="预览大图"
                onClick={(e) => {
                  e.stopPropagation();
                  onPreview();
                }}
              >
                <Icon name="eye" size={15} />
              </button>
            ) : null}
          </>
        ) : video ? (
          <div class="asset-video-fallback">
            <Icon name="play" size={26} />
            <span>{asset.file ? '视频文件' : '分镜 / 笔记'}</span>
          </div>
        ) : (
          <div class="asset-paper">
            <span class="asset-paper-kind">{kind.label}</span>
            <div class="asset-paper-lines">
              <span /><span /><span /><span />
            </div>
            <span class="asset-paper-mark">{asset.title.slice(0, 1)}</span>
          </div>
        )}
        <div class="asset-card-tools">
          {selectable ? (
            <label class="asset-select" onClick={(e) => e.stopPropagation()}>
              <input
                type="checkbox"
                checked={selected}
                aria-label={`选择素材：${asset.title}`}
                onChange={(e) => {
                  e.stopPropagation();
                  onSelect?.();
                }}
              />
            </label>
          ) : null}
          {onDelete ? (
            <button
              type="button"
              class="asset-delete"
              aria-label={`删除素材：${asset.title}`}
              title="删除素材"
              onClick={(e) => {
                e.stopPropagation();
                onDelete();
              }}
            >
              <Icon name="trash" size={14} />
            </button>
          ) : null}
        </div>
        <div class="asset-card-badges">
          <Badge tone={ASSET_STATUS[asset.status]?.color ?? 'neutral'}>
            {ASSET_STATUS[asset.status]?.label ?? asset.status}
          </Badge>
          {asset.source === 'llm' ? <span class="ai-badge">AI</span> : null}
        </div>
      </div>
      <div class="asset-card-body">
        <div class="asset-card-meta">
          <span class="asset-kind-label" style={{ color: kind.accent }}>{kind.label}</span>
          <span>{relativeTime(asset.updatedAt)}</span>
        </div>
        <h3>{asset.title}</h3>
        <p>{excerpt(asset.content || asset.prompt || (image ? '图片素材' : ''), 80) || '还没有内容'}</p>
      </div>
    </article>
  );
}
