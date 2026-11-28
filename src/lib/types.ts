export type Kind = 'article' | 'script' | 'illustration' | 'card' | 'video';

export interface KindMeta {
  id: Kind;
  label: string;
  plural: string;
  dir: string;
  description: string;
  accent: string;
  action: string;
  upload: boolean;
  accept?: string;
}

export const KINDS: KindMeta[] = [
  {
    id: 'article',
    label: '文章',
    plural: '文章',
    dir: 'article',
    description: '长文、公众号、专栏',
    accent: 'var(--kind-article)',
    action: '新建文章',
    upload: false,
  },
  {
    id: 'script',
    label: '脚本',
    plural: '脚本',
    dir: 'script',
    description: '口播、分镜、旁白',
    accent: 'var(--kind-script)',
    action: '新建脚本',
    upload: false,
  },
  {
    id: 'illustration',
    label: '插图',
    plural: '插图',
    dir: 'illustration',
    description: '配图与图像提示词',
    accent: 'var(--kind-illustration)',
    action: '上传插图',
    upload: true,
    accept: 'image/png,image/jpeg,image/webp,image/gif,image/avif',
  },
  {
    id: 'card',
    label: '文卡图',
    plural: '文卡图',
    dir: 'card',
    description: '可保存的短句卡片',
    accent: 'var(--kind-card)',
    action: 'AI 文卡图',
    upload: true,
    accept: 'image/png,image/jpeg,image/webp,image/gif,image/avif',
  },
  {
    id: 'video',
    label: '视频',
    plural: '视频',
    dir: 'video',
    description: '成片、素材与分镜',
    accent: 'var(--kind-video)',
    action: '上传视频',
    upload: true,
    accept: 'video/mp4,video/webm,video/quicktime,video/x-m4v',
  },
];

export const KIND_MAP = Object.fromEntries(KINDS.map((k) => [k.id, k])) as Record<Kind, KindMeta>;

export type IdeaStatus = 'inbox' | 'promoted' | 'archived';
export type ProjectStatus = 'active' | 'paused' | 'done' | 'archived';
export type AssetStatus = 'draft' | 'review' | 'ready' | 'published';

export const IDEA_STATUS: Record<IdeaStatus, { label: string; color: string }> = {
  inbox: { label: '待立项', color: 'neutral' },
  promoted: { label: '已立项', color: 'jade' },
  archived: { label: '已归档', color: 'muted' },
};

export const PROJECT_STATUS: Record<ProjectStatus, { label: string; color: string }> = {
  active: { label: '进行中', color: 'accent' },
  paused: { label: '已暂停', color: 'gold' },
  done: { label: '已完成', color: 'jade' },
  archived: { label: '已归档', color: 'muted' },
};

export const ASSET_STATUS: Record<AssetStatus, { label: string; color: string }> = {
  draft: { label: '草稿', color: 'neutral' },
  review: { label: '待审', color: 'gold' },
  ready: { label: '可发布', color: 'jade' },
  published: { label: '已发布', color: 'accent' },
};

export interface Idea {
  id: string;
  title: string;
  status: IdeaStatus;
  tags: string[];
  createdAt: string;
  updatedAt: string;
  projectId?: string;
  content: string;
  excerpt: string;
  relPath: string;
}

export interface ProjectMeta {
  id: string;
  title: string;
  status: ProjectStatus;
  tags: string[];
  createdAt: string;
  updatedAt: string;
  ideaIds: string[];
  skillIds: string[];
  presetIds: string[];
}

export interface ProjectSummary extends ProjectMeta {
  briefExcerpt: string;
  dirName: string;
  counts: Record<Kind, number>;
  coverUrl?: string;
}

export interface AssetMeta {
  id: string;
  kind: Kind;
  title: string;
  status: AssetStatus;
  tags: string[];
  createdAt: string;
  updatedAt: string;
  file?: string;
  prompt?: string;
  source?: 'manual' | 'llm' | 'media-gen';
  cardGroupId?: string;
  cardGroupTitle?: string;
  cardGroupIndex?: number;
  cardGroupTotal?: number;
}

