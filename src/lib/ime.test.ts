import { describe, expect, it } from 'vitest';
import { isImeComposing } from './ime';

describe('isImeComposing', () => {
  it('accepts native composition flags', () => {
    expect(isImeComposing({ isComposing: true, keyCode: 13 } as KeyboardEvent)).toBe(true);
    expect(isImeComposing({ isComposing: false, keyCode: 13 } as KeyboardEvent)).toBe(false);
  });

  it('accepts the legacy keyCode 229 sent by WebKit during composition', () => {
    expect(isImeComposing({ isComposing: false, keyCode: 229 } as KeyboardEvent)).toBe(true);
  });
});
