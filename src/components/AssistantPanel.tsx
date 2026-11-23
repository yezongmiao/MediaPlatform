import { useCallback, useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import type { Asset, AssistantMessage, AssistantMode, Kind, Project } from '../lib/types';
import { KINDS } from '../lib/types';
import { Button, ConfirmDialog, InlineError, Modal } from './ui';
import { Icon } from './Icon';
import { MarkdownView } from './MarkdownView';
import { navigate } from '../lib/router';

const SUGGESTIONS = [
  '帮我梳理这个项目还缺什么素材。',
  '把当前文章的标题改得更有钩子，先给我建议。',
  '基于项目简介，给插图提 3 个画面方向。',
  '检查现有素材里有没有重复或互相矛盾的地方。',
];

function messageTime(value: string): string {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return '';
  return date.toLocaleTimeString('zh-CN', { hour: '2-digit', minute: '2-digit' });
}

function kernelLabel(kernel: string): string {
  if (kernel === 'api') return 'API 大模型';
  if (kernel === 'claude') return 'Claude Code CLI';
  if (kernel === 'opencode') return 'OpenCode CLI';
  if (kernel === 'deepseek') return 'DeepSeek CLI';
  return kernel;
}

export function AssistantPanel({
  project,
  open,
  onClose,
  onChanged,
}: {
  project: Project;
  open: boolean;
  onClose: () => void;
  onChanged: () => void | Promise<void>;
}) {
  const app = useApp();
  const [messages, setMessages] = useState<AssistantMessage[]>([]);
  const [input, setInput] = useState('');
  const [mode, setMode] = useState<AssistantMode>('suggest');
  const [loading, setLoading] = useState(false);
  const [historyLoading, setHistoryLoading] = useState(false);
  const [error, setError] = useState('');
  const [errorCode, setErrorCode] = useState('');
  const [warnings, setWarnings] = useState<string[]>([]);
  const [kernel, setKernel] = useState('');
  const [clearOpen, setClearOpen] = useState(false);
  const [clearing, setClearing] = useState(false);
  const scrollRef = useRef<HTMLDivElement>(null);

  const loadHistory = useCallback(async () => {
    setHistoryLoading(true);
    setError('');
    try {
      const history = await api.getAssistantHistory(project.id);
      setMessages(history.messages);
      setKernel(history.kernel);
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '无法读取助手历史');
    } finally {
      setHistoryLoading(false);
    }
  }, [project.id]);

  useEffect(() => {
    if (open) loadHistory();
  }, [open, loadHistory]);

  useLayoutEffect(() => {
    if (!open) return;
    const node = scrollRef.current;
    if (node) node.scrollTop = node.scrollHeight;
  }, [messages, open]);

  const send = async () => {
    const message = input.trim();
    if (!message || loading) return;
    setLoading(true);
    setError('');
    setErrorCode('');
    setWarnings([]);
    try {
      const response = await api.chatAssistant(project.id, { message, mode });
      setMessages(response.messages);
      setKernel(response.kernel);
      setInput('');
      setWarnings(response.warnings);
      if (response.applied) {
        await onChanged();
        await Promise.all([app.refreshProjects(), app.refreshWorkspace()]);
        app.toast({ message: '助手已更新项目素材', tone: 'success' });
      }
    } catch (err) {
      const apiErr = err instanceof ApiError ? err : null;
      setError(apiErr?.message ?? '助手回复失败');
      setErrorCode(apiErr?.code ?? '');
    } finally {
      setLoading(false);
    }
  };

  const clear = async () => {
    setClearing(true);
    try {
      await api.clearAssistantHistory(project.id);
      setMessages([]);
      setWarnings([]);
      setClearOpen(false);
      app.toast({ message: '助手对话已清空', tone: 'success' });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '清空失败');
    } finally {
      setClearing(false);
    }
  };

  const assetById = (id: string): Asset | undefined => {
    for (const kind of KINDS) {
      const found = project.assets[kind.id]?.find((asset) => asset.id === id);
      if (found) return found;
    }
    return undefined;
  };

  const kindLabelFor = (id: string): Kind | undefined =>
    KINDS.find((kind) => project.assets[kind.id]?.some((asset) => asset.id === id))?.id;

  return (
    <>
      <Modal
        open={open}
        onClose={onClose}
        title="项目助手"
        subtitle={`${project.title} · 读取当前项目与素材，可讨论，也可直接改稿`}
        size="full"
        class="assistant-modal"
      >
        <div class="assistant-layout">
          <div class="assistant-toolbar">
            <div class="assistant-mode">
              <span class="assistant-mode-label">改动方式</span>
              <div class="assistant-mode-switch">
                <button
                  class={mode === 'suggest' ? 'is-active' : ''}
                  onClick={() => setMode('suggest')}
                >
                  建议式
                </button>
                <button
                  class={mode === 'apply' ? 'is-active' : ''}
                  onClick={() => setMode('apply')}
                >
                  覆盖式
                </button>
              </div>
              <span class="assistant-mode-hint">
                {mode === 'suggest' ? '只展示修改建议，不动文件。' : '会把建议直接写入素材，请谨慎。'}
              </span>
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
                <p>正在读取对话…</p>
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
                        <strong>{isUser ? '你' : '项目助手'}</strong>
                        <small>{messageTime(message.createdAt)}</small>
                      </div>
                      {isUser ? (
                        <p class="assistant-plain">{message.content}</p>
                      ) : (
                        <MarkdownView source={message.content} />
                      )}
                      {message.edits?.length ? (
                        <div class={`assistant-edits ${message.applied ? 'is-applied' : 'is-suggested'}`}>
                          <div class="assistant-edits-head">
                            <Icon name={message.applied ? 'check' : 'eye'} size={14} />
                            {message.applied
                              ? `已更新 ${message.edits.length} 条素材`
                              : `建议修改 ${message.edits.length} 条素材`}
                          </div>
                          {message.edits.map((edit) => {
                            const asset = assetById(edit.assetId);
                            const kind = kindLabelFor(edit.assetId);
                            return (
                              <div class="assistant-edit-row" key={`${message.id}-${edit.assetId}`}>
                                <span>
                                  <strong>{edit.title || asset?.title || edit.assetId}</strong>
                                  <small>
                                    {kind ? KINDS.find((item) => item.id === kind)?.label : '素材'}
                                    {edit.summary ? ` · ${edit.summary}` : ''}
                                  </small>
                                </span>
                                <span class={`assistant-edit-state ${message.applied ? 'is-applied' : ''}`}>
                                  {message.applied ? '已写入' : '未写入'}
                                </span>
                              </div>
                            );
                          })}
                        </div>
                      ) : null}
                    </div>
                  </div>
                );
              })
            ) : (
              <div class="assistant-empty">
                <span class="assistant-empty-mark">「</span>
                <h3>把项目当成一个可以对话的工作台。</h3>
                <p>助手会读取项目简介、灵感和现有素材。你可以让它梳理结构、改稿、补标题，或提出下一步创作方向。</p>
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
              <div class="assistant-typing">
                <span class="spinner" />
                <span>助手正在读取项目并组织回复…</span>
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
              {['CLI_NOT_FOUND', 'LLM_NOT_CONFIGURED', 'LLM_REQUEST_FAILED'].includes(errorCode) ? (
                <button class="text-link" onClick={() => { onClose(); navigate('/settings'); }}>
                  去设置检查内核
                </button>
              ) : null}
            </div>
          ) : null}

          <div class="assistant-composer">
            <textarea
              class="assistant-input"
              rows={3}
              value={input}
              placeholder={mode === 'apply' ? '描述你要改的素材；覆盖式会直接写入。' : '问点什么，或让助手提出改稿建议…'}
              aria-label="给项目助手的消息"
              onInput={(e) => setInput((e.target as HTMLTextAreaElement).value)}
              onKeyDown={(e) => {
                if (e.key === 'Enter' && !e.shiftKey && !(e as unknown as { isComposing?: boolean }).isComposing) {
                  e.preventDefault();
                  send();
                }
              }}
            />
            <div class="assistant-composer-foot">
              <span>Enter 发送 · Shift + Enter 换行 · {mode === 'apply' ? '覆盖模式会直接写文件' : '建议模式不会改文件'}</span>
              <Button
                variant="primary"
                icon="arrow-right"
                loading={loading}
                disabled={!input.trim() || historyLoading}
                onClick={send}
              >
                发送
              </Button>
            </div>
          </div>
        </div>
      </Modal>

      <ConfirmDialog
        open={clearOpen}
        title="清空助手对话？"
        message="历史记录会从项目目录删除，但已经写入的素材修改不会回滚。"
        confirmLabel="清空对话"
        loading={clearing}
        onClose={() => setClearOpen(false)}
        onConfirm={clear}
      />
    </>
  );
}
