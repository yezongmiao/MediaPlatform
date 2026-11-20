import { createPortal } from 'preact/compat';
import type { ComponentChildren, JSX } from 'preact';
import { useEffect, useId, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { Icon } from './Icon';
import type { IconName } from './Icon';

type ButtonVariant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'quiet';
type ButtonSize = 'sm' | 'md';

export interface ButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: ButtonVariant;
  size?: ButtonSize;
  icon?: IconName;
  iconAfter?: IconName;
  loading?: boolean;
}

export function Button({
  variant = 'secondary',
  size = 'md',
  icon,
  iconAfter,
  loading,
  class: className,
  children,
  disabled,
  type = 'button',
  ...rest
}: ButtonProps) {
  return (
    <button
      type={type}
      class={`btn btn-${variant} btn-${size} ${className ?? ''}`}
      disabled={disabled || loading}
      {...rest}
    >
      {loading ? <span class="spinner spinner-inline" /> : icon ? <Icon name={icon} size={size === 'sm' ? 15 : 17} /> : null}
      {children ? <span>{children}</span> : null}
      {iconAfter ? <Icon name={iconAfter} size={size === 'sm' ? 15 : 17} /> : null}
    </button>
  );
}

interface IconButtonProps extends JSX.ButtonHTMLAttributes<HTMLButtonElement> {
  icon: IconName;
  label: string;
  size?: number;
  variant?: 'ghost' | 'soft' | 'danger';
}

export function IconButton({
  icon,
  label,
  size = 18,
  variant = 'ghost',
  class: className,
  ...rest
}: IconButtonProps) {
  return (
    <button
      type="button"
      class={`icon-btn icon-btn-${variant} ${className ?? ''}`}
      aria-label={label}
      title={label}
      {...rest}
    >
      <Icon name={icon} size={size} />
    </button>
  );
}

export function Badge({
  children,
  tone = 'neutral',
}: {
  children: ComponentChildren;
  tone?: string;
}) {
  return <span class={`badge badge-${tone}`}>{children}</span>;
}

export function Spinner({ size = 18 }: { size?: number }) {
  return <span class="spinner" style={{ width: `${size}px`, height: `${size}px` }} />;
}

export function Field({
  label,
  hint,
  children,
  required,
  htmlFor,
}: {
  label: string;
  hint?: string;
  children: ComponentChildren;
  required?: boolean;
  htmlFor?: string;
}) {
  return (
    <label class="field" htmlFor={htmlFor}>
      <span class="field-label">
        {label}
        {required ? <span class="field-required">必填</span> : null}
      </span>
      {children}
      {hint ? <span class="field-hint">{hint}</span> : null}
    </label>
  );
}

