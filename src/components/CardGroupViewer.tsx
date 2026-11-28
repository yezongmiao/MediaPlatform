import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { createPortal } from 'preact/compat';
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
  const [zoomed, setZoomed] = useState(false);
  const previewRef = useRef<HTMLButtonElement>(null);
  const lightboxCloseRef = useRef<HTMLButtonElement>(null);
  const isIllustration = kind === 'illustration';

  const label = isIllustration ? '配图' : '文卡图';

  useEffect(() => {
    if (!open) {
      setSelectedIndex(0);
      setZoomed(false);
      return;
    }
    setSelectedIndex((current) => Math.min(current, Math.max(0, ordered.length - 1)));
  }, [open, assets]);

  useLayoutEffect(() => {
    if (!open || ordered.length < 2) return;
    const onKey = (event: KeyboardEvent) => {
      if (zoomed || isImeComposing(event)) return;
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
  }, [open, ordered.length, zoomed]);

  useLayoutEffect(() => {
    if (!open || !zoomed) return;
    lightboxCloseRef.current?.focus();
    const onKey = (event: KeyboardEvent) => {
      if (isImeComposing(event)) return;
      if (event.key === 'Escape') {
        event.preventDefault();
        event.stopPropagation();
        setZoomed(false);
        return;
      }
      if (event.key !== 'ArrowLeft' && event.key !== 'ArrowRight') return;
      event.preventDefault();
      event.stopPropagation();
      setSelectedIndex((current) =>
        event.key === 'ArrowLeft'
          ? (current - 1 + ordered.length) % ordered.length
          : (current + 1) % ordered.length,
      );
    };
    document.addEventListener('keydown', onKey, true);
    return () => document.removeEventListener('keydown', onKey, true);
  }, [open, zoomed, ordered.length]);

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
      subtitle={`同一次生成的 ${ordered.length} 张${label} · 点击放大 · ← → 切换`}
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
          <button
            ref={previewRef}
            type="button"
            class={`card-group-preview-image ${isIllustration ? 'is-illustration' : ''}`}
            onClick={() => selected?.fileUrl && setZoomed(true)}
            disabled={!selected?.fileUrl}
            aria-label={selected ? `放大预览：${selected.title}` : '放大预览'}
            title="点击放大，使用 ← → 切换"
          >
            {selected?.fileUrl ? (
              <>
                <img src={selected.fileUrl} alt={selected.title} loading="eager" />
                <span class="card-group-zoom-hint"><Icon name="eye" size={14} /> 放大</span>
              </>
            ) : (
              <span class="card-group-thumb-empty">
                <Icon name={isIllustration ? 'image' : 'cards'} size={28} />
              </span>
            )}
          </button>
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
              onClick={() => {
                setSelectedIndex(index);
                if (asset.fileUrl) setZoomed(true);
              }}
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

      {zoomed && selected?.fileUrl
        ? createPortal(
            <div
              class="lightbox group-lightbox"
              role="dialog"
              aria-modal="true"
              aria-label={`${title} 大图预览`}
              onClick={() => setZoomed(false)}
            >
              <button
                ref={lightboxCloseRef}
                class="lightbox-close"
                aria-label="关闭大图预览"
                onClick={(event) => {
                  event.stopPropagation();
                  setZoomed(false);
                }}
              >
                <Icon name="close" size={22} />
              </button>
              {ordered.length > 1 ? (
                <>
                  <button
                    type="button"
                    class="lightbox-nav is-prev"
                    aria-label="上一张"
                    onClick={(event) => {
                      event.stopPropagation();
                      setSelectedIndex((current) => (current - 1 + ordered.length) % ordered.length);
                    }}
                  >
                    <Icon name="chevron-right" size={26} />
                  </button>
                  <button
                    type="button"
                    class="lightbox-nav is-next"
                    aria-label="下一张"
                    onClick={(event) => {
                      event.stopPropagation();
                      setSelectedIndex((current) => (current + 1) % ordered.length);
                    }}
                  >
                    <Icon name="chevron-right" size={26} />
                  </button>
                </>
              ) : null}
              <img
                src={selected.fileUrl}
                alt={selected.title}
                onClick={(event) => event.stopPropagation()}
              />
              <div class="lightbox-caption" onClick={(event) => event.stopPropagation()}>
                <span class="lightbox-counter">
                  {String(selectedIndex + 1).padStart(2, '0')} / {String(ordered.length).padStart(2, '0')}
                </span>
                <span>{selected.title}</span>
                <span class="lightbox-hint">← → 切换 · Esc 关闭</span>
              </div>
            </div>,
            document.body,
          )
        : null}
    </Modal>
  );
}
