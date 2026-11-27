import { useEffect, useLayoutEffect, useState } from 'preact/hooks';
import type { Asset } from '../lib/types';
import { Button, Modal } from './ui';
import { Icon } from './Icon';
import { relativeTime } from '../lib/format';
import { isImeComposing } from '../lib/ime';
import type { AssetGroupKind } from './CardGroupCard';

export function CardGroupViewer({
  assets,
  kind = 'card',
  open,
  onClose,
  onOpenAsset,
  onDeleteAsset,
  onDeleteGroup,
}: {
  assets: Asset[];
  kind?: AssetGroupKind;
  open: boolean;
  onClose: () => void;
  onOpenAsset: (asset: Asset) => void;
  onDeleteAsset: (asset: Asset) => void;
  onDeleteGroup: () => void;
}) {
  const ordered = [...assets].sort(
    (a, b) => (a.cardGroupIndex ?? 0) - (b.cardGroupIndex ?? 0),
  );
  const [selectedIndex, setSelectedIndex] = useState(0);
  const isIllustration = kind === 'illustration';

  const label = isIllustration ? '配图' : '文卡图';

  useEffect(() => {
    if (!open) {
      setSelectedIndex(0);
      return;
    }
    setSelectedIndex((current) => Math.min(current, Math.max(0, ordered.length - 1)));
  }, [open, assets]);

  useLayoutEffect(() => {
    if (!open || ordered.length < 2) return;
    const onKey = (event: KeyboardEvent) => {
      if (isImeComposing(event)) return;
      if (
        event.target instanceof HTMLElement &&
        event.target.closest('input, textarea, select, [contenteditable="true"]')
      ) {
        return;
      }
      if (event.key === 'ArrowLeft') {
        event.preventDefault();
        setSelectedIndex((current) => (current - 1 + ordered.length) % ordered.length);
      } else if (event.key === 'ArrowRight') {
        event.preventDefault();
        setSelectedIndex((current) => (current + 1) % ordered.length);
      }
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [open, ordered.length]);

  const title =
    ordered[0]?.cardGroupTitle ||
    ordered[0]?.title ||
    (isIllustration ? 'AI 配图组' : 'AI 文卡图组');
  const selected = ordered[Math.min(selectedIndex, Math.max(0, ordered.length - 1))];

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={title}
      subtitle={`同一次生成的 ${ordered.length} 张${label} · ← → 切换`}
      size="xl"
      class={`card-group-modal ${isIllustration ? 'illustration-group-modal' : ''}`}
      footer={
        <>
          <Button variant="ghost" icon="trash" onClick={onDeleteGroup}>
            删除整组
          </Button>
          <div class="modal-actions">
            <span class="shortcut-hint">← → 切换 · Esc 关闭</span>
            <Button variant="secondary" onClick={onClose}>关闭</Button>
          </div>
        </>
      }
    >
      <div class={`card-group-showcase ${isIllustration ? 'is-illustration' : ''}`}>
        <div class="card-group-preview">
          <div class={`card-group-preview-image ${isIllustration ? 'is-illustration' : ''}`}>
            {selected?.fileUrl ? (
              <img src={selected.fileUrl} alt={selected.title} loading="eager" />
            ) : (
              <span class="card-group-thumb-empty">
                <Icon name={isIllustration ? 'image' : 'cards'} size={28} />
              </span>
            )}
          </div>
          <div class="card-group-preview-meta">
            <span class="card-group-index">
              {String(selectedIndex + 1).padStart(2, '0')} / {String(ordered.length).padStart(2, '0')}
            </span>
            <h3>{selected?.title ?? `未命名${label}`}</h3>
            <p>{selected ? relativeTime(selected.updatedAt) : ''}</p>
            <div class="card-group-preview-actions">
              <Button
                size="sm"
                variant="primary"
                icon="edit"
                disabled={!selected}
                onClick={() => selected && onOpenAsset(selected)}
              >
                打开这张
              </Button>
              <Button
                size="sm"
                variant="ghost"
                icon="trash"
                disabled={!selected}
                onClick={() => selected && onDeleteAsset(selected)}
              >
                删除这张
              </Button>
            </div>
          </div>
        </div>

        <div class="card-group-deck" aria-label={`叠加${label}列表`}>
          {ordered.map((asset, index) => (
            <button
              key={asset.id}
              type="button"
              class={`card-group-deck-item ${isIllustration ? 'is-illustration' : ''} ${index === selectedIndex ? 'is-active' : ''}`}
              onClick={() => setSelectedIndex(index)}
              onDblClick={() => onOpenAsset(asset)}
              aria-label={`第 ${index + 1} 张：${asset.title}`}
              title={asset.title}
            >
              {asset.fileUrl ? (
                <img src={asset.fileUrl} alt="" loading="lazy" />
              ) : (
                <span class="card-group-thumb-empty">
                  <Icon name={isIllustration ? 'image' : 'cards'} size={22} />
                </span>
              )}
              <span class="card-group-deck-index">{String(index + 1).padStart(2, '0')}</span>
            </button>
          ))}
        </div>
      </div>
    </Modal>
  );
}
