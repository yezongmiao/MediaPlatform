import { createContext } from 'preact';
import { useContext } from 'preact/hooks';
import type { ConfigView, Idea, Preset, ProjectSummary, Skill, ThemeMode, WorkspaceInfo } from './types';

export interface AppData {
  workspace: WorkspaceInfo;
  config: ConfigView;
  ideas: Idea[];
  projects: ProjectSummary[];
  skills: Skill[];
  presets: Preset[];
}

export interface ToastInput {
  message: string;
  tone?: 'default' | 'success' | 'error';
  duration?: number;
}

export interface AppContextValue {
  data: AppData;
  loading: boolean;
  refreshAll: () => Promise<void>;
  refreshIdeas: () => Promise<void>;
  refreshProjects: () => Promise<void>;
  refreshLibrary: () => Promise<void>;
  refreshWorkspace: () => Promise<void>;
  refreshConfig: () => Promise<void>;
  toast: (input: ToastInput | string) => void;
  theme: ThemeMode;
  setTheme: (theme: ThemeMode) => void;
  openQuickCapture: () => void;
  openCommandPalette: () => void;
}

export const AppContext = createContext<AppContextValue | null>(null);

export function useApp(): AppContextValue {
  const value = useContext(AppContext);
  if (!value) throw new Error('useApp must be used inside AppContext');
  return value;
}
