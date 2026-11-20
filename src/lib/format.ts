import { ASSET_STATUS, IDEA_STATUS, KIND_MAP, PROJECT_STATUS } from './types';
import type { AssetStatus, IdeaStatus, Kind, ProjectStatus } from './types';

export function formatDate(input: string, withTime = false): string {
  if (!input) return '—';
  const date = new Date(input);
  if (Number.isNaN(date.getTime())) return '—';
  return new Intl.DateTimeFormat('zh-CN', {
    month: '2-digit',
    day: '2-digit',
    ...(withTime ? { hour: '2-digit', minute: '2-digit' } : {}),
  }).format(date);
}

export function formatFullDate(input: string): string {
  if (!input) return '—';
  const date = new Date(input);
  if (Number.isNaN(date.getTime())) return '—';
  return new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
  }).format(date);
}

export function relativeTime(input: string): string {
  if (!input) return '—';
  const date = new Date(input);
  if (Number.isNaN(date.getTime())) return '—';
  const diff = Date.now() - date.getTime();
  const minute = 60_000;
  const hour = 60 * minute;
  const day = 24 * hour;
  if (diff < minute) return '刚刚';
  if (diff < hour) return `${Math.floor(diff / minute)} 分钟前`;
  if (diff < day) return `${Math.floor(diff / hour)} 小时前`;
  if (diff < 7 * day) return `${Math.floor(diff / day)} 天前`;
  if (diff < 365 * day) return formatDate(input);
  return `${Math.floor(diff / (365 * day))} 年前`;
}

export function formatBytes(bytes: number): string {
  if (!Number.isFinite(bytes) || bytes <= 0) return '0 B';
  const units = ['B', 'KB', 'MB', 'GB'];
  const index = Math.min(Math.floor(Math.log(bytes) / Math.log(1024)), units.length - 1);
  const value = bytes / 1024 ** index;
  const rounded = Number(value.toFixed(1));
  return `${rounded >= 10 || index === 0 ? Math.round(rounded) : rounded} ${units[index]}`;
}

export function statusLabel(status: string, scope: 'idea' | 'project' | 'asset'): string {
  if (scope === 'idea') return IDEA_STATUS[status as IdeaStatus]?.label ?? status;
  if (scope === 'project') return PROJECT_STATUS[status as ProjectStatus]?.label ?? status;
  return ASSET_STATUS[status as AssetStatus]?.label ?? status;
}

export function statusTone(status: string, scope: 'idea' | 'project' | 'asset'): string {
  if (scope === 'idea') return IDEA_STATUS[status as IdeaStatus]?.color ?? 'neutral';
  if (scope === 'project') return PROJECT_STATUS[status as ProjectStatus]?.color ?? 'neutral';
  return ASSET_STATUS[status as AssetStatus]?.color ?? 'neutral';
}

export function kindLabel(kind: Kind | string): string {
  return KIND_MAP[kind as Kind]?.label ?? String(kind);
}

export function countLabel(counts: Partial<Record<Kind, number>>): string {
  const parts = (Object.keys(counts) as Kind[])
    .filter((k) => (counts[k] ?? 0) > 0)
    .map((k) => `${KIND_MAP[k]?.label ?? k} ${counts[k]}`);
  return parts.length ? parts.join(' · ') : '还没有素材';
}

export function excerpt(text: string, max = 120): string {
  const cleaned = text
    .replace(/^---[\s\S]*?---/, '')
    .replace(/[#>*_`~\-]+/g, ' ')
    .replace(/\s+/g, ' ')
    .trim();
  return cleaned.length > max ? `${cleaned.slice(0, max)}…` : cleaned;
}

export function extractTitle(markdown: string, fallback: string): string {
  const first = markdown
    .split('\n')
    .map((line) => line.trim())
    .find((line) => line.length > 0);
  if (!first) return fallback;
  const title = first.replace(/^#+\s*/, '').replace(/[*_`]/g, '').trim();
  return title.length > 0 && title.length <= 80 ? title : fallback;
}

export function todayLabel(): string {
  return new Intl.DateTimeFormat('zh-CN', {
    year: 'numeric',
    month: 'long',
    day: 'numeric',
    weekday: 'long',
  }).format(new Date());
}

export function greeting(): string {
  const hour = new Date().getHours();
  if (hour < 6) return '夜深了';
  if (hour < 11) return '早上好';
  if (hour < 14) return '中午好';
  if (hour < 18) return '下午好';
  return '晚上好';
}

export function copyText(text: string): Promise<void> {
  if (navigator.clipboard?.writeText) return navigator.clipboard.writeText(text);
  const area = document.createElement('textarea');
  area.value = text;
  area.style.position = 'fixed';
  area.style.opacity = '0';
  document.body.appendChild(area);
  area.select();
  document.execCommand('copy');
  area.remove();
  return Promise.resolve();
}

export function downloadUrl(url: string, filename: string): void {
  const link = document.createElement('a');
  link.href = url;
  link.download = filename;
  link.rel = 'noopener';
  document.body.appendChild(link);
  link.click();
  link.remove();
}

export function projectInitials(title: string): string {
  const chars = Array.from(title.replace(/\s+/g, '').trim());
  return chars.slice(0, 2).join('') || '项';
}
