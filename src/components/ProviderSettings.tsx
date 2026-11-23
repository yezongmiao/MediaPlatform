import { useEffect, useLayoutEffect, useRef, useState } from 'preact/hooks';
import { api, ApiError } from '../lib/api';
import { useApp } from '../lib/app-context';
import type { ModelInfo, ProviderSpec, RequestHeader } from '../lib/types';
import { Button, Field, InlineError } from './ui';
import { Icon } from './Icon';

interface HeaderRow extends RequestHeader {
  key: number;
}

let headerKey = 0;

function rowsFrom(headers: RequestHeader[] | undefined): HeaderRow[] {
  return (headers ?? []).map((header) => ({ ...header, key: ++headerKey }));
}

function protocolLabel(protocol: string): string {
  if (protocol === 'anthropic') return 'Anthropic Messages';
  if (protocol === 'responses') return 'OpenAI Responses';
  return 'OpenAI Chat';
}

function authLabel(auth: string): string {
  if (auth === 'x-api-key') return 'x-api-key';
  if (auth === 'none') return '无需 Key';
  return 'Bearer';
}

export function ProviderSettings() {
  const app = useApp();
  const config = app.data.config;
  const llm = config.llm;
  const [providers, setProviders] = useState<ProviderSpec[]>([]);
  const [catalogError, setCatalogError] = useState('');
  const [providerId, setProviderId] = useState(llm.providerId || 'custom');
  const [openId, setOpenId] = useState<string | null>(llm.providerId || 'custom');
  const [baseUrl, setBaseUrl] = useState(llm.baseUrl);
  const [model, setModel] = useState(llm.model);
  const [apiKey, setApiKey] = useState('');
  const [hasKey, setHasKey] = useState(llm.hasApiKey);
  const [clearKey, setClearKey] = useState(false);
  const [showKey, setShowKey] = useState(false);
  const [protocol, setProtocol] = useState(llm.protocol || 'openai');
  const [headers, setHeaders] = useState<HeaderRow[]>(() => rowsFrom(llm.customHeaders));
  const [temperature, setTemperature] = useState(llm.temperature);
  const [maxTokens, setMaxTokens] = useState(llm.maxTokens);
  const [timeoutSecs, setTimeoutSecs] = useState(llm.timeoutSecs);
  const [models, setModels] = useState<ModelInfo[]>([]);
  const [modelsOpen, setModelsOpen] = useState(false);
  const [modelsLoading, setModelsLoading] = useState(false);
  const [modelsError, setModelsError] = useState('');
  const [modelQuery, setModelQuery] = useState('');
  const [testing, setTesting] = useState(false);
  const [testResult, setTestResult] = useState<{ ok: boolean; message: string; model?: string; latencyMs?: number } | null>(null);
  const [saving, setSaving] = useState(false);
  const [savedFlash, setSavedFlash] = useState(false);
  const [error, setError] = useState('');
  const [search, setSearch] = useState('');
  const synced = useRef('');

  useEffect(() => {
    let cancelled = false;
    api
      .listProviders()
      .then((data) => {
        if (!cancelled) setProviders(data);
      })
      .catch((err) => {
        if (!cancelled) setCatalogError(err instanceof ApiError ? err.message : '无法读取服务商目录');
      });
    return () => {
      cancelled = true;
    };
  }, []);

  useLayoutEffect(() => {
    const signature = JSON.stringify([
      llm.providerId,
      llm.baseUrl,
      llm.model,
      llm.temperature,
      llm.maxTokens,
      llm.timeoutSecs,
      llm.hasApiKey,
      llm.protocol,
      llm.customHeaders,
    ]);
    if (signature === synced.current) return;
    synced.current = signature;
    setProviderId(llm.providerId || 'custom');
    setBaseUrl(llm.baseUrl);
    setModel(llm.model);
    setProtocol(llm.protocol || 'openai');
    setHeaders(rowsFrom(llm.customHeaders));
    setTemperature(llm.temperature);
    setMaxTokens(llm.maxTokens);
    setTimeoutSecs(llm.timeoutSecs);
    setHasKey(llm.hasApiKey);
    setApiKey('');
    setClearKey(false);
  }, [llm]);

  const activeProvider = providers.find((provider) => provider.id === providerId) ?? providers[providers.length - 1];
  const openProvider = providers.find((provider) => provider.id === openId) ?? null;

  const applyProvider = (provider: ProviderSpec) => {
    const isSaved = provider.id === (llm.providerId || 'custom');
    setProviderId(provider.id);
    setProtocol(isSaved ? llm.protocol : provider.protocol);
    setBaseUrl(isSaved ? llm.baseUrl : provider.baseUrl);
    setModel(isSaved ? llm.model : provider.defaultModel);
    setHeaders(rowsFrom(isSaved ? llm.customHeaders : []));
    setHasKey(isSaved ? llm.hasApiKey : false);
    setApiKey('');
    setClearKey(false);
    setModels([]);
    setModelsOpen(false);
    setModelsError('');
    setTestResult(null);
    setError('');
  };

  const toggleProvider = (provider: ProviderSpec) => {
    if (openId === provider.id) {
      setOpenId(null);
      return;
    }
    setOpenId(provider.id);
    if (provider.id !== providerId) applyProvider(provider);
  };

  const updateHeader = (key: number, field: 'name' | 'value', value: string) => {
    setHeaders((prev) =>
      prev.map((header) => (header.key === key ? { ...header, [field]: value } : header)),
    );
  };

  const addHeader = () => {
    setHeaders((prev) => [...prev, { key: ++headerKey, name: '', value: '' }]);
  };

  const removeHeader = (key: number) => {
    setHeaders((prev) => prev.filter((header) => header.key !== key));
  };

  const payloadHeaders = (): RequestHeader[] => {
    const result: RequestHeader[] = [];
    for (const header of headers) {
      const name = header.name.trim();
      const value = header.value.trim();
      if (!name && !value) continue;
      if (!name || !value) throw new Error('每个请求头都需要填写名称和值；空行可以直接删除。');
      result.push({ name, value });
    }
    return result;
  };

  const normalizedBase = (value: string) => value.trim().replace(/\/+$/, '');
  const baseChanged = normalizedBase(baseUrl) !== normalizedBase(llm.baseUrl);

  const keyPayload = (): string | undefined => {
    if (clearKey) return '';
    if (apiKey.trim()) return apiKey.trim();
    if (providerId !== (llm.providerId || 'custom') || baseChanged) return '';
    return undefined;
  };

  const validate = () => {
    if (!baseUrl.trim()) {
      setError('请填写 Base URL');
      return false;
    }
    if (!model.trim()) {
      setError('请选择或填写模型 ID');
      return false;
    }
    return true;
  };

  const fetchModels = async () => {
    setModelsLoading(true);
    setModelsError('');
    setModelsOpen(true);
    try {
      const data = await api.listModels({
        providerId,
        baseUrl,
        apiKey: apiKey.trim() || undefined,
        protocol,
        customHeaders: payloadHeaders(),
      });
      setModels(data.models);
      setModelQuery('');
    } catch (err) {
      setModels([]);
      setModelsError(err instanceof ApiError ? err.message : '获取模型列表失败');
    } finally {
      setModelsLoading(false);
    }
  };

  const save = async () => {
    if (!validate()) return;
    setSaving(true);
    setError('');
    try {
      const updated = await api.saveConfig({
        providerId,
        baseUrl,
        model,
        temperature,
        maxTokens,
        timeoutSecs,
        apiKey: keyPayload(),
        protocol,
        customHeaders: payloadHeaders(),
      });
      setHasKey(updated.llm.hasApiKey);
      setApiKey('');
      setClearKey(false);
      setSavedFlash(true);
      window.setTimeout(() => setSavedFlash(false), 1800);
      await app.refreshConfig();
      app.toast({ message: '大模型设置已保存', tone: 'success' });
    } catch (err) {
      setError(err instanceof ApiError ? err.message : err instanceof Error ? err.message : '保存失败');
    } finally {
      setSaving(false);
    }
  };

  const test = async () => {
    if (!validate()) return;
    setTesting(true);
    setTestResult(null);
    setError('');
    try {
      const result = await api.testLlm({
        providerId,
        baseUrl,
        model,
        temperature,
        maxTokens,
        timeoutSecs,
        apiKey: keyPayload(),
        protocol,
        customHeaders: payloadHeaders(),
      });
      setTestResult(result);
      app.toast({ message: `连接成功 · ${result.latencyMs}ms`, tone: 'success' });
    } catch (err) {
      setTestResult({
        ok: false,
        message: err instanceof ApiError ? err.message : '连接测试失败',
      });
    } finally {
      setTesting(false);
    }
  };

  const filteredModels = models.filter((item) => {
    if (!modelQuery.trim()) return true;
    const query = modelQuery.toLowerCase();
    return item.id.toLowerCase().includes(query) || (item.name ?? '').toLowerCase().includes(query);
  });

  const visibleProviders = providers.filter((provider) => {
    if (!search.trim()) return true;
    const query = search.toLowerCase();
    return (
      provider.name.toLowerCase().includes(query) ||
      provider.id.toLowerCase().includes(query) ||
      provider.description.toLowerCase().includes(query)
    );
  });

  return (
    <section class="panel settings-card provider-settings-card">
      <div class="settings-card-head">
        <div>
          <p class="eyebrow">PROVIDERS · 服务商</p>
          <h2>大模型接入</h2>
        </div>
        <span class={`settings-badge ${hasKey || activeProvider?.auth === 'none' ? 'is-ok' : ''}`}>
          <span class="status-dot" />
          {activeProvider?.name ?? '自定义'}
          {hasKey ? ' · 已配置' : ''}
        </span>
      </div>

      <div class="provider-intro">
        <p>选择一个服务商展开配置，其余保持折叠。不同服务商可能需要特殊请求头或协议，MediaDesk 会自动处理已内置的差异。</p>
        <div class="provider-search">
          <Icon name="search" size={14} />
          <input
            value={search}
            placeholder="搜索服务商"
            aria-label="搜索服务商"
            onInput={(e) => setSearch((e.target as HTMLInputElement).value)}
          />
        </div>
      </div>

      {catalogError ? <InlineError message={catalogError} /> : null}

      <div class="provider-accordion">
        {visibleProviders.map((provider) => {
          const isOpen = openId === provider.id;
          const isSelected = providerId === provider.id;
          const isSaved = (llm.providerId || 'custom') === provider.id;
          return (
            <div class={`provider-entry ${isOpen ? 'is-open' : ''} ${isSelected ? 'is-selected' : ''}`} key={provider.id}>
              <button
                class="provider-row"
                aria-expanded={isOpen}
                onClick={() => toggleProvider(provider)}
              >
                <span class="provider-mark" style={{ background: `${provider.accent}16`, color: provider.accent }}>
                  {provider.mark}
                </span>
                <span class="provider-row-main">
                  <span class="provider-row-title">
                    <strong>{provider.name}</strong>
                    {isSaved ? <span class="provider-saved">已保存</span> : null}
                    {isSelected && !isSaved ? <span class="provider-unsaved">待保存</span> : null}
                  </span>
                  <small>{provider.description}</small>
                </span>
                <span class="provider-row-state">
                  {isSelected && model && isOpen ? <code>{model}</code> : null}
                  <Icon name="chevron-down" size={15} />
                </span>
              </button>

              {isOpen ? (
                <div class="provider-panel">
                  <div class="provider-note">
                    <Icon name={provider.requiresSession ? 'alert' : 'check'} size={15} />
                    <p>{provider.note || '使用该服务商的默认接入方式。'}</p>
                  </div>

                  <div class="provider-meta-row">
                    <span><small>协议</small><strong>{protocolLabel(provider.id === 'custom' ? protocol : provider.protocol)}</strong></span>
                    <span><small>认证</small><strong>{authLabel(provider.auth)}</strong></span>
                    {provider.fixedHeaders.length ? (
                      <span class="provider-fixed-headers">
                        <small>固定请求头</small>
                        <span>{provider.fixedHeaders.map((header) => <code key={header.name}>{header.name}</code>)}</span>
                      </span>
                    ) : null}
                    {provider.requiresSession ? (
                      <span class="provider-fixed-headers">
                        <small>会话</small>
                        <span><code>x-opencode-session</code></span>
                      </span>
                    ) : null}
                  </div>

                  <div class="settings-form-grid">
                    <Field label="Base URL" hint="通常以 /v1 结尾" required>
                      <input
                        class="input"
                        value={baseUrl}
                        placeholder="https://..."
                        onInput={(e) => setBaseUrl((e.target as HTMLInputElement).value)}
                      />
                    </Field>
                    <Field label="模型 ID" required>
                      <div class="provider-model-input">
                        <input
                          class="input"
                          value={model}
                          placeholder={provider.defaultModel || '先获取列表或手动填写'}
                          onInput={(e) => setModel((e.target as HTMLInputElement).value)}
                        />
                        <Button
                          size="sm"
                          variant="secondary"
                          icon="refresh"
                          loading={modelsLoading}
                          onClick={fetchModels}
                        >
                          获取列表
                        </Button>
                      </div>
                    </Field>
                  </div>

                  {modelsOpen ? (
                    <div class="model-picker">
                      <div class="model-picker-head">
                        <div>
                          <strong>{modelsLoading ? '正在获取模型…' : `模型列表 · ${models.length}`}</strong>
                          <span>来自 {provider.name} 的 /models</span>
                        </div>
                        <button class="icon-btn icon-btn-ghost" aria-label="关闭模型列表" onClick={() => setModelsOpen(false)}>
                          <Icon name="close" size={15} />
                        </button>
                      </div>
                      {models.length ? (
                        <>
                          <input
                            class="input model-search"
                            value={modelQuery}
                            placeholder="筛选模型"
                            aria-label="筛选模型"
                            onInput={(e) => setModelQuery((e.target as HTMLInputElement).value)}
                          />
                          <div class="model-options">
                            {filteredModels.map((item) => (
                              <button
                                class={`model-option ${model === item.id ? 'is-active' : ''}`}
                                key={item.id}
                                onClick={() => {
                                  setModel(item.id);
                                  setModelsOpen(false);
                                  setTestResult(null);
                                }}
                              >
                                <span>
                                  <strong>{item.name || item.id}</strong>
                                  {item.name ? <small>{item.id}</small> : null}
                                </span>
                                {model === item.id ? <Icon name="check" size={14} /> : null}
                              </button>
                            ))}
                            {!filteredModels.length ? <p class="muted-text">没有匹配的模型。</p> : null}
                          </div>
                        </>
                      ) : modelsError ? (
                        <div class="model-picker-error">
                          <InlineError message={modelsError} onRetry={fetchModels} />
                          <span class="muted-text">也可以直接在上方手动填写模型 ID。</span>
                        </div>
                      ) : modelsLoading ? (
                        <div class="model-picker-loading"><span class="spinner" /> 正在读取模型…</div>
                      ) : null}
                    </div>
                  ) : null}

                  <Field
                    label="API Key"
                    hint={
                      provider.auth === 'none'
                        ? '本地服务不需要 Key。'
                        : hasKey && !clearKey && providerId === (llm.providerId || 'custom')
                          ? '已保存在本机。留空保持不变；输入新值可覆盖。'
                          : '只保存在本机；切换服务商或修改 Base URL 后需要重新填写。'
                    }
                  >
                    <div class="secret-input">
                      <input
                        class="input"
                        type={showKey ? 'text' : 'password'}
                        value={apiKey}
                        placeholder={
                          hasKey && !clearKey && providerId === (llm.providerId || 'custom')
                            ? '••••••••••••（已保存）'
                            : provider.auth === 'none'
                              ? '不需要'
                              : 'sk-…'
                        }
                        disabled={provider.auth === 'none'}
                        onInput={(e) => {
                          setApiKey((e.target as HTMLInputElement).value);
                          setClearKey(false);
                        }}
                        autoComplete="off"
                      />
                      <button type="button" class="icon-btn icon-btn-ghost" aria-label={showKey ? '隐藏密钥' : '显示密钥'} onClick={() => setShowKey(!showKey)}>
                        <Icon name={showKey ? 'eye-off' : 'eye'} size={17} />
                      </button>
                      {hasKey && providerId === (llm.providerId || 'custom') ? (
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

                  <details class="provider-advanced">
                    <summary>
                      <Icon name="sliders" size={15} />
                      高级：协议与自定义请求头
                      <span>{headers.length ? `${headers.length} 个` : ''}</span>
                    </summary>
                    <div class="provider-advanced-body">
                      {provider.id === 'custom' ? (
                        <Field label="协议" hint="按目标服务端选择请求格式">
                          <select class="select" value={protocol} onChange={(e) => setProtocol((e.target as HTMLSelectElement).value)}>
                            <option value="openai">OpenAI Chat Completions</option>
                            <option value="anthropic">Anthropic Messages</option>
                            <option value="responses">OpenAI Responses</option>
                          </select>
                        </Field>
                      ) : (
                        <p class="muted-text">
                          该服务商固定使用 <strong>{protocolLabel(provider.protocol)}</strong>；如需其他协议，请选择「自定义服务」。
                        </p>
                      )}
                      <div class="provider-header-head">
                        <div>
                          <strong>自定义请求头</strong>
                          <p>会附加到模型请求；同名头会覆盖内置值。Authorization 等敏感值只存在本机。</p>
                        </div>
                        <Button size="sm" variant="secondary" icon="plus" onClick={addHeader}>
                          添加
                        </Button>
                      </div>
                      {headers.length ? (
                        <div class="provider-header-list">
                          {headers.map((header) => (
                            <div class="provider-header-row" key={header.key}>
                              <input
                                class="input"
                                value={header.name}
                                placeholder="请求头名称"
                                aria-label="自定义请求头名称"
                                onInput={(e) => updateHeader(header.key, 'name', (e.target as HTMLInputElement).value)}
                              />
                              <input
                                class="input"
                                value={header.value}
                                placeholder="值"
                                aria-label="自定义请求头值"
                                onInput={(e) => updateHeader(header.key, 'value', (e.target as HTMLInputElement).value)}
                              />
                              <button
                                type="button"
                                class="icon-btn icon-btn-ghost"
                                aria-label="删除自定义请求头"
                                onClick={() => removeHeader(header.key)}
                              >
                                <Icon name="trash" size={15} />
                              </button>
                            </div>
                          ))}
                        </div>
                      ) : (
                        <p class="muted-text">没有自定义请求头。OpenCode Go 等特殊头已由内置服务商配置自动处理。</p>
                      )}
                    </div>
                  </details>

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
                </div>
              ) : null}
            </div>
          );
        })}
        {!providers.length && !catalogError ? (
          <div class="provider-loading"><span class="spinner" /> 正在读取服务商目录…</div>
        ) : null}
      </div>

      {error ? <InlineError message={error} /> : null}
      <div class="settings-actions-row end provider-actions">
        <Button variant="secondary" icon="spark" loading={testing} onClick={test}>
          测试连接
        </Button>
        <Button variant="primary" icon={savedFlash ? 'check' : undefined} loading={saving} onClick={save}>
          {savedFlash ? '已保存' : '保存设置'}
        </Button>
      </div>
      <p class="settings-note">
        密钥由本地 Rust 服务持有，前端只能看到「是否已配置」。模型列表通过本地服务请求，避免浏览器跨域；服务商特殊请求头（如 OpenCode Go 的 x-opencode-session）会自动附加。
      </p>
    </section>
  );
}
