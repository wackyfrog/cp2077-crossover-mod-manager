import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./JackInOverlay.css";

const GLITCH_CHARS = "!@#$%^&*()_+-=[]{}|;:,.<>?/~`0123456789ABCDEFabcdef";

function DecodeLine({ text }) {
  const [display, setDisplay] = useState("");
  const timerRef = useRef(null);

  useEffect(() => {
    const positions = [];
    for (let i = 0; i < text.length; i++) {
      if (text[i] !== " ") positions.push(i);
    }
    for (let i = positions.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [positions[i], positions[j]] = [positions[j], positions[i]];
    }

    const revealed = new Set();
    let frame = 0;
    const revealPerFrame = Math.max(2, Math.ceil(positions.length / 6));
    const totalFrames = Math.ceil(positions.length / revealPerFrame);

    const tick = () => {
      const start = frame * revealPerFrame;
      const end = Math.min(start + revealPerFrame, positions.length);
      for (let k = start; k < end; k++) revealed.add(positions[k]);
      frame++;

      let result = "";
      for (let i = 0; i < text.length; i++) {
        if (text[i] === " " || revealed.has(i)) result += text[i];
        else result += GLITCH_CHARS[Math.floor(Math.random() * GLITCH_CHARS.length)];
      }
      setDisplay(result);

      if (frame <= totalFrames) timerRef.current = setTimeout(tick, 50);
      else setDisplay(text);
    };

    tick();
    return () => clearTimeout(timerRef.current);
  }, [text]);

  return <>{display}</>;
}

const FAKE_SHORT = [
  "> PING 0xFFFF ... OK",
  "> vmem patched",
  "> ICE bypass [OK]",
  "> entropy rotated",
  "> cache hit",
  "> ACK 0x00",
  "> sync ... nominal",
  "> TTL=64 OK",
  "> auth token valid",
  "> CRC32 match",
];

const FAKE_MED = [
  "> scanning malware signatures ...",
  "> patching vmem 0x7fff2a10 ...",
  "> compiling shader cache ...",
  "> syncing ICE protocols ...",
  "> verifying quickhack checksums ...",
  "> loading braindance codec v4.2 ...",
  "> calibrating optical camo mesh ...",
  "> decrypting shard payload ...",
  "> injecting daemon: PING 0xFFFF ...",
  "> rotating entropy pool ...",
  "> flashing BIOS subsystem ...",
  "> handshake OK — latency 2ms",
  "> resolving subnet mask ...",
  "> allocating cyberspace buffer ...",
  "> monowire firmware check ... [PASS]",
  "> sandevistan sync ... nominal",
  "> linking Kiroshi optics driver ...",
  "> hashing mod manifest ...",
  "> RED4ext bridge ... [READY]",
  "> INIT netrunner_link.so ... [OK]",
  "> negotiating TLS 1.3 handshake ...",
  "> spawning daemon thread #7 ...",
  "> validating Arasaka cert chain ...",
  "> decompressing zlib stream ...",
  "> rebasing virtual address table ...",
  "> flushing L2 instruction cache ...",
  "> mapping shared memory segment ...",
  "> polling NIC interrupt vector ...",
  "> writing journal checkpoint ...",
  "> recycling socket descriptors ...",
  "> verifying ELF section headers ...",
  "> probing PCI bus for devices ...",
  "> calibrating gyro stabilizer ...",
  "> attaching debugger hooks ... [SKIP]",
];

