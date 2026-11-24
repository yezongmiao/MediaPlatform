import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import { isImeComposing } from '../lib/ime';
import type {
  LibraryAction,
  LibraryAssistantMessage,
  LibraryAssistantResponse,
} from '../lib/types';
import { Button, ConfirmDialog, IconButton, InlineError } from './ui';
import { Icon } from './Icon';
import { MarkdownView } from './MarkdownView';
import { extractStreamReply } from '../lib/assistant-stream';

const SUGGESTIONS = [
  '盘点现有技能，指出重复、缺口和可以合并的内容。',
  '为文章创作新建一套更具体的预设，并关联已有技能。',
  '把「标题实验」技能改得更可执行，给出完整正文。',
  '删除重复或长期不用的技能；先说明会删哪些。',
];

function kernelLabel(kernel: string): string {
  if (kernel === 'api') return 'API 大模型';
  if (kernel === 'claude') return 'Claude Code CLI';
  if (kernel === 'codex') return 'Codex CLI';
  if (kernel === 'opencode') return 'OpenCode CLI';
  if (kernel === 'deepseek') return 'DeepSeek CLI';
  return kernel || '未配置';
}

function actionTypeLabel(action: LibraryAction): string {
  switch (action.type) {
    case 'createSkill': return '新建技能';
    case 'updateSkill': return '更新技能';
    case 'deleteSkill': return '删除技能';
    case 'createPreset': return '新建预设';
    case 'updatePreset': return '更新预设';
    case 'deletePreset': return '删除预设';
  }
}

function actionLabel(action: LibraryAction): string {
  if (action.type === 'deleteSkill' || action.type === 'deletePreset') {
    return action.id || '未指定 id';
  }
  return action.name || action.id || '未命名';
}

function messageTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '';
  return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
}

