import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./Logs.css";

function Logs() {
  const [logs, setLogs] = useState([]);
  const [autoRefresh, setAutoRefresh] = useState(true);
  const [filterLevel, setFilterLevel] = useState("all");
  const [filterCategory, setFilterCategory] = useState("all");

  useEffect(() => {
    loadLogs();
  }, []);

  useEffect(() => {
    if (autoRefresh) {
      const interval = setInterval(loadLogs, 1000); // Refresh every second
      return () => clearInterval(interval);
    }
  }, [autoRefresh]);

  const loadLogs = async () => {
    try {
      const logData = await invoke("get_logs");
      setLogs(logData);
    } catch (error) {
      console.error("Failed to load logs:", error);
    }
  };

  const clearLogs = async () => {
    try {
      await invoke("clear_logs");
      setLogs([]);
    } catch (error) {
      console.error("Failed to clear logs:", error);
    }
  };

  const filteredLogs = logs.filter((log) => {
    const levelMatch = filterLevel === "all" || log.level === filterLevel;
    const categoryMatch =
      filterCategory === "all" || log.category === filterCategory;
    return levelMatch && categoryMatch;
  });

  const getLogLevelIcon = (level) => {
    switch (level) {
      case "error":
        return "❌";
      case "warning":
        return "⚠️";
      case "info":
        return "ℹ️";
      default:
        return "📝";
    }
  };

  const getCategoryIcon = (category) => {
    switch (category) {
      case "download":
        return "📥";
      case "installation":
        return "⚙️";
      case "system":
        return "🖥️";
      default:
        return "📋";
    }
  };

  return (
    <div className="logs">
      <div className="logs-header">
        <h2>Installation Logs</h2>
        <div className="logs-controls">
          <div className="filter-group">
            <label>Level:</label>
            <select
              value={filterLevel}
              onChange={(e) => setFilterLevel(e.target.value)}
            >
              <option value="all">All</option>
              <option value="info">Info</option>
              <option value="warning">Warning</option>
              <option value="error">Error</option>
            </select>
          </div>

          <div className="filter-group">
            <label>Category:</label>
            <select
              value={filterCategory}
              onChange={(e) => setFilterCategory(e.target.value)}
            >
              <option value="all">All</option>
              <option value="download">Download</option>
              <option value="installation">Installation</option>
              <option value="system">System</option>
            </select>
          </div>

          <label className="auto-refresh-toggle">
            <input
              type="checkbox"
              checked={autoRefresh}
              onChange={(e) => setAutoRefresh(e.target.checked)}
            />
            Auto-refresh
          </label>

          <button onClick={loadLogs} className="refresh-button">
            🔄 Refresh
          </button>

          <button onClick={clearLogs} className="clear-button">
            🗑️ Clear
          </button>
        </div>
      </div>

      <div className="logs-content">
        {filteredLogs.length === 0 ? (
          <div className="no-logs">
            <p>No logs to display</p>
            <p className="help-text">
              Logs will appear here when you download or install mods
            </p>
          </div>
        ) : (
          <div className="logs-list">
            {filteredLogs.map((log, index) => (
              <div key={index} className={`log-entry log-${log.level}`}>
                <div className="log-header">
                  <span className="log-icons">
                    {getLogLevelIcon(log.level)} {getCategoryIcon(log.category)}
                  </span>
                  <span className="log-timestamp">{log.timestamp}</span>
                  <span className={`log-level log-level-${log.level}`}>
                    {log.level.toUpperCase()}
                  </span>
                  <span className={`log-category log-category-${log.category}`}>
                    {log.category}
                  </span>
                </div>
                <div className="log-message">{log.message}</div>
              </div>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

export default Logs;
