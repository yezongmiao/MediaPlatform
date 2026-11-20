import { useEffect, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import { FolderBrowser } from '../components/FolderBrowser';
import { Icon } from '../components/Icon';
import { Button, Field, InlineError, Modal, Segmented } from '../components/ui';
import { copyText, formatBytes } from '../lib/format';
import type { ThemeMode } from '../lib/types';

const PROVIDERS = [
  {
    id: 'deepseek',
    name: 'DeepSeek',
    baseUrl: 'https://api.deepseek.com/v1',
    model: 'deepseek-chat',
    hint: '国内网络友好，性价比高',
  },
  {
    id: 'openai',
    name: 'OpenAI',
    baseUrl: 'https://api.openai.com/v1',
    model: 'gpt-4o-mini',
    hint: '需要可用的 OpenAI API Key',
  },
  {
    id: 'moonshot',
    name: 'Moonshot',
    baseUrl: 'https://api.moonshot.cn/v1',
    model: 'moonshot-v1-8k',
    hint: 'Kimi 开放平台',
  },
  {
    id: 'ollama',
    name: 'Ollama 本地',
    baseUrl: 'http://localhost:11434/v1',
    model: 'qwen2.5:7b',
    hint: '无需 API Key，先启动本地服务',
  },
];

export function SettingsPage() {
  const app = useApp();
  const config = app.data.config;
  const [baseUrl, setBaseUrl] = useState(config.llm.baseUrl);
  const [model, setModel] = useState(config.llm.model);
  const [apiKey, setApiKey] = useState('');
  const [hasKey, setHasKey] = useState(config.llm.hasApiKey);
  const [clearKey, setClearKey] = useState(false);
  const [showKey, setShowKey] = useState(false);
  const [temperature, setTemperature] = useState(config.llm.temperature);
  const [maxTokens, setMaxTokens] = useState(config.llm.maxTokens);
  const [timeoutSecs, setTimeoutSecs] = useState(config.llm.timeoutSecs);
  const [saving, setSaving] = useState(false);
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ ok: boolean; message: string; model?: string; latencyMs?: number } | null>(null);
  const [error, setError] = useState('');
  const [folderOpen, setFolderOpen] = useState(false);
  const [switching, setSwitching] = useState(false);
  const [savedFlash, setSavedFlash] = useState(false);

  useEffect(() => {
    setBaseUrl(config.llm.baseUrl);
    setModel(config.llm.model);
    setTemperature(config.llm.temperature);
    setMaxTokens(config.llm.maxTokens);
    setTimeoutSecs(config.llm.timeoutSecs);
    setHasKey(config.llm.hasApiKey);
    setApiKey('');
    setClearKey(false);
  }, [config]);

  const keyPayload = () => {
    if (clearKey) return '';
    if (apiKey.trim()) return apiKey.trim();
    return undefined;
  };

  const applyProvider = (provider: (typeof PROVIDERS)[number]) => {
    setBaseUrl(provider.baseUrl);
    setModel(provider.model);
    setTestResult(null);
  };

  const save = async () => {
    setSaving(true);
    setError('');
    try {
      const updated = await api.saveConfig({
        baseUrl,
        model,
        temperature,
        maxTokens,
        timeoutSecs,
        apiKey: keyPayload(),
      });
      setHasKey(updated.llm.hasApiKey);
      setApiKey('');
      setClearKey(false);
      setSavedFlash(true);
      window.setTimeout(() => setSavedFlash(false), 1800);
      await app.refreshConfig();
      app.toast({ message: '大模型设置已保存', tone: 'success' });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const test = async () => {
    setTesting(true);
    setTestResult(null);
    setError('');
    try {
      const result = await api.testLlm({
        baseUrl,
        model,
        temperature,
        maxTokens,
        timeoutSecs,
        apiKey: keyPayload(),
      });
      setTestResult(result);
      await app.refreshConfig();
      app.toast({ message: `连接成功 · ${result.latencyMs}ms`, tone: 'success' });
    } catch (err) {
      const message = err instanceof ApiError ? err.message : '连接测试失败';
      setTestResult({ ok: false, message });
      setError('');
    } finally {
      setTesting(false);
    }
  };

  const switchWorkspace = async (path: string) => {
    setSwitching(true);
    try {
      await api.switchWorkspace(path);
      await app.refreshAll();
      app.toast({ message: '工作区已切换', tone: 'success' });
      setFolderOpen(false);
    } finally {
      setSwitching(false);
    }
  };

  const theme = app.theme;
  const providerActive = (provider: (typeof PROVIDERS)[number]) =>
    baseUrl === provider.baseUrl && model === provider.model;

  return (
    <div class="page settings-page">
      <header class="page-heading">
        <div>
          <p class="eyebrow">SETTINGS · 设置</p>
          <h1>把工具调成顺手的样子。</h1>
          <p class="page-subtitle">工作区、模型与外观。所有配置只保存在本机。</p>
        </div>
      </header>

      <div class="settings-layout">
        <section class="panel settings-card">
          <div class="settings-card-head">
            <div>
              <p class="eyebrow">WORKSPACE</p>
              <h2>工作区</h2>
            </div>
            <span class="settings-badge"><Icon name="folder" size={14} /> 本地文件</span>
          </div>
          <div class="workspace-path-card">
            <span class="workspace-path-icon"><Icon name="folder-open" size={20} /></span>
            <div>
              <strong>{app.data.workspace.path}</strong>
              <small>
                {app.data.workspace.ideas} 条想法 · {app.data.workspace.projects} 个项目 ·{' '}
                {app.data.workspace.assets} 个素材 · {formatBytes(app.data.workspace.bytes)}
              </small>
            </div>
          </div>
          <div class="settings-actions-row">
            <Button variant="secondary" icon="folder" onClick={() => setFolderOpen(true)}>
              更换工作区
            </Button>
            <Button
              variant="ghost"
              icon="folder-open"
              onClick={async () => {
                try {
                  await api.revealWorkspace();
                } catch (err) {
                  app.toast({ message: err instanceof ApiError ? err.message : '无法打开文件夹', tone: 'error' });
                }
              }}
            >
              在 Finder / 资源管理器中打开
            </Button>
            <Button
              variant="ghost"
              icon="copy"
              onClick={async () => {
                await copyText(app.data.workspace.path);
                app.toast({ message: '路径已复制', tone: 'success' });
              }}
            >
              复制路径
            </Button>
          </div>
          <p class="settings-note">
            工作区是可移植的普通目录：<code>ideas/</code>、<code>projects/</code> 与{' '}
            <code>.mediadesk/</code>。可以放进 Git、iCloud 或移动硬盘。
          </p>
        </section>

        <section class="panel settings-card">
          <div class="settings-card-head">
            <div>
              <p class="eyebrow">MODEL</p>
              <h2>大模型接入</h2>
            </div>
            <span class={`settings-badge ${hasKey ? 'is-ok' : ''}`}>
              <span class="status-dot" />
              {hasKey ? '已配置密钥' : '未配置密钥'}
            </span>
          </div>

          <div class="provider-grid">
            {PROVIDERS.map((provider) => (
              <button
                key={provider.id}
                class={`provider-card ${providerActive(provider) ? 'is-active' : ''}`}
                onClick={() => applyProvider(provider)}
              >
                <strong>{provider.name}</strong>
                <small>{provider.hint}</small>
                {providerActive(provider) ? <Icon name="check" size={15} /> : null}
              </button>
            ))}
          </div>

          <div class="settings-form-grid">
            <Field label="Base URL" hint="OpenAI 兼容接口，通常以 /v1 结尾" required>
              <input class="input" value={baseUrl} onInput={(e) => setBaseUrl((e.target as HTMLInputElement).value)} />
            </Field>
            <Field label="模型" required>
              <input class="input" value={model} onInput={(e) => setModel((e.target as HTMLInputElement).value)} />
            </Field>
          </div>

          <Field
            label="API Key"
            hint={
              hasKey && !clearKey
                ? '已保存在本机。留空保持不变；输入新值可覆盖。'
                : '仅写入本机 .mediadesk/config.json（Unix 下权限 0600），不会回传到前端或任何第三方。'
            }
          >
            <div class="secret-input">
              <input
                class="input"
                type={showKey ? 'text' : 'password'}
                value={apiKey}
                placeholder={hasKey && !clearKey ? '••••••••••••（已保存）' : 'sk-…'}
                onInput={(e) => {
                  setApiKey((e.target as HTMLInputElement).value);
                  setClearKey(false);
                }}
                autoComplete="off"
              />
              <button type="button" class="icon-btn icon-btn-ghost" aria-label={showKey ? '隐藏密钥' : '显示密钥'} onClick={() => setShowKey(!showKey)}>
                <Icon name={showKey ? 'eye-off' : 'eye'} size={17} />
              </button>
              {hasKey ? (
                <button
                  type="button"
                  class="text-link danger-link"
                  onClick={() => {
                    setClearKey(!clearKey);
                    setApiKey('');
                  }}
                >
                  {clearKey ? '撤销清除' : '清除'}
                </button>
              ) : null}
            </div>
          </Field>

          <div class="settings-form-grid three">
            <Field label={`temperature · ${temperature.toFixed(1)}`} hint="越高越发散">
              <input
                type="range"
                min="0"
                max="2"
                step="0.1"
                value={temperature}
                onInput={(e) => setTemperature(Number((e.target as HTMLInputElement).value))}
              />
            </Field>
            <Field label="最大输出 tokens">
              <input
                class="input"
                type="number"
                min={16}
                max={32768}
                value={maxTokens}
                onInput={(e) => setMaxTokens(Number((e.target as HTMLInputElement).value))}
              />
            </Field>
            <Field label="超时（秒）">
              <input
                class="input"
                type="number"
                min={5}
                max={600}
                value={timeoutSecs}
                onInput={(e) => setTimeoutSecs(Number((e.target as HTMLInputElement).value))}
              />
            </Field>
          </div>

          {testResult ? (
            <div class={`test-result ${testResult.ok ? 'is-ok' : 'is-error'}`}>
              <Icon name={testResult.ok ? 'check' : 'alert'} size={16} />
              <div>
                <strong>{testResult.ok ? '连接成功' : '连接失败'}</strong>
                <p>{testResult.message}</p>
                {testResult.ok ? <small>{testResult.model} · {testResult.latencyMs}ms</small> : null}
              </div>
            </div>
          ) : null}
          {error ? <InlineError message={error} /> : null}

          <div class="settings-actions-row end">
            <Button variant="secondary" icon="spark" loading={testing} onClick={test}>
              测试连接
            </Button>
            <Button variant="primary" loading={saving} onClick={save} icon={savedFlash ? 'check' : undefined}>
              {savedFlash ? '已保存' : '保存设置'}
            </Button>
          </div>
          <p class="settings-note">
            密钥由本地 Rust 服务持有，浏览器前端只能看到「是否已配置」。生成失败时会展示服务商错误摘要，不会泄露密钥。
          </p>
        </section>

        <section class="panel settings-card">
          <div class="settings-card-head">
            <div>
              <p class="eyebrow">APPEARANCE</p>
              <h2>外观</h2>
            </div>
          </div>
          <div class="appearance-row">
            <Segmented<ThemeMode>
              options={[
                { value: 'light', label: '浅色', icon: 'sun' },
                { value: 'dark', label: '深色', icon: 'moon' },
                { value: 'system', label: '跟随系统', icon: 'monitor' },
              ]}
              value={theme}
              onChange={(value) => app.setTheme(value)}
            />
            <p class="muted-text">主题只影响界面，不写入工作区。</p>
          </div>
        </section>

        <section class="panel settings-card about-card">
          <div class="about-mark">媒</div>
          <div>
            <h2>媒材工坊 MediaDesk <span class="version-pill">v{app.data.workspace.version}</span></h2>
            <p>本地优先的自媒体素材工作台。想法、项目与五类素材都以普通文件保存，可被 VS Code 与任意 AI 读取。</p>
            <div class="about-points">
              <span><Icon name="check" size={13} /> 无账号、无遥测</span>
              <span><Icon name="check" size={13} /> 无外部字体与 CDN</span>
              <span><Icon name="check" size={13} /> 数据可随时迁走</span>
              <span><Icon name="check" size={13} /> Rust + WebView 低占用</span>
            </div>
          </div>
        </section>
      </div>

      <FolderBrowser
        open={folderOpen}
        initialPath={app.data.workspace.path}
        onSelect={switchWorkspace}
        onClose={() => setFolderOpen(false)}
      />
      <Modal open={switching} onClose={() => {}} title="正在切换工作区" size="sm" hideClose>
        <div class="folder-loading"><span class="spinner" /> 读取工作区文件…</div>
      </Modal>
    </div>
  );
}
