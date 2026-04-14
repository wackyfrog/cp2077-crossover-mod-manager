import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./DevRelayOverlay.css";

/**
 * Shown only on the /Applications instance when an nxm:// link arrives
 * and a dev instance is running. Displays relay progress; on error shows
 * "Process here" / "Exit" actions.
 *
 * Props:
 *   stage      — "relaying" | "done" | "error"
 *   message    — string
 *   nxmUrl     — string | null  (needed for "process" action)
 *   coldStart  — bool (true = app launched for this link; false = app was already open)
 *   onDismiss  — function to clear the overlay
 */
export default function DevRelayOverlay({ stage, message, nxmUrl, coldStart, onDismiss }) {
  const [dismissed, setDismissed] = useState(false);
  const autoCloseRef = useRef(null);

  // Cancel auto-close countdown if user dismisses
  const handleDismiss = () => {
    if (autoCloseRef.current) clearTimeout(autoCloseRef.current);
    setDismissed(true);
    onDismiss?.();
  };

  // Reset dismissed state when new relay starts
  useEffect(() => {
    if (stage === "relaying") {
      setDismissed(false);
    }
  }, [stage]);

  if (!stage || dismissed) return null;

  const handleProcess = async () => {
    try {
      await invoke("handle_relay_action", { action: "process", nxmUrl });
    } catch (e) {
      console.error("handle_relay_action process failed:", e);
    }
  };

  const handleExit = async () => {
    try {
      await invoke("handle_relay_action", { action: "exit", nxmUrl: null });
    } catch (e) {
      console.error("handle_relay_action exit failed:", e);
    }
  };

  return (
    <div className="dro-backdrop">
      <div className="dro">
        <div className="dro-header">
          <span className="dro-prefix">▸</span>
          <span className="dro-title">NXM RELAY</span>
        </div>

        <div className="dro-body">
          <div className={`dro-stage dro-stage--${stage}`}>
            {stage === "relaying" && <span className="dro-spinner">◈</span>}
            {stage === "done"     && <span className="dro-icon-done">✓</span>}
            {stage === "error"    && <span className="dro-icon-error">✗</span>}
            <span className="dro-message">{message}</span>
          </div>

          {nxmUrl && (
            <div className="dro-url-detail">
              <div className="dro-url-label">NXM URL</div>
              <div className="dro-url-value">{nxmUrl}</div>
            </div>
          )}

          {stage === "error" && (
            <div className="dro-actions">
              <button className="dro-btn dro-btn-process" onClick={handleProcess}>
                Process here
              </button>
              <button className="dro-btn dro-btn-exit" onClick={handleExit}>
                Exit
              </button>
            </div>
          )}

          {stage === "done" && (
            <div className="dro-actions">
              <button className="dro-btn dro-btn-dismiss" onClick={handleDismiss}>
                Dismiss
              </button>
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
