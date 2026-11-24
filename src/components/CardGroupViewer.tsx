import { useEffect, useState } from 'preact/hooks';
import type { Asset } from '../lib/types';
import { Button, Modal } from './ui';
import { Icon } from './Icon';
import { relativeTime } from '../lib/format';

export function CardGroupViewer({
  assets,
  open,
  onClose,
  onOpenAsset,
  onDeleteAsset,
  onDeleteGroup,
}: {
  assets: Asset[];
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

  useEffect(() => {
    if (!open) {
      setSelectedIndex(0);
      return;
    }
    setSelectedIndex((current) => Math.min(current, Math.max(0, ordered.length - 1)));
  }, [open, assets]);

  const title = ordered[0]?.cardGroupTitle || ordered[0]?.title || 'AI 文卡图组';
  const selected = ordered[Math.min(selectedIndex, Math.max(0, ordered.length - 1))];

  return (
    <Modal
      open={open}
      onClose={onClose}
      title={title}
      subtitle={`同一次生成的 ${ordered.length} 张文卡图 · 点击下方叠放缩略图切换`}
      size="xl"
      class="card-group-modal"
      footer={
        <>
          <Button variant="ghost" icon="trash" onClick={onDeleteGroup}>
            删除整组
          </Button>
          <div class="modal-actions">
            <Button variant="secondary" onClick={onClose}>关闭</Button>
          </div>
        </>
      }
    >
      <div class="card-group-showcase">
        <div class="card-group-preview">
          <div class="card-group-preview-image">
            {selected?.fileUrl ? (
              <img src={selected.fileUrl} alt={selected.title} loading="eager" />
            ) : (
              <span class="card-group-thumb-empty"><Icon name="cards" size={28} /></span>
            )}
          </div>
          <div class="card-group-preview-meta">
            <span class="card-group-index">
              {String(selectedIndex + 1).padStart(2, '0')} / {String(ordered.length).padStart(2, '0')}
            </span>
            <h3>{selected?.title ?? '未命名文卡图'}</h3>
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

        <div class="card-group-deck" aria-label="叠加文卡图列表">
          {ordered.map((asset, index) => (
            <button
              key={asset.id}
              type="button"
              class={`card-group-deck-item ${index === selectedIndex ? 'is-active' : ''}`}
              onClick={() => setSelectedIndex(index)}
              onDblClick={() => onOpenAsset(asset)}
              aria-label={`第 ${index + 1} 张：${asset.title}`}
              title={asset.title}
            >
              {asset.fileUrl ? (
                <img src={asset.fileUrl} alt="" loading="lazy" />
              ) : (
                <span class="card-group-thumb-empty"><Icon name="cards" size={22} /></span>
              )}
              <span class="card-group-deck-index">{String(index + 1).padStart(2, '0')}</span>
            </button>
          ))}
        </div>
      </div>
    </Modal>
  );
}
