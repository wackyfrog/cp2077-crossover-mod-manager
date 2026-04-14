import { useEffect, useState, useRef, useImperativeHandle, forwardRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./Logs.css";

const Logs = forwardRef(function Logs({ onLastLog }, ref) {
  const [logs, setLogs] = useState([]);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [filterLevel, setFilterLevel] = useState("all");
  const [filterCategory, setFilterCategory] = useState("all");
  const bottomRef = useRef(null);

  useImperativeHandle(ref, () => ({
    get autoRefresh() { return autoRefresh; },
    toggleAutoRefresh() { setAutoRefresh((v) => !v); },
    clear: async () => {
      try { await invoke("clear_logs"); setLogs([]); } catch {}
    },
  }), [autoRefresh]);

  useEffect(() => { loadLogs(); }, []);

  useEffect(() => {
    if (autoRefresh) {
      const interval = setInterval(loadLogs, 1000);
      return () => clearInterval(interval);
    }
  }, [autoRefresh]);

  useEffect(() => {
    if (onLastLog && logs.length > 0) onLastLog(logs[logs.length - 1]);
  }, [logs, onLastLog]);

  const firstLoadRef = useRef(true);

  // Scroll to bottom on first load, then only if near bottom
  useEffect(() => {
    const el = bottomRef.current?.parentElement;
    if (!el) return;
    if (firstLoadRef.current && logs.length > 0) {
      firstLoadRef.current = false;
      bottomRef.current?.scrollIntoView();
      return;
    }
    const nearBottom = el.scrollHeight - el.scrollTop - el.clientHeight < 60;
    if (nearBottom) bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [logs]);

  const loadLogs = async () => {
    try { setLogs(await invoke("get_logs")); } catch {}
  };

  const filteredLogs = logs.filter((log) => {
    return (filterLevel === "all" || log.level === filterLevel)
      && (filterCategory === "all" || log.category === filterCategory);
  });

  const formatTime = (ts) => {
    if (!ts) return "";
    try {
      // Parse "2026-04-13 08:32:36 UTC" → local time
      const d = new Date(ts.replace(" UTC", "Z").replace(" ", "T"));
      if (!isNaN(d)) return d.toLocaleTimeString("en-GB", { hour: "2-digit", minute: "2-digit", second: "2-digit" });
    } catch {}
    const match = ts.match(/(\d{2}:\d{2}:\d{2})/);
    return match ? match[1] : ts;
  };

  const levelPrefix = (level) => {
    switch (level) {
      case "error":   return "ERR";
      case "warning": return "WRN";
      case "info":    return "INF";
      default:        return "LOG";
    }
  };

  return (
    <div className="logs">
      <div className="logs-toolbar">
        <div className="logs-filters">
          <select value={filterLevel} onChange={(e) => setFilterLevel(e.target.value)}>
            <option value="all">ALL</option>
            <option value="info">INF</option>
            <option value="warning">WRN</option>
            <option value="error">ERR</option>
          </select>
          <select value={filterCategory} onChange={(e) => setFilterCategory(e.target.value)}>
            <option value="all">ALL</option>
            <option value="download">DOWNLOAD</option>
            <option value="installation">INSTALL</option>
            <option value="system">SYSTEM</option>
            <option value="nxm_protocol">NXM</option>
          </select>
        </div>
      </div>

      <div className="logs-terminal">
        {filteredLogs.length === 0 ? (
          <div className="logs-empty">no log entries</div>
        ) : (
          filteredLogs.map((log, i) => (
            <div key={i} className={`log-line log-line-${log.level}`}>
              <span className="log-ts">{formatTime(log.timestamp)}</span>
              <span className={`log-lvl log-lvl-${log.level}`}>[{levelPrefix(log.level)}]</span>
              <span className="log-cat">{log.category}</span>
              <span className="log-msg">{log.message}</span>
            </div>
          ))
        )}
        <div ref={bottomRef} />
      </div>
    </div>
  );
});

export default Logs;
