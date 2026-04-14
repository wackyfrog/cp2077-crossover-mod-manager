import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./InstallOverlay.css";

const GLITCH_CHARS = "!@#$%^&*()_+-=[]{}|;:,.<>?/~`0123456789ABCDEFabcdef";

function DecodeLine({ text }) {
  const [display, setDisplay] = useState("");
  const timerRef = useRef(null);

  useEffect(() => {
    // Build list of scrambable positions (non-space)
    const positions = [];
    for (let i = 0; i < text.length; i++) {
      if (text[i] !== " ") positions.push(i);
    }
    // Shuffle to randomize reveal order
    for (let i = positions.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [positions[i], positions[j]] = [positions[j], positions[i]];
    }

    const revealed = new Set();
    let frame = 0;
    const revealPerFrame = Math.max(2, Math.ceil(positions.length / 6)); // ~6 frames total
    const totalFrames = Math.ceil(positions.length / revealPerFrame);

    const tick = () => {
      // Reveal a batch of positions
      const start = frame * revealPerFrame;
      const end = Math.min(start + revealPerFrame, positions.length);
      for (let k = start; k < end; k++) revealed.add(positions[k]);
      frame++;

      let result = "";
      for (let i = 0; i < text.length; i++) {
        if (text[i] === " " || revealed.has(i)) {
          result += text[i];
        } else {
          result += GLITCH_CHARS[Math.floor(Math.random() * GLITCH_CHARS.length)];
        }
      }
      setDisplay(result);

      if (frame <= totalFrames) {
        timerRef.current = setTimeout(tick, 50);
      } else {
        setDisplay(text);
      }
    };

    tick();
    return () => clearTimeout(timerRef.current);
  }, [text]);

  return <>{display}</>;
}

const FAKE_LINES = [
  "> INIT netrunner_link.so ... [OK]",
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
];

const STAGE_LABELS = {
  fetching: "FETCHING",
  downloading: "DOWNLOADING",
  extracting: "EXTRACTING",
  installing: "INSTALLING",
  registering: "REGISTERING",
  done: "COMPLETE",
  error: "ERROR",
};

export default function InstallOverlay({ progress, onDismiss }) {
  const [lines, setLines] = useState([]);
  const [dismissed, setDismissed] = useState(false);
  const [cancelling, setCancelling] = useState(false);
  const termRef = useRef(null);
  const fakeIdxRef = useRef(Math.floor(Math.random() * FAKE_LINES.length));
  const prevStageRef = useRef(null);
  const autoCloseRef = useRef(null);

  // Add real lines when stage/message changes
  useEffect(() => {
    if (!progress) return;
    const { stage, message } = progress;

    // Avoid duplicate lines for same stage+message
    if (prevStageRef.current === `${stage}:${message}`) return;
    prevStageRef.current = `${stage}:${message}`;

    const type = stage === "error" ? "error" : stage === "done" ? "done" : "stage";
    setLines((prev) => [...prev, { text: `[${STAGE_LABELS[stage] || stage}] ${message}`, type }]);
  }, [progress?.stage, progress?.message]);

  // Fake lines ticker
  useEffect(() => {
    if (!progress || progress.stage === "done" || progress.stage === "error") return;

    const tick = () => {
      const delay = 800 + Math.random() * 1400;
      return setTimeout(() => {
        const line = FAKE_LINES[fakeIdxRef.current % FAKE_LINES.length];
        fakeIdxRef.current++;
        setLines((prev) => [...prev, { text: line, type: "fake" }]);
        timerRef = tick();
      }, delay);
    };
    let timerRef = tick();
    return () => clearTimeout(timerRef);
  }, [progress?.stage]);

  // Auto-scroll terminal
  useEffect(() => {
    if (termRef.current) {
      termRef.current.scrollTop = termRef.current.scrollHeight;
    }
  }, [lines]);

  // Auto-close on done
  useEffect(() => {
    if (progress?.stage === "done") {
      setCancelling(false);
      autoCloseRef.current = setTimeout(() => {
        setDismissed(true);
        onDismiss?.();
      }, 2500);
    }
    if (progress?.stage === "error") {
      setCancelling(false);
    }
    return () => {
      if (autoCloseRef.current) clearTimeout(autoCloseRef.current);
    };
  }, [progress?.stage]);

  if (!progress || dismissed) return null;

  const { stage, message, bytes_received, total_bytes, file_count, file_total, nxm_url } = progress;
  const stageLabel = STAGE_LABELS[stage] || stage;

  // Progress bar
  let progressPct = null;
  if (stage === "downloading" && total_bytes && total_bytes > 0) {
    progressPct = Math.min(100, ((bytes_received || 0) / total_bytes) * 100);
  } else if (stage === "installing" && file_total && file_total > 0) {
    progressPct = Math.min(100, ((file_count || 0) / file_total) * 100);
  }

  const showIndeterminate = (stage === "downloading" && !total_bytes)
    || stage === "extracting"
    || stage === "registering"
    || stage === "fetching";

  const isActive = stage !== "done" && stage !== "error";

  const handleCancel = async () => {
    setCancelling(true);
    try {
      await invoke("cancel_install");
    } catch (e) {
      console.error("Cancel failed:", e);
    }
  };

  const handleRetry = async () => {
    if (!nxm_url) {
      console.warn("Retry: no nxm_url in progress payload");
      return;
    }
    setLines([]);
    prevStageRef.current = null;
    setCancelling(false);
    try {
      await invoke("handle_nxm_url", { nxmUrl: nxm_url });
    } catch (e) {
      console.error("Retry failed:", e);
    }
  };

  const handleDismiss = () => {
    setDismissed(true);
    onDismiss?.();
  };

  return (
    <div className="install-overlay">
      <div className="install-overlay-box">
        <div className="io-header">
          <div className={`io-stage stage-${stage}`}>{stageLabel}</div>
          <div className="io-message">{message}</div>

          {progressPct !== null && (
            <div className="io-progress-wrap">
              <div className="io-progress-bar" style={{ width: `${progressPct}%` }} />
            </div>
          )}
          {showIndeterminate && (
            <div className="io-progress-wrap">
              <div className="io-progress-indeterminate" />
            </div>
          )}
        </div>

        <div className="io-terminal" ref={termRef}>
          {lines.map((l, i) => (
            <div key={i} className={`io-line io-line-${l.type}`}>
              {l.type === "fake" ? <DecodeLine text={l.text} /> : l.text}
            </div>
          ))}
          {isActive && <span className="io-cursor" />}
        </div>

        <div className="io-actions">
          {isActive && (
            <button
              className="io-btn io-btn-danger"
              onClick={handleCancel}
              disabled={cancelling}
            >
              {cancelling ? "Aborting..." : "Cancel"}
            </button>
          )}

          {stage === "error" && (
            <>
              <button className="io-btn" onClick={handleDismiss}>Dismiss</button>
              {nxm_url && (
                <button className="io-btn io-btn-primary" onClick={handleRetry}>Retry</button>
              )}
            </>
          )}

          {stage === "done" && (
            <button className="io-btn io-btn-primary" onClick={handleDismiss}>Close</button>
          )}
        </div>
      </div>
    </div>
  );
}
