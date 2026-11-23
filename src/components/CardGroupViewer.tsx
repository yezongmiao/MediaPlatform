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
  const title = ordered[0]?.cardGroupTitle || ordered[0]?.title || 'AI 文卡图组';
  return (
    <Modal
      open={open}
      onClose={onClose}
      title={title}
      subtitle={`一次生成的 ${ordered.length} 张文卡图 · 点击查看或编辑`}
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
      <div class="card-group-grid">
        {ordered.map((asset, index) => (
          <article class="card-group-item" key={asset.id}>
            <button
              type="button"
              class="card-group-thumb"
              onClick={() => onOpenAsset(asset)}
              aria-label={`打开第 ${index + 1} 张：${asset.title}`}
            >
              {asset.fileUrl ? (
                <img src={asset.fileUrl} alt={asset.title} loading="lazy" />
              ) : (
                <span class="card-group-thumb-empty"><Icon name="cards" size={24} /></span>
              )}
            </button>
            <div class="card-group-item-body">
              <span class="card-group-index">{String(index + 1).padStart(2, '0')} / {String(ordered.length).padStart(2, '0')}</span>
              <h3>{asset.title}</h3>
              <p>{relativeTime(asset.updatedAt)}</p>
            </div>
            <div class="card-group-item-actions">
              <Button size="sm" variant="ghost" icon="edit" onClick={() => onOpenAsset(asset)}>
                打开
              </Button>
              <Button size="sm" variant="ghost" icon="trash" onClick={() => onDeleteAsset(asset)}>
                删除
              </Button>
            </div>
          </article>
        ))}
      </div>
    </Modal>
  );
}