const FAKE_LONG = [
  "> === RUNNING POST-INSTALL INTEGRITY CHECK ON DOWNLOADED ARCHIVE ===",
  "> scanning 2,847 files across 14 directories for conflict markers ...",
  "> rebuilding shader permutation cache — this may take a moment ...",
  "> === NETRUNNER HANDSHAKE SEQUENCE: AUTH > VERIFY > DECRYPT > INJECT ===",
  "> comparing local file hashes against Nexus CDN manifest (SHA-256) ...",
  "> decompressing multi-part archive: part 1/3 ... part 2/3 ... part 3/3 ...",
  "> resolving 47 symbolic links in game directory tree ... all valid",
  "> === ICE PENETRATION LAYER 3 OF 5 — ESTIMATED TIME: <2s ===",
  "> validating mod load order: checking 229 installed modules for conflicts ...",
  "> cross-referencing REDmod deployment manifest with installed DLC list ...",
];

function pickFake() {
  const r = Math.random();
  const pool = r < 0.15 ? FAKE_SHORT : r < 0.92 ? FAKE_MED : FAKE_LONG;
  return pool[Math.floor(Math.random() * pool.length)];
}

const MAX_LINES = 35;

function trimLines(lines) {
  if (lines.length <= MAX_LINES) return lines;
  // Keep last MAX_LINES, but try to remove fake first
  const excess = lines.length - MAX_LINES;
  let removed = 0;
  const result = [];
  for (const line of lines) {
    if (removed < excess && line.type === "fake") {
      removed++;
    } else {
      result.push(line);
    }
  }
  // If still too many, hard-slice (keeps most recent)
  return result.slice(-MAX_LINES);
}

function friendlyError(msg) {
  if (!msg) return null;
  if (msg.includes("already installed")) {
    // Extract mod name: "Mod 'Something v1.2' with the same..."
    const match = msg.match(/Mod '(.+?)' with the same/);
    return {
      type: "conflict",
      title: "Already jacked in",
      modName: match?.[1] || null,
      hint: "This chrome is already installed with the same file version.",
    };
  }
  if (msg.includes("403") && msg.includes("premium")) {
    return {
      title: "Premium required for direct links",
      hint: "Click \"Download with Mod Manager\" on nexusmods.com instead — that provides a download key that works for all users.",
    };
  }
  if (msg.includes("401") || msg.includes("Invalid API Key")) {
    return {
      title: "Invalid or missing API key",
      hint: "Go to Config → paste your NexusMods API key. Get one at nexusmods.com/users/myaccount?tab=api+access",
    };
  }
  if (msg.includes("404")) {
    return {
      title: "Mod or file not found",
      hint: "The mod ID or file ID in the URL may be wrong. Check the link on nexusmods.com.",
    };
  }
  if (msg.includes("No response") || msg.includes("timed out")) {
    return {
      title: "No response from backend",
      hint: "The URL may be malformed. Use the full link from nexusmods.com with all parameters.",
    };
  }
  return null;
}

const STAGE_LABELS = {
  fetching: "FETCHING",
  downloading: "DOWNLOADING",
  extracting: "EXTRACTING",
  installing: "INSTALLING",
  registering: "REGISTERING",
  done: "COMPLETE",
  error: "ERROR",
};