export function TagInput({
  value,
  onChange,
  placeholder = '输入标签后按回车',
  max = 8,
}: {
  value: string[];
  onChange: (tags: string[]) => void;
  placeholder?: string;
  max?: number;
}) {
  const [draft, setDraft] = useState('');
  const add = () => {
    const tag = draft.trim().replace(/^#/, '');
    if (!tag || value.includes(tag) || value.length >= max) {
      setDraft('');
      return;
    }
    onChange([...value, tag]);
    setDraft('');
  };
  return (
    <div class="tag-input">
      {value.map((tag) => (
        <span class="tag" key={tag}>
          {tag}
          <button type="button" aria-label={`移除标签 ${tag}`} onClick={() => onChange(value.filter((x) => x !== tag))}>
            <Icon name="close" size={11} />
          </button>
        </span>
      ))}
      <input
        value={draft}
        placeholder={value.length >= max ? '最多 8 个标签' : placeholder}
        disabled={value.length >= max}
        onInput={(e) => setDraft((e.target as HTMLInputElement).value)}
        onKeyDown={(e) => {
          if (e.key === 'Enter' || e.key === ',') {
            e.preventDefault();
            add();
          } else if (e.key === 'Backspace' && !draft && value.length) {
            onChange(value.slice(0, -1));
          }
        }}
        onBlur={add}
      />
    </div>
  );
}

export type ModalSize = 'sm' | 'md' | 'lg' | 'xl' | 'full';

export function Modal({
  open,
  onClose,
  title,
  subtitle,
  children,
  footer,
  size = 'md',
  class: className,
  hideClose = false,
}: {
  open: boolean;
  onClose: () => void;
  title?: ComponentChildren;
  subtitle?: ComponentChildren;
  children: ComponentChildren;
  footer?: ComponentChildren;
  size?: ModalSize;
  class?: string;
  hideClose?: boolean;
}) {
  const panelRef = useRef<HTMLDivElement>(null);
  const titleId = useId();

  useLayoutEffect(() => {
    if (!open) return;
    const previous = document.activeElement as HTMLElement | null;
    document.body.classList.add('modal-open');
    const timer = window.setTimeout(() => {
      const target =
        panelRef.current?.querySelector<HTMLElement>(
          '[data-autofocus], input:not([type=hidden]), textarea, select, button',
        ) ?? panelRef.current;
      target?.focus();
    }, 30);
    const handleKey = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return;
      // Nested dialogs portal later into <body>; only the top one may close.
      const panels = document.querySelectorAll('.modal-panel');
      if (panels[panels.length - 1] !== panelRef.current) return;
      event.stopPropagation();
      onClose();
    };
    document.addEventListener('keydown', handleKey);
    return () => {
      window.clearTimeout(timer);
      document.removeEventListener('keydown', handleKey);
      window.setTimeout(() => {
        if (!document.querySelector('.modal-panel')) {
          document.body.classList.remove('modal-open');
        }
      }, 0);
      previous?.focus?.();
    };
  }, [open, onClose]);

  if (!open) return null;

  return createPortal(
    <div
      class="modal-backdrop"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div
        ref={panelRef}
        class={`modal-panel modal-${size} ${className ?? ''}`}
        role="dialog"
        aria-modal="true"
        aria-labelledby={title ? titleId : undefined}
        tabIndex={-1}
      >
        {(title || !hideClose) && (
          <header class="modal-header">
            <div class="modal-title-wrap">
              {title ? (
                <h2 class="modal-title" id={titleId}>
                  {title}
                </h2>
              ) : null}
              {subtitle ? <p class="modal-subtitle">{subtitle}</p> : null}
            </div>
            {!hideClose && <IconButton icon="close" label="关闭" onClick={onClose} />}
          </header>
        )}
        <div class="modal-body">{children}</div>
        {footer ? <footer class="modal-footer">{footer}</footer> : null}
      </div>
    </div>,
    document.body,
  );
}

export function ConfirmDialog({
  open,
  title,
  message,
  confirmLabel = '删除',
  onConfirm,
  onClose,
  loading,
  danger = true,
}: {
  open: boolean;
  title: string;
  message: ComponentChildren;
  confirmLabel?: string;
  onConfirm: () => void;
  onClose: () => void;
  loading?: boolean;
  danger?: boolean;
}) {
  return (
    <Modal
      open={open}
      onClose={onClose}
      title={title}
      size="sm"
      footer={
        <>
          <Button variant="ghost" onClick={onClose}>
            取消
          </Button>
          <Button variant={danger ? 'danger' : 'primary'} onClick={onConfirm} loading={loading}>
            {confirmLabel}
          </Button>
        </>
      }
    >
      <p class="confirm-message">{message}</p>
    </Modal>
  );
}

export function EmptyState({
  icon = 'bulb',
  title,
  description,
  action,
}: {
  icon?: IconName;
  title: string;
  description?: ComponentChildren;
  action?: ComponentChildren;
}) {
  return (
    <div class="empty-state">
      <span class="empty-icon">
        <Icon name={icon} size={26} />
      </span>
      <h3>{title}</h3>
      {description ? <p>{description}</p> : null}
      {action ? <div class="empty-action">{action}</div> : null}
    </div>
  );
}

export function Segmented<T extends string>({
  options,
  value,
  onChange,
  size = 'md',
}: {
  options: { value: T; label: string; icon?: IconName }[];
  value: T;
  onChange: (value: T) => void;
  size?: 'sm' | 'md';
}) {
  return (
    <div class={`segmented segmented-${size}`} role="tablist">
      {options.map((option) => (
        <button
          key={option.value}
          type="button"
          role="tab"
          aria-selected={option.value === value}
          class={option.value === value ? 'is-active' : ''}
          onClick={() => onChange(option.value)}
        >
          {option.icon ? <Icon name={option.icon} size={15} /> : null}
          {option.label}
        </button>
      ))}
    </div>
  );
}

export function InlineError({ message, onRetry }: { message: string; onRetry?: () => void }) {
  return (
    <div class="inline-error" role="alert">
      <Icon name="alert" size={17} />
      <span>{message}</span>
      {onRetry ? (
        <Button size="sm" variant="ghost" onClick={onRetry}>
          重试
        </Button>
      ) : null}
    </div>
  );
}

export function Skeleton({ class: className }: { class?: string }) {
  return <span class={`skeleton ${className ?? ''}`} />;
}
