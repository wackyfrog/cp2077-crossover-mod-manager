import { useEffect, useRef, useState, useCallback, memo } from "react";

const GLITCH_CHARS = "!@#$%^&*()_+-=[]{}|;:,.<>?/~`0123456789ABCDEFabcdef";

const FAKE_POOL = [
  "> fetching metadata ...",
  "> parsing version vector ...",
  "> comparing checksums ...",
  "> downloading thumbnail ...",
  "> extracting summary ...",
  "> checking for updates ...",
  "> validating schema ...",
  "> indexing file tree ...",
  "> resolving dependencies ...",
  "> writing to database ...",
  "> refreshing auth token ...",
  "> scanning file list ...",
  "> cross-referencing registry ...",
  "> decompressing payload ...",
];

function pickFake() {
  return FAKE_POOL[Math.floor(Math.random() * FAKE_POOL.length)];
}

// ── Scrambled text that types word-by-word then decodes ──
const ScrambledText = memo(function ScrambledText({ text }) {
  const [display, setDisplay] = useState("");
  const timerRef = useRef(null);
  const words = useRef(text.split(/(\s+)/));

  useEffect(() => {
    let wordIdx = 0;
    let frame = 0;

    const tick = () => {
      if (wordIdx >= words.current.length) {
        setDisplay(text);
        return;
      }

      const revealed = words.current.slice(0, wordIdx + 1).join("");
      frame++;

      // Scramble unrevealed chars in current word
      if (frame <= 3) {
        let result = "";
        const settled = words.current.slice(0, wordIdx).join("").length;
        for (let i = 0; i < revealed.length; i++) {
          if (i < settled || revealed[i] === " ") result += revealed[i];
          else result += GLITCH_CHARS[Math.floor(Math.random() * GLITCH_CHARS.length)];
        }
        setDisplay(result);
        timerRef.current = setTimeout(tick, 30);
      } else {
        setDisplay(revealed);
        frame = 0;
        wordIdx++;
        timerRef.current = setTimeout(tick, 40 + Math.random() * 80);
      }
    };

    tick();
    return () => clearTimeout(timerRef.current);
  }, [text]);

  return <>{display}</>;
});

export default function SyncTerminal({ active, onApi }) {
  const [lines, setLines] = useState([]);
  const [glitchIds, setGlitchIds] = useState(new Set());
  const termRef = useRef(null);
  const lineIdRef = useRef(0);
  const fakeTimerRef = useRef(null);
  const spinnerLinesRef = useRef([]); // IDs of current spinner (fake) lines

  // ── Add real line: clear spinner lines, add real ──
  const addRealLine = useCallback((text, type = "real") => {
    const id = ++lineIdRef.current;
    setLines((prev) => {
      // Remove ALL trailing fake lines (everything after last non-fake)
      const cleaned = prev.filter((l) => l.kind !== "fake");
      spinnerLinesRef.current = [];

      if (type === "divider") {
        return [...cleaned, { id, kind: "divider" }];
      }
      return [...cleaned, { id, kind: type, text }];
    });
  }, []);

  // ── Expose API to parent ──
  useEffect(() => {
    if (onApi) onApi({ addRealLine, reset: () => { setLines([]); spinnerLinesRef.current = []; } });
  }, [addRealLine, onApi]);

  // ── Spinner: generate fake lines between real lines ──
  useEffect(() => {
    if (!active) return;

    const tick = () => {
      const delay = 300 + Math.random() * 600;
      fakeTimerRef.current = setTimeout(() => {
        const id = ++lineIdRef.current;
        spinnerLinesRef.current.push(id);
        // Keep max 3 spinner lines
        if (spinnerLinesRef.current.length > 3) {
          const removeId = spinnerLinesRef.current.shift();
          setLines((prev) => [...prev.filter((l) => l.id !== removeId), {
            id, kind: "fake", text: pickFake(),
          }]);
        } else {
          setLines((prev) => [...prev, {
            id, kind: "fake", text: pickFake(),
          }]);
        }
        tick();
      }, delay);
    };
    tick();
    return () => clearTimeout(fakeTimerRef.current);
  }, [active]);

  // ── Glitch random visible lines ──
  useEffect(() => {
    if (!active) return;
    const glitchTick = () => {
      const delay = 2000 + Math.random() * 5000;
      return setTimeout(() => {
        const el = termRef.current;
        if (!el) { timer = glitchTick(); return; }

        // Find lines visible on screen
        const rect = el.getBoundingClientRect();
        const children = el.querySelectorAll(".sync-line");
        const visible = [];
        children.forEach((child) => {
          const cr = child.getBoundingClientRect();
          if (cr.bottom > rect.top && cr.top < rect.bottom) {
            const id = child.dataset.id;
            if (id) visible.push(id);
          }
        });

        if (visible.length > 0) {
          // Pick 2-5 consecutive lines from a random start
          const startIdx = Math.floor(Math.random() * visible.length);
          const count = 2 + Math.floor(Math.random() * 4);
          const ids = new Set();
          for (let i = 0; i < count && startIdx + i < visible.length; i++) {
            ids.add(visible[startIdx + i]);
          }
          setGlitchIds(ids);
          setTimeout(() => setGlitchIds(new Set()), 700);
        }

        timer = glitchTick();
      }, delay);
    };
    let timer = glitchTick();
    return () => clearTimeout(timer);
  }, [active]);

  // ── Cleanup old fake lines far above viewport ──
  useEffect(() => {
    if (!active) return;
    const interval = setInterval(() => {
      const el = termRef.current;
      if (!el) return;
      const maxKeep = Math.floor(el.clientHeight / 16) * 3;
      setLines((prev) => {
        if (prev.length <= maxKeep) return prev;
        const excess = prev.length - maxKeep;
        let removed = 0;
        const kept = [];
        for (const line of prev) {
          if (removed < excess && line.kind === "fake") removed++;
          else kept.push(line);
        }
        return removed > 0 ? kept : prev;
      });
    }, 8000);
    return () => clearInterval(interval);
  }, [active]);

  // ── Auto-scroll ──
  useEffect(() => {
    const el = termRef.current;
    if (!el) return;
    const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 80;
    if (nearBottom) el.scrollTop = el.scrollHeight;
  }, [lines]);

  return (
    <div className="sync-terminal" ref={termRef}>
      {lines.map((l) => {
        if (l.kind === "divider") return <div key={l.id} className="sync-line-divider" />;

        const isGlitched = glitchIds.has(String(l.id));
        const cls = [
          "sync-line",
          `sync-line-${l.kind}`,
          isGlitched ? "sync-line-glitch" : "",
        ].filter(Boolean).join(" ");

        return (
          <div key={l.id} data-id={l.id} className={cls}>
            {l.kind === "fake" ? <ScrambledText text={l.text} /> : l.text}
          </div>
        );
      })}
      {active && <span className="sync-cursor" />}
    </div>
  );
}
