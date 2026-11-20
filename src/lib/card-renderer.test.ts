import { describe, expect, it } from 'vitest';
import { CARD_RATIOS, CARD_THEMES, extractCardSubtext, extractCardText } from './card-renderer';

describe('card copy extraction', () => {
  const generated = '**主张**：创作是把想法变成可保存的东西\n**补充**：先写下来，再慢慢变好。\n别的内容';

  it('extracts the core line and subtext from generated copy', () => {
    expect(extractCardText(generated)).toBe('创作是把想法变成可保存的东西');
    expect(extractCardSubtext(generated)).toBe('先写下来，再慢慢变好。');
  });

  it('falls back to the first meaningful line', () => {
    expect(extractCardText('# 标题\n\n正文第一句')).toBe('标题');
    expect(extractCardSubtext('# 标题\n\n正文第一句')).toBe('');
  });

  it('exposes stable ratios and themes', () => {
    expect(CARD_RATIOS['3:4']).toMatchObject({ width: 1080, height: 1440 });
    expect(CARD_THEMES).toHaveLength(5);
  });
});
