import { useCallback, useEffect, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import type { KernelStatus, TextKernel } from '../lib/types';
import { Button, Field, InlineError } from './ui';
import { Icon } from './Icon';

const KERNEL_OPTIONS: {
  id: TextKernel;
  label: string;
  description: string;
  icon: 'spark' | 'code' | 'sliders' | 'book';
}[] = [
  { id: 'api', label: 'API 大模型', description: '走下方 OpenAI 兼容接口', icon: 'spark' },
  { id: 'claude', label: 'Claude Code', description: '本机 claude CLI', icon: 'code' },
  { id: 'opencode', label: 'OpenCode', description: '本机 opencode CLI', icon: 'sliders' },
  { id: 'deepseek', label: 'DeepSeek CLI', description: '本机 deepseek CLI', icon: 'book' },
];

export function TextKernelSettings() {
  const app = useApp();
  const config = app.data.config;
  const [kernel, setKernel] = useState<TextKernel>(config.textKernel ?? 'api');
  const [binary, setBinary] = useState(config.cli?.binary ?? '');
  const [argsText, setArgsText] = useState((config.cli?.args ?? []).join('\n'));
  const [timeoutSecs, setTimeoutSecs] = useState(config.cli?.timeoutSecs ?? 240);
  const [status, setStatus] = useState<KernelStatus | null>(null);
  const [checking, setChecking] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState('');

  useEffect(() => {
    setKernel(config.textKernel ?? 'api');
    setBinary(config.cli?.binary ?? '');
    setArgsText((config.cli?.args ?? []).join('\n'));
    setTimeoutSecs(config.cli?.timeoutSecs ?? 240);
  }, [config]);

  const detect = useCallback(async () => {
    setChecking(true);
    try {
      setStatus(await api.getKernelStatus());
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '无法检测本机 CLI');
    } finally {
      setChecking(false);
    }
  }, []);

  useEffect(() => {
    detect();
  }, [detect]);

  const save = async () => {
    setSaving(true);
    setError('');
    try {
      await api.saveKernelConfig({
        kernel,
        binary: binary.trim() || null,
        args: argsText
          .split('\n')
          .map((line) => line.trim())
          .filter(Boolean),
        timeoutSecs,
      });
      await app.refreshConfig();
      setSaved(true);
      window.setTimeout(() => setSaved(false), 1800);
      app.toast({ message: '文本内核已保存', tone: 'success' });
      await detect();
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const selected = status?.kernels.find((item) => item.id === kernel);
  const isCli = kernel !== 'api';

  return (
    <section class="panel settings-card kernel-card">
      <div class="settings-card-head">
        <div>
          <p class="eyebrow">TEXT KERNEL · 文本内核</p>
          <h2>文章、脚本、文卡图由谁生成</h2>
        </div>
        <span class={`settings-badge ${kernel === 'api' ? '' : selected?.found ? 'is-ok' : 'is-warn'}`}>
          <span class="status-dot" />
          {kernel === 'api' ? 'API' : selected?.found ? 'CLI 就绪' : '未检测到 CLI'}
        </span>
      </div>

      <div class="kernel-grid">
        {KERNEL_OPTIONS.map((option) => (
          <button
            key={option.id}
            class={`kernel-card-option ${kernel === option.id ? 'is-active' : ''}`}
            onClick={() => setKernel(option.id)}
          >
            <span class="kernel-icon"><Icon name={option.icon} size={18} /></span>
            <span>
              <strong>{option.label}</strong>
              <small>{option.description}</small>
            </span>
            {kernel === option.id ? <Icon name="check" size={15} /> : null}
          </button>
        ))}
      </div>

      {isCli ? (
        <div class="kernel-cli-config">
          <div class={`kernel-status ${selected?.found ? 'is-ok' : 'is-missing'}`}>
            <Icon name={selected?.found ? 'check' : 'alert'} size={16} />
            <div>
              <strong>{selected?.found ? '已找到可执行文件' : '未找到可执行文件'}</strong>
              {selected?.path ? <code>{selected.path}</code> : <p>{selected?.installHint ?? '点击重新检测，或手动填写路径。'}</p>}
              {status?.envOverride ? <small>当前由 MEDIADESK_CLI_BIN 环境变量覆盖</small> : null}
            </div>
            <Button size="sm" variant="ghost" icon="refresh" loading={checking} onClick={detect}>
              重新检测
            </Button>
          </div>

          <div class="settings-form-grid">
            <Field label="CLI 路径" hint="留空自动从 PATH 探测；也可填写绝对路径">
              <input
                class="input"
                value={binary}
                placeholder={selected?.path ?? '自动探测'}
                onInput={(e) => setBinary((e.target as HTMLInputElement).value)}
              />
            </Field>
            <Field label="超时（秒）" hint="10–1800 秒">
              <input
                class="input"
                type="number"
                min={10}
                max={1800}
                value={timeoutSecs}
                onInput={(e) => setTimeoutSecs(Number((e.target as HTMLInputElement).value))}
              />
            </Field>
          </div>

          <Field label="附加参数" hint="每行一个；默认参数因内核而异。如 CLI 需要 prompt 作为参数，可使用 {prompt}">
            <textarea
              class="textarea kernel-args"
              rows={3}
              value={argsText}
              placeholder={kernel === 'claude' ? '-p' : kernel === 'opencode' ? 'run' : ''}
              onInput={(e) => setArgsText((e.target as HTMLTextAreaElement).value)}
            />
          </Field>
          <p class="settings-note">
            子进程会在当前项目目录内执行，项目路径与锁定的目录一致；stdout 作为生成结果，stderr 仅在失败时用于报错。
          </p>
        </div>
      ) : (
        <p class="settings-note">
          当前使用「大模型接入」中的 OpenAI 兼容接口。切换为本地 CLI 后，文章、脚本和文卡图生成会改为调用本机进程；插图和视频始终走媒体生成模型。
        </p>
      )}

      {error ? <InlineError message={error} /> : null}
      <div class="settings-actions-row end">
        <Button variant="primary" icon={saved ? 'check' : undefined} loading={saving} onClick={save}>
          {saved ? '已保存' : '保存文本内核'}
        </Button>
      </div>
    </section>
  );
}
