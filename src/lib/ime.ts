/**
 * Text inputs in the app are controlled by Preact. During an IME composition
 * (Chinese/Japanese/Korean), updating state on each `input` event can make the
 * browser reset the composition buffer. Keep composition events away from
 * Preact and commit once the candidate is confirmed.
 *
 * WebKit does not always deliver `compositionend` when the field is blurred,
 * unmounted, or the window loses focus. A single missed end event used to leave
 * the global guard stuck forever, which made every controlled input stop
 * updating and all Enter shortcuts ignore input. The guard therefore has a
 * focus-out/window-blur fallback and a watchdog.
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
  let compositionTarget: EventTarget | null = null;
  let safetyTimer: ReturnType<typeof setTimeout> | undefined;

  const clearSafety = () => {
    if (safetyTimer !== undefined) {
      clearTimeout(safetyTimer);
      safetyTimer = undefined;
    }
  };

  const commit = (target: EventTarget | null) => {
    if (target instanceof HTMLInputElement || target instanceof HTMLTextAreaElement) {
      // Safari/WebKit variants do not always emit a final input event after
      // compositionend. A synthetic input lets controlled inputs commit once.
      target.dispatchEvent(new Event('input', { bubbles: true }));
    }
  };

  const armSafety = () => {
    clearSafety();
    safetyTimer = setTimeout(() => {
      if (!composing) return;
      composing = false;
      activeCompositions = 0;
      commit(compositionTarget);
      compositionTarget = null;
    }, 30_000);
  };

  const onStart = (event: CompositionEvent) => {
    if (!composing) activeCompositions += 1;
    composing = true;
    compositionTarget = event.target;
    armSafety();
  };

  const onUpdate = () => {
    if (composing) armSafety();
  };

  const onEnd = (event: CompositionEvent) => {
    if (composing) activeCompositions = Math.max(0, activeCompositions - 1);
    composing = false;
    clearSafety();
    commit(event.target);
    if (compositionTarget === event.target) compositionTarget = null;
  };

  const onInput = (event: Event) => {
    if (composing || (event as InputEvent).isComposing) {
      event.stopImmediatePropagation();
    }
  };

  const onFocusOut = (event: FocusEvent) => {
    if (!composing) return;
    // A blur means the composition is being abandoned/committed by the
    // browser. Commit the current DOM value immediately and release the guard;
    // WebKit may never send compositionend in this path.
    composing = false;
    activeCompositions = 0;
    clearSafety();
    commit(event.target);
    compositionTarget = null;
  };

  const onWindowBlur = () => {
    if (!composing) return;
    composing = false;
    activeCompositions = 0;
    clearSafety();
    commit(compositionTarget);
    compositionTarget = null;
  };

  document.addEventListener('compositionstart', onStart, true);
  document.addEventListener('compositionupdate', onUpdate, true);
  document.addEventListener('compositionend', onEnd, true);
  document.addEventListener('input', onInput, true);
  document.addEventListener('focusout', onFocusOut, true);
  window.addEventListener('blur', onWindowBlur);

  return () => {
    document.removeEventListener('compositionstart', onStart, true);
    document.removeEventListener('compositionupdate', onUpdate, true);
    document.removeEventListener('compositionend', onEnd, true);
    document.removeEventListener('input', onInput, true);
    document.removeEventListener('focusout', onFocusOut, true);
    window.removeEventListener('blur', onWindowBlur);
    clearSafety();
    activeCompositions = 0;
    installed = false;
  };
}