export interface Asset extends AssetMeta {
  content: string;
  fileUrl?: string;
  relPath: string;
  projectId: string;
}

export type LibraryActionType =
  | 'createSkill'
  | 'updateSkill'
  | 'deleteSkill'
  | 'createPreset'
  | 'updatePreset'
  | 'deletePreset';

export interface LibraryAction {
  type: LibraryActionType;
  name?: string;
  id?: string;
  description?: string;
  appliesTo?: Kind[];
  kind?: Kind;
  skillIds?: string[];
  content?: string;
  code?: SkillCode[];
}

export interface LibraryActionResult {
  action: string;
  ok: boolean;
  message: string;
}

export interface LibraryAssistantMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  createdAt: string;
  actions?: LibraryAction[];
  applied?: boolean;
}

export interface LibraryAssistantHistory {
  messages: LibraryAssistantMessage[];
  kernel: string;
}

export interface LibraryAssistantResponse {
  reply: string;
  actions: LibraryAction[];
  results: LibraryActionResult[];
  warnings: string[];
  applied: boolean;
  kernel: string;
  model: string;
  latencyMs: number;
  messages: LibraryAssistantMessage[];
}

export type LibraryAssistantStreamEvent =
  | { type: 'status'; message: string }
  | { type: 'delta'; text: string }
  | { type: 'done'; response: LibraryAssistantResponse }
  | { type: 'error'; code: string; message: string };

export interface Project extends ProjectMeta {
  brief: string;
  dirName: string;
  assets: Record<Kind, Asset[]>;
  counts: Record<Kind, number>;
  coverUrl?: string;
}

export interface SkillCode {
  name: string;
  language: string;
  content: string;
}

export interface Skill {
  id: string;
  name: string;
  description: string;
  appliesTo: Kind[];
  content: string;
  code?: SkillCode[];
}

export interface Preset {
  id: string;
  name: string;
  description: string;
  kind: Kind;
  skillIds: string[];
  content: string;
}

export interface RequestHeader {
  name: string;
  value: string;
}

export interface LlmConfigView {
  providerId: string;
  baseUrl: string;
  model: string;
  temperature: number;
  maxTokens: number;
  timeoutSecs: number;
  hasApiKey: boolean;
  protocol: string;
  customHeaders: RequestHeader[];
}

export interface ProviderSpec {
  id: string;
  name: string;
  description: string;
  mark: string;
  accent: string;
  baseUrl: string;
  protocol: string;
  defaultModel: string;
  modelsPath: string;
  auth: string;
  requiresSession: boolean;
  note: string;
  docsUrl: string;
  fixedHeaders: RequestHeader[];
  fallbackModels: string[];
}

export interface ModelInfo {
  id: string;
  name?: string;
}

export interface ModelListResponse {
  providerId: string;
  models: ModelInfo[];
  fetched: boolean;
  note?: string;
  docsUrl?: string;
}

export type TextKernel = 'api' | 'claude' | 'codex' | 'opencode' | 'deepseek';

export interface CliConfig {
  binary?: string;
  args: string[];
  timeoutSecs: number;
}

export interface MediaProfileView {
  baseUrl: string;
  model: string;
  protocol: string;
  createPath: string;
  statusPath: string;
  style: string;
  hasApiKey: boolean;
}

export interface MediaGenView {
  illustration: MediaProfileView;
  video: MediaProfileView;
  timeoutSecs: number;
  pollIntervalSecs: number;
  videoTimeoutSecs: number;
}

export interface ConfigView {
  version: number;
  llm: LlmConfigView;
  appearance: { theme: string };
  textKernel: TextKernel;
  cli: CliConfig;
  mediaGen: MediaGenView;
}

export interface KernelInfo {
  id: 'claude' | 'codex' | 'opencode' | 'deepseek';
  label: string;
  found: boolean;
  path?: string;
  installHint: string;
}