export function LibraryAssistantPanel({
  open,
  onClose,
  onChanged,
}: {
  open: boolean;
  onClose: () => void;
  onChanged: () => void | Promise<void>;
}) {
  const app = useApp();
  const [messages, setMessages] = useState<LibraryAssistantMessage[]>([]);
  const [input, setInput] = useState('');
  const [loading, setLoading] = useState(false);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [error, setError] = useState('');
  const [warnings, setWarnings] = useState<string[]>([]);
  const [streamText, setStreamText] = useState('');
  const [streamStatus, setStreamStatus] = useState('');
  const [elapsed, setElapsed] = useState(0);
  const [kernel, setKernel] = useState('');
  const [clearOpen, setClearOpen] = useState(false);
  const [clearing, setClearing] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  const loadHistory = useCallback(async () => {
    setHistoryLoading(true);
    setError('');
    try {
      const history = await api.getLibraryAssistantHistory();
      setMessages(history.messages);
      setKernel(history.kernel);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '无法读取技能助手历史');
    } finally {
      setHistoryLoading(false);
    }
  }, []);

  useEffect(() => {
    if (open) loadHistory();
  }, [open, loadHistory]);

  useLayoutEffect(() => {
    if (!open) return;
    const node = scrollRef.current;
    if (node) node.scrollTop = node.scrollHeight;
  }, [messages, streamText, streamStatus, loading, open]);

  useEffect(() => {
    if (!loading) {
      setElapsed(0);
      return;
    }
    const timer = window.setInterval(() => setElapsed((value) => value + 1), 1000);
    return () => window.clearInterval(timer);
  }, [loading]);

  const send = async () => {
    const message = input.trim();
    if (!message || loading || historyLoading) return;
    const pendingId = `pending-${Date.now()}`;
    setMessages((current) => [
      ...current,
      {
        id: pendingId,
        role: 'user',
        content: message,
        createdAt: new Date().toISOString(),
      },
    ]);
    setInput('');
    setLoading(true);
    setElapsed(0);
    setStreamText('');
    setStreamStatus('正在读取技能与预设…');
    setError('');
    setWarnings([]);
    try {
      const response: LibraryAssistantResponse = await api.streamLibraryAssistant(
        { message },
        (event) => {
          if (event.type === 'status') {
            setStreamStatus(event.message);
          } else if (event.type === 'delta') {
            setStreamText((current) => current + event.text);
            setStreamStatus('正在接收…');
          }
        },
      );
      setMessages(response.messages);
      setKernel(response.kernel);
      setWarnings(response.warnings);
      setStreamText('');
      setStreamStatus('');
      if (response.applied) {
        await onChanged();
        app.toast({ message: `技能助手已执行 ${response.results.filter((result) => result.ok).length} 项变更`, tone: 'success' });
      }
    } catch (err) {
      setMessages((current) => current.filter((item) => item.id !== pendingId));
      setInput(message);
      setError(err instanceof ApiError ? err.message : '技能助手回复失败');
    } finally {
      setLoading(false);
      setStreamText('');
      setStreamStatus('');
    }
  };

  const clear = async () => {
    setClearing(true);
    try {
      await api.clearLibraryAssistantHistory();
      setMessages([]);
      setWarnings([]);
      setClearOpen(false);
      app.toast({ message: '技能助手对话已清空', tone: 'success' });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '清空失败');
    } finally {
      setClearing(false);
    }
  };

  const streamPreview = extractStreamReply(streamText);

  return (
    <>
      <div class="assistant-scrim" onClick={onClose} />
      <aside class="assistant-sidebar library-assistant-sidebar" aria-label="技能与预设助手">
        <header class="assistant-sidebar-header">
          <div>
            <p class="eyebrow">LIBRARY ASSISTANT</p>
            <h2>技能与预设助手</h2>
            <p class="assistant-sidebar-subtitle">独立上下文 · 只管理技能与预设</p>
          </div>
          <IconButton icon="close" label="关闭技能助手" onClick={onClose} />
        </header>
        <div class="assistant-layout">
          <div class="assistant-toolbar">
            <div class="assistant-mode">
              <span class="assistant-mode-label">变更方式</span>
              <span class="assistant-direct-mode">
                <Icon name="check" size={12} />
                直接执行
              </span>
              <span class="assistant-mode-hint">助手会直接写入技能与预设文件；不读取项目素材。</span>
            </div>
            <div class="assistant-toolbar-right">
              <span class="assistant-kernel">
                <Icon name="code" size={13} />
                {kernelLabel(kernel || app.data.config.textKernel)}
              </span>
              <Button
                size="sm"
                variant="ghost"
                icon="trash"
                disabled={!messages.length}
                onClick={() => setClearOpen(true)}
              >
                清空
              </Button>
            </div>
          </div>

          <div class="assistant-thread" ref={scrollRef}>
            {historyLoading ? (
              <div class="assistant-empty">
                <span class="spinner" />
                <p>正在读取技能库对话…</p>
              </div>
            ) : messages.length ? (
              messages.map((message) => {
                const isUser = message.role === 'user';
                return (
                  <div class={`assistant-message ${isUser ? 'is-user' : 'is-assistant'}`} key={message.id}>
                    <div class="assistant-avatar">
                      {isUser ? '你' : <Icon name="spark" size={16} />}
                    </div>
                    <div class="assistant-bubble">
                      <div class="assistant-bubble-head">
                        <strong>{isUser ? '你' : '技能助手'}</strong>
                        <small>{messageTime(message.createdAt)}</small>
                      </div>
                      {isUser ? (
                        <p class="assistant-plain">{message.content}</p>
                      ) : (
                        <MarkdownView source={message.content} />
                      )}
                      {message.actions?.length ? (
                        <div class={`assistant-edits ${message.applied ? 'is-applied' : 'is-suggested'}`}>
                          <div class="assistant-edits-head">
                            <Icon name={message.applied ? 'check' : 'alert'} size={14} />
                            {message.applied
                              ? `已执行 ${message.actions.length} 项技能/预设变更`
                              : '变更未执行'}
                          </div>
                          {message.actions.map((action, index) => (
                            <div class="assistant-edit-row library-action-row" key={`${message.id}-${index}`}>
                              <span>
                                <strong>{actionLabel(action)}</strong>
                                <small>{actionTypeLabel(action)}{action.description ? ` · ${action.description}` : ''}</small>
                              </span>
                              <span class={`assistant-edit-state ${message.applied ? 'is-applied' : ''}`}>
                                {message.applied ? '已执行' : '未执行'}
                              </span>
                            </div>
                          ))}
                        </div>
                      ) : null}
                    </div>
                  </div>
                );
              })
            ) : (
              <div class="assistant-empty">
                <span class="assistant-empty-mark">「</span>
                <h3>把技能库当成可以对话的工作台。</h3>
                <p>这里的对话与项目助手完全分开，只读取技能和预设文件。助手可以直接创建、改写、删除技能与预设。</p>
                <div class="assistant-suggestions">
                  {SUGGESTIONS.map((suggestion) => (
                    <button key={suggestion} onClick={() => setInput(suggestion)}>
                      {suggestion}
                      <Icon name="arrow-right" size={14} />
                    </button>
                  ))}
                </div>
              </div>
            )}
            {loading ? (
              <div class="assistant-message is-assistant is-streaming">
                <div class="assistant-avatar"><Icon name="spark" size={16} /></div>
                <div class="assistant-bubble">
                  <div class="assistant-bubble-head">
                    <strong>技能助手</strong>
                    <small>{elapsed ? `${elapsed}s · 流式` : '流式'}</small>
                  </div>
                  {streamPreview ? (
                    <>
                      <MarkdownView source={streamPreview} />
                      <span class="assistant-stream-caret" aria-hidden="true" />
                      <p class="assistant-stream-meta">
                        {streamStatus || '正在接收…'}{elapsed ? ` · ${elapsed}s` : ''}
                      </p>
                    </>
                  ) : (
                    <div class="assistant-stream-wait">
                      <span class="spinner" />
                      <span>{streamStatus || '正在读取技能库并组织回复…'}</span>
                      {elapsed ? <small>{elapsed}s</small> : null}
                    </div>
                  )}
                </div>
              </div>
            ) : null}
          </div>

          {warnings.length ? (
            <div class="assistant-warnings">
              <Icon name="alert" size={15} />
              <div>
                {warnings.map((warning) => (
                  <p key={warning}>{warning}</p>
                ))}
              </div>
            </div>
          ) : null}
          {error ? (
            <div class="assistant-error">
              <InlineError message={error} />
            </div>
          ) : null}

          <div class="assistant-composer">
            <textarea
              class="assistant-input"
              rows={3}
              value={input}
              placeholder="例如：盘点现有技能，合并重复项；或为文卡图新建一个预设。"
              aria-label="给技能与预设助手的消息"
              onInput={(e) => setInput((e.target as HTMLTextAreaElement).value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey && !isImeComposing(e)) {
                  e.preventDefault();
                  send();
                }
              }}
            />
            <div class="assistant-composer-foot">
              <span>Enter 发送 · Shift + Enter 换行 · 变更会直接写文件</span>
              <Button
                variant="primary"
                icon="arrow-right"
                loading={loading}
                disabled={!input.trim() || historyLoading || loading}
                onClick={send}
              >
                发送
              </Button>
            </div>
          </div>
        </div>
      </aside>

      <ConfirmDialog
        open={clearOpen}
        title="清空技能助手对话？"
        message="只会删除这段独立对话历史，不会删除或回滚已经创建、修改、删除的技能与预设。"
        confirmLabel="清空对话"
        loading={clearing}
        onClose={() => setClearOpen(false)}
        onConfirm={clear}
      />
    </>
  );
}
