/**
 * Text inputs in the app are controlled by Preact. During an IME composition
 * (Chinese/Japanese/Korean), updating state on each `input` event can make the
 * browser reset the composition buffer. Keep composition events away from
 * Preact and commit once the candidate is confirmed.
 */
let installed = false;
let activeCompositions = 0;

export function isImeComposing(event?: KeyboardEvent): boolean {
  return (
    activeCompositions > 0 ||
    Boolean(event?.isComposing) ||
    event?.keyCode === 229
  );
}

export function installImeGuard(): () => void {
  if (typeof document === 'undefined' || installed) return () => {};
  installed = true;
  let composing = false;

  const onStart = () => {
    composing = true;
    activeCompositions += 1;
  };

  const onEnd = (event: CompositionEvent) => {
    composing = false;
    activeCompositions = Math.max(0, activeCompositions - 1);
    const target = event.target;
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
      // Safari/WebKit variants do not always emit a final input event after
      // compositionend. A synthetic input lets controlled inputs commit once.
      target.dispatchEvent(new Event('input', { bubbles: true }));
    }
  };

  const onInput = (event: Event) => {
    if (composing || (event as InputEvent).isComposing) {
      event.stopImmediatePropagation();
    }
  };

  document.addEventListener('compositionstart', onStart, true);
  document.addEventListener('compositionend', onEnd, true);
  document.addEventListener('input', onInput, true);

  return () => {
    document.removeEventListener('compositionstart', onStart, true);
    document.removeEventListener('compositionend', onEnd, true);
    document.removeEventListener('input', onInput, true);
    activeCompositions = 0;
    installed = false;
  };
}
