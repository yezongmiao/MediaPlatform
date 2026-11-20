import { describe, expect, it } from 'vitest';
import { renderMarkdown } from './markdown';

describe('markdown rendering', () => {
  it('renders headings, lists and tables', () => {
    const html = renderMarkdown('# 标题\n\n- 一\n- 二\n\n| A | B |\n|---|---|\n| 1 | 2 |');
    expect(html).toContain('<h1');
    expect(html).toContain('<li>一</li>');
    expect(html).toContain('<table>');
  });

  it('sanitizes scripts and event handlers', () => {
    const html = renderMarkdown(
      '<script>alert(1)</script>\n\n<img src="x" onerror="alert(2)">\n\n[链接](javascript:alert(3))',
    );
    expect(html).not.toContain('<script');
    expect(html).not.toContain('onerror');
    expect(html).not.toContain('javascript:');
  });
});
