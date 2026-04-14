import { useEffect, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import SyncTerminal from "./SyncTerminal";
import "./SyncOverlay.css";

export default function SyncOverlay({ open, syncProgress, mods, onStart, onCancel, onClose, syncSummary }) {
  const prevModName = useRef(null);
  const terminalRef = useRef(null); // { addRealLine, reset }

  const phase = syncSummary ? "done" : syncProgress ? "syncing" : "info";
  const syncable = mods?.filter(m => m.mod_id && !m.removed).length || 0;
  const skipped = mods?.filter(m => !m.mod_id && !m.removed).length || 0;

  // Listen directly for sync-progress events (not via App.jsx proxy)
  // This ensures we catch every event including errors that might be batched away
  useEffect(() => {
    if (phase !== "syncing" && phase !== "done") return;

    let unlisten;
    listen("sync-progress", (event) => {
      const t = terminalRef.current;
      if (!t) return;
      const p = event.payload;
      if (!p.mod_name) return;

      const isError = !!p.error;
      // Deduplicate only non-errors with same name
      if (!isError && p.mod_name === prevModName.current) return;
      prevModName.current = p.mod_name;

      const cur = p.current || 0;
      const total = p.total || 1;
      const ver = p.version ? ` v${p.version}` : "";
      const hasUpdate = p.update_available;

      if (isError) {
        t.addRealLine(`[${cur}/${total}] ✗ ${p.mod_name}${ver} — ${p.error}`, "error");
      } else if (hasUpdate) {
        t.addRealLine(`[${cur}/${total}] ${p.mod_name}${ver} ↑ update available`, "update");
      } else {
        t.addRealLine(`[${cur}/${total}] ${p.mod_name}${ver}`);
      }
    }).then((u) => { unlisten = u; });

    return () => { if (unlisten) unlisten(); };
  }, [phase]);

  // Summary lines
  useEffect(() => {
    if (!syncSummary || !open || !terminalRef.current) return;
    const t = terminalRef.current;
    t.addRealLine("", "divider");
    // Force scroll to bottom for summary
    setTimeout(() => {
      const el = document.querySelector(".sync-terminal");
      if (el) el.scrollTop = el.scrollHeight;
    }, 100);
    t.addRealLine(`> ✓ synced ${syncSummary.synced} / ${syncSummary.total} modules`, "sum-ok");
    if (syncSummary.updated > 0)
      t.addRealLine(`> ↑ ${syncSummary.updated} update${syncSummary.updated > 1 ? "s" : ""} available`, "sum-update");
    if (syncSummary.errors > 0)
      t.addRealLine(`> ✗ ${syncSummary.errors} error${syncSummary.errors > 1 ? "s" : ""}`, "sum-error");
    if (syncSummary.cancelled)
      t.addRealLine("> ⚠ cancelled before completion", "sum-warn");
    t.addRealLine("> netrun complete", "sum-ok");
    setTimeout(() => {
      const el = document.querySelector(".sync-terminal");
      if (el) el.scrollTop = el.scrollHeight;
    }, 200);
  }, [syncSummary]);

  // Reset terminal on open
  useEffect(() => {
    if (open) {
      prevModName.current = null;
      terminalRef.current?.reset();
    }
  }, [open]);

  if (!open) return null;

  const pct = syncProgress?.total > 0
    ? Math.round((syncProgress.current / syncProgress.total) * 100) : 0;

  return (
    <div className="sync-overlay">
      <div className="sync-chrome-bar">
        <span className="sync-chrome-tag">NEXUS UPLINK</span>
        <span className="sync-chrome-divider" />
        <span className="sync-chrome-sys">SYNC PROTOCOL v2.0</span>
        <span className="sync-chrome-divider" />
        <span className="sync-chrome-status">
          {phase === "info" ? "AWAITING COMMAND" : phase === "syncing" ? "ACTIVE SESSION" : "COMPLETE"}
        </span>
      </div>

      <div className="sync-header">
        <div className="sync-title-row">
          <div className="sync-title">Netrun</div>
          {phase === "syncing" && <div className="sync-pct">{pct}%</div>}
          {phase === "done" && <div className="sync-pct-done">DONE</div>}
        </div>

        {phase === "info" && (
          <div className="sync-info-terminal">
            <div className="sync-info-line dim">&gt; querying nexus mod registry ...</div>
            <div className="sync-info-line">&gt; <span className="glow-green">{syncable}</span> modules with Nexus ID queued for sync</div>
            {skipped > 0 && <div className="sync-info-line dim">&gt; {skipped} modules without Nexus ID — skipped</div>}
            <div className="sync-info-line dim">&gt; fetching per module: <span className="glow-cyan">description</span> · <span className="glow-cyan">thumbnail</span> · <span className="glow-cyan">version</span> · <span className="glow-cyan">file names</span></div>
            <div className="sync-info-line dim">&gt; update check: compares installed vs <span className="glow-yellow">latest on Nexus</span></div>
            <div className="sync-info-line dim">&gt; requires API key in Config</div>
            <div className="sync-info-line">&gt; awaiting command <span className="sync-cursor" /></div>
          </div>
        )}

        {phase === "syncing" && (
          <>
            <div className="sync-message">{syncProgress?.modName}</div>
            <div className="sync-progress-duo">
              <div className="sync-progress-sharp" style={{ width: `${pct}%` }} />
              <div className="sync-progress-glow" style={{ width: `${pct}%` }} />
            </div>
            <div className="sync-stats">{syncProgress?.current}/{syncProgress?.total} modules</div>
          </>
        )}
      </div>

      {phase !== "info" ? (
        <SyncTerminal
          active={phase === "syncing"}
          onApi={(api) => { terminalRef.current = api; }}
        />
      ) : (
        <div className="sync-spacer" />
      )}

      <div className="sync-footer">
        <div className="sync-footer-info">
          {phase === "syncing" && (
            <span className="sync-footer-readout">{pct}% // {syncProgress?.current}/{syncProgress?.total} modules</span>
          )}
        </div>
        <div className="sync-footer-actions">
          {phase === "info" && (
            <>
              <button className="sync-btn cancel" onClick={onClose}>Disconnect</button>
              <button className="sync-btn primary" onClick={onStart}>Start</button>
            </>
          )}
          {phase === "syncing" && (
            <button className="sync-btn danger" onClick={onCancel}>Abort</button>
          )}
          {phase === "done" && (
            <button className="sync-btn primary" onClick={onClose}>Disconnect</button>
          )}
        </div>
      </div>
    </div>
  );
}
