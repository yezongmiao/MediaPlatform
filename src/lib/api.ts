import type {
  Asset,
  AssistantChatResponse,
  AssistantHistory,
  BrowseInfo,
  CliConfig,
  ConfigView,
  GenerateRequest,
  GenerateResult,
  Idea,
  Kind,
  KernelStatus,
  LlmTestResult,
  MediaProfileView,
  Preset,
  PresetInput,
  Project,
  ProjectSummary,
  PromptResult,
  SearchResult,
  Skill,
  TextKernel,
  SkillInput,
  VscodeResult,
  WorkspaceInfo,
} from './types';

// These payload types live here so the API surface is explicit.
export interface CreateIdeaPayload {
  title: string;
  content?: string;
  tags?: string[];
}
export interface UpdateIdeaPayload {
  title?: string;
  content?: string;
  tags?: string[];
  status?: string;
}
export interface CreateProjectPayload {
  title: string;
  brief?: string;
  ideaIds?: string[];
  skillIds?: string[];
  presetIds?: string[];
}
export interface UpdateProjectPayload {
  title?: string;
  brief?: string;
  status?: string;
  tags?: string[];
  ideaIds?: string[];
  skillIds?: string[];
  presetIds?: string[];
}
export interface CreateAssetPayload {
  kind: Kind;
  title: string;
  content?: string;
  status?: string;
  tags?: string[];
  prompt?: string;
  source?: string;
}
export interface UpdateAssetPayload {
  title?: string;
  content?: string;
  status?: string;
  tags?: string[];
  prompt?: string;
}

export interface ApiErrorPayload {
  code: string;
  message: string;
  details?: unknown;
}

export class ApiError extends Error {
  code: string;
  status: number;
  details?: unknown;

  constructor(message: string, code = 'UNKNOWN', status = 0, details?: unknown) {
    super(message);
    this.name = 'ApiError';
    this.code = code;
    this.status = status;
    this.details = details;
  }
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  let response: Response;
  try {
    response = await fetch(path, init);
  } catch (err) {
    throw new ApiError(
      err instanceof Error ? `网络请求失败：${err.message}` : '网络请求失败',
      'NETWORK',
      0,
    );
  }
  if (response.status === 204) return undefined as T;
  const text = await response.text();
  let data: unknown = null;
  if (text) {
    try {
      data = JSON.parse(text);
    } catch {
      data = text;
    }
  }
  if (!response.ok) {
    const payload = (data as { error?: ApiErrorPayload } | null)?.error;
    throw new ApiError(
      payload?.message ?? `请求失败（HTTP ${response.status}）`,
      payload?.code ?? 'HTTP_ERROR',
      response.status,
      payload?.details ?? data,
    );
  }
  return data as T;
}

function jsonRequest<T>(path: string, method: string, body?: unknown): Promise<T> {
  return request<T>(path, {
    method,
    headers: body !== undefined ? { 'Content-Type': 'application/json' } : undefined,
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });
}

export interface LlmConfigPayload {
  baseUrl: string;
  model: string;
  temperature: number;
  maxTokens: number;
  timeoutSecs: number;
  apiKey?: string;
}

export interface TestLlmPayload {
  baseUrl?: string;
  model?: string;
  temperature?: number;
  maxTokens?: number;
  timeoutSecs?: number;
  apiKey?: string;
}

export interface KernelConfigPayload {
  kernel: TextKernel;
  binary?: string | null;
  args: string[];
  timeoutSecs: number;
}

export interface MediaProfilePayload {
  baseUrl: string;
  apiKey?: string;
  model: string;
  protocol: string;
  createPath: string;
  statusPath: string;
  style: string;
}

export interface MediaConfigPayload {
  illustration: MediaProfilePayload;
  video: MediaProfilePayload;
  timeoutSecs: number;
  pollIntervalSecs: number;
  videoTimeoutSecs: number;
}

