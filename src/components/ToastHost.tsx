import { Icon } from './Icon';

export interface ToastItem {
  id: number;
  message: string;
  tone: 'default' | 'success' | 'error';
}

export function ToastHost({ toasts, onDismiss }: { toasts: ToastItem[]; onDismiss: (id: number) => void }) {
  return (
    <div class="toast-host" role="status" aria-live="polite">
      {toasts.map((toast) => (
        <div class={`toast toast-${toast.tone}`} key={toast.id}>
          <span class="toast-icon">
            <Icon
              name={toast.tone === 'error' ? 'alert' : toast.tone === 'success' ? 'check' : 'bulb'}
              size={16}
            />
          </span>
          <span class="toast-message">{toast.message}</span>
          <button class="toast-close" aria-label="关闭提示" onClick={() => onDismiss(toast.id)}>
            <Icon name="close" size={13} />
          </button>
        </div>
      ))}
    </div>
  );
}