export default function JackInOverlay({ open, progress, onSubmit, onRetry, onReinstall, onCancel, onDismiss }) {
  const [url, setUrl] = useState("");
  const [lastNxmUrl, setLastNxmUrl] = useState(null);
  const [modLabel, setModLabel] = useState(null);
  const [lines, setLines] = useState([]);
  const [phase, setPhase] = useState("input"); // input | working | done | error
  const [cancelling, setCancelling] = useState(false);
  const [typingLine, setTypingLine] = useState(null);
  const [eta, setEta] = useState(null);
  const [glitchLines, setGlitchLines] = useState(new Set());
  const [blockGlitch, setBlockGlitch] = useState(false);
  const inputRef = useRef(null);
  const termRef = useRef(null);
  const prevStageRef = useRef(null);
  const autoCloseRef = useRef(null);
  const typingTimerRef = useRef(null);
  const fakeTimerRef = useRef(null);
  const dlStartRef = useRef(null); // { time, bytes } — snapshot when download started

  // Finish current typing line immediately — push full text to lines
  const flushTyping = () => {
    if (typingTimerRef.current) clearTimeout(typingTimerRef.current);
    setTypingLine((cur) => {
      if (cur) setLines((prev) => [...prev, { text: cur.full, type: "fake", done: true }]);
      return null;
    });
  };

  // Focus input on open
  useEffect(() => {
    if (open && phase === "input") {
      setTimeout(() => inputRef.current?.focus(), 100);
    }
  }, [open, phase]);

  // Track progress — flush fake before adding real line
  useEffect(() => {
    if (!progress) return;
    const { stage, message } = progress;

    if (progress.nxm_url) setLastNxmUrl(progress.nxm_url);
    if (progress.mod_name) setModLabel(progress.mod_name);
    // Try to extract mod name from message if backend didn't provide it
    if (!modLabel && message) {
      // Try to extract from "Downloading file: FileName.zip" or "Installing ModName"
      const dlMatch = message.match(/^Downloading file:\s*(.+)/i);
      const instMatch = message.match(/^Installing (.+?)(?:\s*$)/);
      if (dlMatch) setModLabel(dlMatch[1]);
      else if (instMatch) setModLabel(instMatch[1]);
    }
    if (stage === "done") setPhase("done");
    else if (stage === "error") setPhase("error");
    else if (phase === "input") setPhase("working");

    if (prevStageRef.current === `${stage}:${message}`) return;
    prevStageRef.current = `${stage}:${message}`;

    flushTyping();

    const type = stage === "error" ? "error" : stage === "done" ? "done" : "stage";
    const newText = `[${STAGE_LABELS[stage] || stage}] ${message}`;

    setLines((prev) => {
      // For downloading/installing updates, replace last stage line of same type to avoid spam
      if (stage === "downloading" || stage === "installing") {
        const lastIdx = prev.findLastIndex((l) => l.type === "stage" && l.text.startsWith(`[${STAGE_LABELS[stage]}`));
        if (lastIdx >= 0) {
          const updated = [...prev];
          updated[lastIdx] = { text: newText, type };
          return trimLines(updated);
        }
      }
      return trimLines([...prev, { text: newText, type }]);
    });
  }, [progress?.stage, progress?.message]);

  // Word-by-word typing of fake line
  useEffect(() => {
    if (!typingLine || typingLine.revealed >= typingLine.words.length) return;

    const delay = 60 + Math.random() * 180; // 60-240ms per word
    typingTimerRef.current = setTimeout(() => {
      setTypingLine((cur) => cur ? { ...cur, revealed: cur.revealed + 1 } : null);
    }, delay);

    return () => clearTimeout(typingTimerRef.current);
  }, [typingLine?.revealed]);

  // When typing finishes, commit line and schedule next fake
  useEffect(() => {
    if (!typingLine || typingLine.revealed < typingLine.words.length) return;

    // Typing done — commit to lines
    setLines((prev) => trimLines([...prev, { text: typingLine.full, type: "fake", done: true }]));
    setTypingLine(null);
  }, [typingLine?.revealed]);

  // Schedule next fake line
  useEffect(() => {
    if (phase !== "working") return;
    if (typingLine) return; // wait for current to finish

    const r = Math.random();
    const delay = r < 0.1 ? 100 + Math.random() * 200
                : r < 0.85 ? 400 + Math.random() * 800
                : 1500 + Math.random() * 1500;

    fakeTimerRef.current = setTimeout(() => {
      const text = pickFake();
      const words = text.split(/(\s+)/); // keep whitespace as tokens
      setTypingLine({ words, revealed: 0, full: text });
    }, delay);

    return () => clearTimeout(fakeTimerRef.current);
  }, [phase, typingLine]);

  // Auto-scroll
  useEffect(() => {
    if (termRef.current) termRef.current.scrollTop = termRef.current.scrollHeight;
  }, [lines, typingLine?.revealed]);

  // Auto-close on done
  useEffect(() => {
    if (phase === "done") {
      setCancelling(false);
    }
    if (phase === "error") setCancelling(false);
    return () => { if (autoCloseRef.current) clearTimeout(autoCloseRef.current); };
  }, [phase]);

  // Random glitches
  useEffect(() => {
    if (phase !== "working") return;

    const glitchTick = () => {
      const delay = 2000 + Math.random() * 6000;
      return setTimeout(() => {
        const r = Math.random();
        if (r < 0.6 && lines.length > 0) {
          // Glitch 1-3 random lines
          const count = 1 + Math.floor(Math.random() * 3);
          const ids = new Set();
          for (let i = 0; i < count; i++) ids.add(Math.floor(Math.random() * lines.length));
          setGlitchLines(ids);
          setTimeout(() => setGlitchLines(new Set()), 750);
        } else {
          // Block glitch — whole terminal
          setBlockGlitch(true);
          setTimeout(() => setBlockGlitch(false), 550);
        }
        timer = glitchTick();
      }, delay);
    };
    let timer = glitchTick();
    return () => clearTimeout(timer);
  }, [phase, lines.length]);

  // ETA calculation
  useEffect(() => {
    if (!progress || progress.stage !== "downloading" || !progress.total_bytes) {
      if (progress?.stage !== "downloading") { setEta(null); dlStartRef.current = null; }
      return;
    }
    const now = Date.now();
    const recv = progress.bytes_received || 0;

    // Capture start point
    if (!dlStartRef.current || dlStartRef.current.bytes > recv) {
      dlStartRef.current = { time: now, bytes: recv };
      return;
    }

    const elapsed = (now - dlStartRef.current.time) / 1000; // seconds
    const downloaded = recv - dlStartRef.current.bytes;

    if (elapsed < 1 || downloaded < 1024) return; // not enough data

    const speed = downloaded / elapsed; // bytes/sec
    const remaining = progress.total_bytes - recv;
    const secsLeft = remaining / speed;

    if (secsLeft < 1) { setEta("< 1s"); return; }
    if (secsLeft < 60) { setEta(`~${Math.ceil(secsLeft)}s`); return; }
    const mins = Math.floor(secsLeft / 60);
    const secs = Math.ceil(secsLeft % 60);
    setEta(`~${mins}m ${secs}s`);
  }, [progress?.bytes_received]);

  // Reset on open
  useEffect(() => {
    if (open) {
      setUrl("");
      setLastNxmUrl(null);
      setModLabel(null);
      setInputError(null);
      setLines([]);
      setPhase("input");
      setCancelling(false);
      setTypingLine(null);
      setEta(null);
      dlStartRef.current = null;
      prevStageRef.current = null;
      clearTimeout(typingTimerRef.current);
      clearTimeout(fakeTimerRef.current);
    }
  }, [open]);

  const [inputError, setInputError] = useState(null);

  if (!open) return null;

  const nxmUrl = progress?.nxm_url;

  const handleGo = () => {
    const trimmed = url.trim();
    if (!trimmed) return;
    if (!trimmed.startsWith("nxm://")) {
      setInputError("Invalid URL — must start with nxm://");
      return;
    }
    if (!trimmed.includes("cyberpunk2077")) {
      setInputError("Invalid URL — must be a Cyberpunk 2077 mod link");
      return;
    }
    // Must have format: nxm://cyberpunk2077/mods/{mod_id}/files/{file_id}
    const parts = trimmed.replace("nxm://", "").split("/").filter(Boolean);
    if (parts.length < 4 || parts[1] !== "mods" || !parts[3]) {
      setInputError("Invalid format — need nxm://cyberpunk2077/mods/{id}/files/{id}?...");
      return;
    }
    setInputError(null);
    setLastNxmUrl(trimmed);
    setPhase("working");
    onSubmit(trimmed);
  };

  const handleClose = () => {
    setUrl("");
    setLines([]);
    setPhase("input");
    prevStageRef.current = null;
    onDismiss();
  };

  const handleAbort = async () => {
    setCancelling(true);
    try {
      await invoke("cancel_install");
    } catch {}
    // If backend doesn't emit error event within 3s, force error state
    setTimeout(() => {
      setPhase((cur) => {
        setCancelling(false);
        if (cur === "working") {
          setLines((prev) => {
            if (prev.some(l => l.type === "error")) return prev;
            return trimLines([...prev, { text: "[ABORTED] Installation cancelled by user", type: "error" }]);
          });
          return "error";
        }
        return cur;
      });
    }, 3000);
  };

  const handleRetry = () => {
    const retryUrl = lastNxmUrl || nxmUrl || url;
    if (!retryUrl) {
      console.warn("Retry: no URL available");
      return;
    }
    setLines([]);
    prevStageRef.current = null;
    setPhase("working");
    setCancelling(false);
    setModLabel(null);
    if (onRetry) onRetry();
    onSubmit(retryUrl);
  };

  const { stage, message, bytes_received, total_bytes, file_count, file_total, mod_name } = progress || {};
  let progressPct = null;
  if (stage === "downloading" && total_bytes > 0) {
    progressPct = Math.min(100, ((bytes_received || 0) / total_bytes) * 100);
  } else if (stage === "installing" && file_total > 0) {
    progressPct = Math.min(100, ((file_count || 0) / file_total) * 100);
  }

  const showIndeterminate = phase === "working" && progressPct === null && stage !== "done" && stage !== "error";
  const errorInfo = phase === "error" ? friendlyError(message) : null;

  const pctStr = progressPct !== null ? `${progressPct.toFixed(1)}%` : null;
  const speedStr = (() => {
    if (!dlStartRef.current || !progress?.bytes_received) return null;
    const elapsed = (Date.now() - dlStartRef.current.time) / 1000;
    const dl = (progress.bytes_received - dlStartRef.current.bytes);
    if (elapsed < 1 || dl < 1024) return null;
    const speed = dl / elapsed;
    if (speed > 1024 * 1024) return `${(speed / 1024 / 1024).toFixed(1)} MB/s`;
    return `${(speed / 1024).toFixed(0)} KB/s`;
  })();

  return (
    <div className="jackin-overlay">
      {/* ── Militech-style header ── */}
      <div className="jackin-chrome-bar">
        <span className="jackin-chrome-tag">NEXUS UPLINK</span>
        <span className="jackin-chrome-divider" />
        <span className="jackin-chrome-sys">CROSSOVER MOD MANAGER v1.1.0</span>
        <span className="jackin-chrome-divider" />
        <span className="jackin-chrome-status">
          {phase === "input" ? "AWAITING INPUT" : phase === "working" ? "TRANSFER IN PROGRESS" : phase === "done" ? "COMPLETE" : "FAULT DETECTED"}
        </span>
      </div>

      <div className="jackin-header">
        <div className="jackin-title-row">
          <div className="jackin-title">Jack In</div>
          {pctStr && <div className="jackin-pct">{pctStr}</div>}
        </div>

        {phase === "input" ? (
          <div className="jackin-input-area">
            <p className="jackin-desc">Paste an NXM link to download and install</p>
            {inputError && <p className="jackin-input-error">{inputError}</p>}
            <div className="jackin-input-row">
              <input
                ref={inputRef}
                type="text"
                className="jackin-input"
                placeholder="nxm://cyberpunk2077/mods/..."
                value={url}
                onChange={(e) => { setUrl(e.target.value); setInputError(null); }}
                onKeyDown={(e) => {
                  if (e.key === "Enter") handleGo();
                  if (e.key === "Escape") handleClose();
                }}
              />
              <button className="jackin-btn primary" disabled={!url.trim()} onClick={handleGo}>Go</button>
              <button className="jackin-btn" onClick={handleClose}>Abort</button>
            </div>
          </div>
        ) : (
          <div className="jackin-progress-area">
            <div className={`jackin-stage stage-${stage}`}>
              {STAGE_LABELS[stage] || stage || "CONNECTING"}
              {(mod_name || modLabel) && <span className="jackin-mod-name"> — {mod_name || modLabel}</span>}
            </div>
            {errorInfo?.type === "conflict" ? (
              <div className="jackin-conflict">
                <div className="jackin-conflict-title">{errorInfo.title}</div>
                <div className="jackin-conflict-hint">{errorInfo.hint}</div>
                <div className="jackin-conflict-actions">
                  <button className="jackin-btn" onClick={() => {
                    setUrl(""); setLines([]); setPhase("input"); prevStageRef.current = null;
                    onDismiss("conflict-cancel");
                  }}>Cancel</button>
                  <button className="jackin-btn primary" onClick={() => {
                    const retryUrl = lastNxmUrl || nxmUrl || url;
                    if (retryUrl && onReinstall) {
                      setLines([]);
                      prevStageRef.current = null;
                      setPhase("working");
                      setModLabel(null);
                      if (onRetry) onRetry();
                      onReinstall(retryUrl);
                    }
                  }}>Reinstall</button>
                </div>
              </div>
            ) : errorInfo ? (
              <div className="jackin-error-detail">
                <div className="jackin-error-title">{errorInfo.title}</div>
                <div className="jackin-error-hint">{errorInfo.hint}</div>
              </div>
            ) : (
              <div className="jackin-message">
                {message}
                {eta && <span className="jackin-eta"> — {eta} remaining</span>}
                {speedStr && <span className="jackin-speed"> [{speedStr}]</span>}
              </div>
            )}
            {progressPct !== null && (
              <div className="jackin-progress-wrap">
                <div className="jackin-progress-bar" style={{ width: `${progressPct}%` }} />
              </div>
            )}
            {showIndeterminate && (
              <div className="jackin-progress-wrap">
                <div className="jackin-progress-indeterminate" />
              </div>
            )}
          </div>
        )}
      </div>

      {/* ── Terminal body ── */}
      <div className={`jackin-terminal ${blockGlitch ? "block-glitch" : ""}`} ref={termRef}>
        {lines.map((l, i) => (
          <div key={i} className={`jackin-line jackin-line-${l.type} ${glitchLines.has(i) ? "glitch" : ""}`}>
            {l.type === "fake" && !l.done ? <DecodeLine text={l.text} /> : l.text}
          </div>
        ))}
        {typingLine && (
          <div className="jackin-line jackin-line-fake jackin-line-typing">
            {typingLine.words.slice(0, typingLine.revealed).join("")}
            <span className="jackin-cursor" />
          </div>
        )}
        {!typingLine && phase === "working" && <span className="jackin-cursor" />}
      </div>

      {/* ── Footer ── */}
      <div className="jackin-footer">
        <div className="jackin-footer-info">
          {phase === "working" && pctStr && (
            <span className="jackin-footer-readout">{pctStr} {speedStr && `// ${speedStr}`} {eta && `// ETA ${eta}`}</span>
          )}
        </div>
        <div className="jackin-footer-actions">
          {phase === "working" && (
            <button className="jackin-btn danger" onClick={handleAbort} disabled={cancelling}>
              {cancelling ? "Aborting..." : "Cancel"}
            </button>
          )}
          {phase === "error" && !errorInfo?.type && (
            <>
              <button className="jackin-btn" onClick={handleClose}>Dismiss</button>
              <button className="jackin-btn primary" onClick={handleRetry}>Retry</button>
            </>
          )}
          {phase === "done" && (
            <button className="jackin-btn primary" onClick={handleClose}>Done</button>
          )}
        </div>
      </div>
    </div>
  );
}
