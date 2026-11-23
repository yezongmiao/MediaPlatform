import { describe, expect, it } from 'vitest';
import { extractStreamReply } from './assistant-stream';

describe('extractStreamReply', () => {
  it('extracts and decodes a complete reply', () => {
    expect(
      extractStreamReply('{"reply":"你好\\n世界","edits":[{"assetId":"A1"}]}'),
    ).toBe('你好\n世界');
  });

  it('extracts an incomplete streaming reply', () => {
    expect(extractStreamReply('{"reply":"正在组织')).toBe('正在组织');
  });

  it('returns plain text as-is', () => {
    expect(extractStreamReply('先说结论：还不够聚焦。')).toBe('先说结论：还不够聚焦。');
  });

  it('does not flash raw JSON before a reply appears', () => {
    expect(extractStreamReply('{"edits":[')).toBe('');
  });

  it('decodes unicode escapes', () => {
    expect(extractStreamReply('{"reply":"\\u4f60\\u597d"}')).toBe('你好');
  });
});