export const api = {
  health: () => request<{ ok: boolean; version: string }>('/api/health'),

  getWorkspace: () => request<WorkspaceInfo>('/api/workspace'),
  switchWorkspace: (path: string) =>
    jsonRequest<WorkspaceInfo>('/api/workspace', 'POST', { path }),
  revealWorkspace: () => jsonRequest<{ ok: boolean }>('/api/workspace/reveal', 'POST', {}),
  browse: (path?: string) =>
    request<BrowseInfo>(`/api/fs/browse${path ? `?path=${encodeURIComponent(path)}` : ''}`),

  getConfig: () => request<ConfigView>('/api/config'),
  saveConfig: (payload: LlmConfigPayload) => jsonRequest<ConfigView>('/api/config', 'PUT', payload),
  testLlm: (payload: TestLlmPayload) => jsonRequest<LlmTestResult>('/api/llm/test', 'POST', payload),
  saveKernelConfig: (payload: KernelConfigPayload) =>
    jsonRequest<ConfigView>('/api/config/kernel', 'PUT', payload),
  getKernelStatus: () => request<KernelStatus>('/api/cli/status'),
  saveMediaConfig: (payload: MediaConfigPayload) =>
    jsonRequest<ConfigView>('/api/config/media', 'PUT', payload),
  previewPrompt: (payload: GenerateRequest) =>
    jsonRequest<PromptResult>('/api/llm/prompt', 'POST', payload),
  generate: (payload: GenerateRequest) =>
    jsonRequest<GenerateResult>('/api/llm/generate', 'POST', payload),

  listIdeas: () => request<Idea[]>('/api/ideas'),
  createIdea: (payload: CreateIdeaPayload) => jsonRequest<Idea>('/api/ideas', 'POST', payload),
  updateIdea: (id: string, payload: UpdateIdeaPayload) =>
    jsonRequest<Idea>(`/api/ideas/${id}`, 'PUT', payload),
  deleteIdea: (id: string) => jsonRequest<void>(`/api/ideas/${id}`, 'DELETE'),
  promoteIdea: (id: string, payload: { title?: string; brief?: string }) =>
    jsonRequest<Project>(`/api/ideas/${id}/promote`, 'POST', payload),

  listProjects: () => request<ProjectSummary[]>('/api/projects'),
  createProject: (payload: CreateProjectPayload) =>
    jsonRequest<Project>('/api/projects', 'POST', payload),
  getProject: (id: string) => request<Project>(`/api/projects/${id}`),
  updateProject: (id: string, payload: UpdateProjectPayload) =>
    jsonRequest<Project>(`/api/projects/${id}`, 'PUT', payload),
  deleteProject: (id: string) => jsonRequest<void>(`/api/projects/${id}`, 'DELETE'),
  rescanProject: (id: string) => jsonRequest<Project>(`/api/projects/${id}/rescan`, 'POST', {}),
  openVscode: (id: string) => jsonRequest<VscodeResult>(`/api/projects/${id}/vscode`, 'POST', {}),
  prepareContext: (id: string) =>
    jsonRequest<{ projectPath: string; filesWritten: string[] }>(
      `/api/projects/${id}/context`,
      'POST',
      {},
    ),

  createAsset: (projectId: string, payload: CreateAssetPayload) =>
    jsonRequest<Asset>(`/api/projects/${projectId}/assets`, 'POST', payload),
  updateAsset: (projectId: string, assetId: string, payload: UpdateAssetPayload) =>
    jsonRequest<Asset>(`/api/projects/${projectId}/assets/${assetId}`, 'PUT', payload),
  deleteAsset: (projectId: string, assetId: string) =>
    jsonRequest<void>(`/api/projects/${projectId}/assets/${assetId}`, 'DELETE'),

  uploadAsset: async (
    projectId: string,
    kind: Kind,
    file: File,
    opts: { assetId?: string; title?: string } = {},
  ): Promise<Asset> => {
    const params = new URLSearchParams({ kind, filename: file.name });
    if (opts.assetId) params.set('assetId', opts.assetId);
    if (opts.title) params.set('title', opts.title);
    const response = await fetch(`/api/projects/${projectId}/uploads?${params.toString()}`, {
      method: 'POST',
      headers: { 'Content-Type': file.type || 'application/octet-stream' },
      body: file,
    });
    const text = await response.text();
    const data = text ? JSON.parse(text) : null;
    if (!response.ok) {
      const payload = (data as { error?: ApiErrorPayload } | null)?.error;
      throw new ApiError(
        payload?.message ?? `上传失败（HTTP ${response.status}）`,
        payload?.code ?? 'UPLOAD_FAILED',
        response.status,
        payload,
      );
    }
    return data as Asset;
  },

  getAssistantHistory: (projectId: string) =>
    request<AssistantHistory>(`/api/projects/${projectId}/chat`),
  chatAssistant: (projectId: string, payload: { message: string; mode: 'suggest' | 'apply' }) =>
    jsonRequest<AssistantChatResponse>(`/api/projects/${projectId}/chat`, 'POST', payload),
  clearAssistantHistory: (projectId: string) =>
    jsonRequest<void>(`/api/projects/${projectId}/chat`, 'DELETE'),

  listSkills: () => request<Skill[]>('/api/skills'),
  createSkill: (payload: SkillInput) => jsonRequest<Skill>('/api/skills', 'POST', payload),
  updateSkill: (id: string, payload: SkillInput) =>
    jsonRequest<Skill>(`/api/skills/${id}`, 'PUT', payload),
  deleteSkill: (id: string) => jsonRequest<void>(`/api/skills/${id}`, 'DELETE'),

  listPresets: () => request<Preset[]>('/api/presets'),
  createPreset: (payload: PresetInput) => jsonRequest<Preset>('/api/presets', 'POST', payload),
  updatePreset: (id: string, payload: PresetInput) =>
    jsonRequest<Preset>(`/api/presets/${id}`, 'PUT', payload),
  deletePreset: (id: string) => jsonRequest<void>(`/api/presets/${id}`, 'DELETE'),

  search: (q: string) => request<SearchResult>(`/api/search?q=${encodeURIComponent(q)}`),
};
