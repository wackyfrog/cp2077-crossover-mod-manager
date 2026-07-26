import { useEffect, useRef } from "react";

/**
 * Close-on-Escape that works wherever focus happens to be.
 *
 * Handlers used to hang off individual `<input onKeyDown>`, so Escape only did
 * anything while a text field had focus — click anywhere else in an overlay and
 * the key went nowhere.
 *
 * Layers form a stack and only the topmost one reacts, so Escape closes the
 * dialog in front rather than everything at once. Registration order is what
 * decides "topmost": a dialog opened on top of an overlay mounts later, so it
 * lands on top of the stack and gets the key first.
 */
const stack = [];

if (typeof window !== "undefined") {
  window.addEventListener(
    "keydown",
    (e) => {
      if (e.key !== "Escape" || stack.length === 0) return;
      e.preventDefault();
      stack[stack.length - 1].run();
    },
    // Capture, so a stray handler on an inner element can't swallow the key first.
    true
  );
}

/**
 * @param {boolean} active whether this layer is currently on screen
 * @param {() => void} onEscape what to do when Escape reaches this layer
 */
export default function useEscape(active, onEscape) {
  // Kept in a ref so an inline arrow function doesn't re-register the layer on
  // every render — that would shuffle it to the top of the stack and break the
  // ordering this hook exists to guarantee.
  const handlerRef = useRef(onEscape);
  useEffect(() => {
    handlerRef.current = onEscape;
  });

  useEffect(() => {
    if (!active) return;
    const entry = { run: () => handlerRef.current?.() };
    stack.push(entry);
    return () => {
      const i = stack.indexOf(entry);
      if (i !== -1) stack.splice(i, 1);
    };
  }, [active]);
}