export interface KernelStatus {
  selected: TextKernel;
  envOverride: boolean;
  kernels: KernelInfo[];
}

export interface WorkspaceInfo {
  path: string;
  ideas: number;
  projects: number;
  assets: number;
  bytes: number;
  version: string;
}

export interface BrowseInfo {
  path: string;
  parent?: string;
  dirs: { name: string; path: string }[];
  isWorkspace: boolean;
}

export interface DirEntryInfo {
  name: string;
  path: string;
}

export interface Usage {
  promptTokens: number;
  completionTokens: number;
  totalTokens: number;
}

export interface PromptResult {
  prompt: string;
  system: string;
  user: string;
  kind: Kind;
  preset?: { id: string; name: string };
  skills: { id: string; name: string }[];
}

export interface GenerateResult {
  content: string;
  model: string;
  usage: Usage;
  latencyMs: number;
  prompt: string;
  presetId?: string;
  skillIds: string[];
  snapshotPath?: string;
  kernel: string;
  asset?: Asset;
}

export interface GenerateRequest {
  projectId?: string;
  ideaIds?: string[];
  kind: Kind;
  presetId?: string | null;
  skillIds?: string[];
  instruction: string;
}

export interface LlmTestResult {
  ok: boolean;
  message: string;
  latencyMs: number;
  model: string;
}

export interface VscodeResult {
  ok: boolean;
  command: string;
  projectPath: string;
  filesWritten: string[];
  message: string;
}

export interface DshResult {
  ok: boolean;
  url: string;
  workspace: string;
  command: string;
  filesWritten: string[];
  message: string;
}

export interface FolderResult {
  ok: boolean;
  path: string;
  message: string;
}

export interface SearchResult {
  query: string;
  ideas: Idea[];
  projects: ProjectSummary[];
  assets: { projectId: string; projectTitle: string; asset: Asset }[];
}

export type ThemeMode = 'light' | 'dark' | 'system';

export interface SkillInput {
  name: string;
  description: string;
  appliesTo: string[];
  content: string;
  code: SkillCode[];
}

export interface PresetInput {
  name: string;
  description: string;
  kind: Kind;
  skillIds: string[];
  content: string;
}


export type AssistantMode = 'suggest' | 'apply';

export interface AssistantEdit {
  assetId: string;
  title?: string;
  content?: string;
  summary?: string;
}

export interface AssistantCreate {
  assetId?: string;
  kind: Kind;
  title: string;
  content: string;
  summary?: string;
}

export interface AssistantMessage {
  id: string;
  role: 'user' | 'assistant';
  content: string;
  createdAt: string;
  edits?: AssistantEdit[];
  newAssets?: AssistantCreate[];
  applied?: boolean;
}

export interface AssistantHistory {
  messages: AssistantMessage[];
  kernel: TextKernel;
}

export interface AssistantStatus {
  running: boolean;
  elapsedSecs?: number;
}

export interface AssistantGroup {
  groupId?: string;
  title: string;
  assetIds: string[];
}

export interface AssistantChatResponse {
  reply: string;
  edits: AssistantEdit[];
  newAssets?: AssistantCreate[];
  groups?: AssistantGroup[];
  applied: boolean;
  kernel: string;
  model: string;
  latencyMs: number;
  warnings: string[];
  messages: AssistantMessage[];
}

export interface AssistantApplyResponse {
  applied: boolean;
  warnings: string[];
  messages: AssistantMessage[];
}

export interface CardPlan {
  kicker: string;
  page: string;
  title: string;
  body: string[];
  footnote: string;
  source: string;
}

export interface GeneratedCard {
  plan: CardPlan;
}

export interface CardPlanResponse {
  cards: GeneratedCard[];
  model: string;
  latencyMs: number;
  sourceChars: number;
  targetCount: number;
  warning?: string;
}

export type AssistantStreamEvent =
  | { type: 'status'; message: string }
  | { type: 'delta'; text: string }
  | { type: 'done'; response: AssistantChatResponse }
  | { type: 'error'; code: string; message: string };
