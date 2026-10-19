import { useCallback, useRef } from "react";

/**
 * Whether a keydown belongs to an IME composition being confirmed, rather
 * than to the operator.
 *
 * Three signals, because no one of them holds across engines:
 *
 * - `isComposing` — true on the confirming keydown in Chrome and Firefox.
 * - `keyCode === 229` — browsers mark keydowns handled by the IME "Process"
 *   with 229; some engines report the confirming Enter that way instead.
 * - a compositionend within the last burst — Safari dispatches the completing
 *   Enter *after* compositionend, with `isComposing` already false and a
 *   normal keyCode (WebKit bug 165004; the ordering is now called out in
 *   MDN's compat data). The only signal that Enter is the composition's is
 *   that the composition ended an instant ago. The window costs one swallowed
 *   Enter only if the operator presses Enter within the window of a
 *   composition they ended some other way, which is the standard trade.
 */
export function useImeEnter(burstMs = 100) {
  const compositionEndAt = useRef(0);
  const confirmedBeforeCompositionEnd = useRef(false);

  const onCompositionStart = useCallback(() => {
    // A new composition discards confirmation state left by the last one:
    // a keyCode-229 completing Enter that arrived after compositionend set
    // this flag without an end of its own to consume it, and it must not
    // disarm this composition's burst window.
    confirmedBeforeCompositionEnd.current = false;
  }, []);

  const onCompositionEnd = useCallback(() => {
    if (confirmedBeforeCompositionEnd.current) {
      // Chrome/Firefox order: the confirming keydown already ran with
      // isComposing (or keyCode 229), so this compositionend needs no burst
      // window at all - arming one would swallow the operator's next Enter.
      confirmedBeforeCompositionEnd.current = false;
      compositionEndAt.current = 0;
    } else {
      compositionEndAt.current = Date.now();
    }
  }, []);

  const confirmsComposition = useCallback(
    (event: React.KeyboardEvent) => {
      const native = event.nativeEvent;
      if (native.repeat && !native.isComposing) {
        // A held Enter repeats after compositionend with isComposing already
        // false. The repeat is the same gesture, never a second post - and
        // with the composition over its default action is a real newline, so
        // cancel that too or holding the key edits the draft it cannot send.
        event.preventDefault();
        return true;
      }
      if (native.isComposing || native.keyCode === 229) {
        confirmedBeforeCompositionEnd.current = true;
        // Engines that end the composition first still send the completing
        // Enter as keyCode 229; that Enter spends the burst window it follows,
        // or the operator's next Enter inside the window is swallowed.
        compositionEndAt.current = 0;
        return true;
      }
      if (Date.now() - compositionEndAt.current < burstMs) {
        // Safari's completing Enter. Suppress exactly once, then stand down:
        // the window must not swallow a genuine Enter that follows quickly.
        compositionEndAt.current = 0;
        return true;
      }
      return false;
    },
    [burstMs],
  );

  return { onCompositionStart, onCompositionEnd, confirmsComposition };
}
