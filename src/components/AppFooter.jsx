import { useEffect, useState, useCallback, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import Logs from "./Logs";
import "./AppFooter.css";

export default function AppFooter({ version, build, status, hoverHint }) {
  const [glitch, setGlitch] = useState(false);
  const [isDev, setIsDev] = useState(false);
  const [buildTs, setBuildTs] = useState("");
  const [logsState, setLogsState] = useState("closed");
  const [lastLog, setLastLog] = useState(null);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const logsRef = useRef(null);

  useEffect(() => {
    invoke("is_dev_build").then(setIsDev).catch(() => {});
    invoke("get_build_timestamp").then(setBuildTs).catch(() => {});
  }, []);

  useEffect(() => {
    const schedule = () => {
      const delay = 18000 + Math.random() * 25000;
      return setTimeout(() => {
        setGlitch(true);
        setTimeout(() => setGlitch(false), 420);
        timerRef = schedule();
      }, delay);
    };
    let timerRef = schedule();
    return () => clearTimeout(timerRef);
  }, []);

  const toggleLogs = () => {
    if (logsState === "closed") setLogsState("open");
    else if (logsState === "open") setLogsState("closing");
  };

  const handleAnimationEnd = () => {
    if (logsState === "closing") setLogsState("closed");
  };

  const handleLastLog = useCallback((log) => setLastLog(log), []);

  const handleAutoToggle = () => {
    logsRef.current?.toggleAutoRefresh();
    setAutoRefresh((v) => !v);
  };

  const handleClear = () => logsRef.current?.clear();

  const displayStatus = hoverHint || status || (lastLog ? lastLog.message : null);
  const isOpen = logsState === "open";

  return (
    <>
      {logsState !== "closed" && (
        <div
          className={`footer-logs-panel ${logsState === "closing" ? "closing" : ""}`}
          onAnimationEnd={handleAnimationEnd}
        >
          <div className="footer-logs-titlebar">
            <span className="footer-logs-title">system log</span>
            <div className="footer-logs-actions">
              <button
                className={`footer-logs-btn ${autoRefresh ? "on" : ""}`}
                onClick={handleAutoToggle}
              >
                auto
              </button>
              <button className="footer-logs-btn danger" onClick={handleClear}>
                clear
              </button>
            </div>
            <button className="footer-logs-close" onClick={toggleLogs}>
              ✕ close
            </button>
          </div>
          <Logs ref={logsRef} onLastLog={handleLastLog} />
        </div>
      )}
      <footer className="app-footer">
        <div
          className={`app-footer-status ${isOpen ? "active" : ""}`}
          onClick={toggleLogs}
          title="Click to toggle system log"
        >
          <span className="app-footer-status-indicator">{isOpen ? "▲" : "▶"}</span>
          <span className="app-footer-status-text">
            {hoverHint ? (
              <span className="app-footer-hint" key={hoverHint}>{hoverHint}</span>
            ) : displayStatus ? displayStatus : <span className="app-footer-idle">·</span>}
          </span>
        </div>
        <div className={`app-footer-version ${glitch ? "glitch" : ""}`}>
          v{version}{build && <span className="app-footer-build"> #{build}</span>}{isDev && <span className="app-footer-dev"> DEV</span>}
        </div>
      </footer>
    </>
  );
}
