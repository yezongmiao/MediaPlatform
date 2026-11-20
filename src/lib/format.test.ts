import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  countLabel,
  excerpt,
  extractTitle,
  formatBytes,
  formatDate,
  projectInitials,
  relativeTime,
  statusLabel,
} from './format';

describe('format helpers', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-11-20T12:00:00+08:00'));
  });
  afterEach(() => {
    vi.useRealTimers();
  });

  it('formats relative times in Chinese', () => {
    expect(relativeTime('2026-11-20T11:59:40+08:00')).toBe('刚刚');
    expect(relativeTime('2026-11-20T11:30:00+08:00')).toBe('30 分钟前');
    expect(relativeTime('2026-11-20T09:00:00+08:00')).toBe('3 小时前');
    expect(relativeTime('2026-11-18T12:00:00+08:00')).toBe('2 天前');
  });

  it('formats bytes with sensible units', () => {
    expect(formatBytes(0)).toBe('0 B');
    expect(formatBytes(512)).toBe('512 B');
    expect(formatBytes(2048)).toBe('2 KB');
    expect(formatBytes(3.5 * 1024 * 1024)).toBe('3.5 MB');
  });

  it('formats dates defensively', () => {
    expect(formatDate('')).toBe('—');
    expect(formatDate('not-a-date')).toBe('—');
    expect(formatDate('2026-11-20T12:00:00+08:00')).toMatch(/11\/20|11-20|11月20日/);
  });

  it('maps status labels', () => {
    expect(statusLabel('ready', 'asset')).toBe('可发布');
    expect(statusLabel('promoted', 'idea')).toBe('已立项');
    expect(statusLabel('done', 'project')).toBe('已完成');
  });

  it('extracts titles from markdown without markup', () => {
    expect(extractTitle('# 我的标题\n\n正文', 'fallback')).toBe('我的标题');
    expect(extractTitle('\n\n', 'fallback')).toBe('fallback');
    expect(extractTitle('**加粗开头**\n正文', 'fallback')).toBe('加粗开头');
  });

  it('builds compact project initials', () => {
    expect(projectInitials('AI 时代的个人创作')).toBe('AI');
    expect(projectInitials('一人公司的内容系统')).toBe('一人');
    expect(projectInitials('   ')).toBe('项');
  });

  it('cleans excerpts and counts', () => {
    expect(excerpt('---\nid: x\n---\n\n## 标题\n\n正文', 20)).toBe('标题 正文');
    expect(countLabel({ article: 2, script: 1 })).toContain('文章 2');
    expect(countLabel({})).toBe('还没有素材');
  });
});
