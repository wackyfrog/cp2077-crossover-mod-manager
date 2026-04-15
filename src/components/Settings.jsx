import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import "./Settings.css";

function Settings({ hint = () => ({}), onNavigateToMod }) {
  const [gamePath, setGamePath] = useState("");
  const [modStoragePath, setModStoragePath] = useState("");
  const [nexusmodsApiKey, setNexusmodsApiKey] = useState("");
  const [showSplash, setShowSplash] = useState(true);
  const [loading, setLoading] = useState(false);
  const [saved, setSaved] = useState(false);
  const [autoDetecting, setAutoDetecting] = useState(false);
  const [detectionResult, setDetectionResult] = useState("");

  useEffect(() => {
    loadSettings();
  }, []);

  const loadSettings = async () => {
    try {
      const settings = await invoke("get_settings");
      setGamePath(settings.game_path || "");
      setModStoragePath(settings.mod_storage_path || "");
      setNexusmodsApiKey(settings.nexusmods_api_key || "");
      setShowSplash(settings.show_splash !== false);
    } catch (error) {
      console.error("Failed to load settings:", error);
    }
  };

  const selectGamePath = async () => {
    try {
      const crossoverBottles = await invoke("get_crossover_bottles_path");
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: "Select Game Installation Directory",
        defaultPath: crossoverBottles || undefined,
      });
      if (selected) setGamePath(selected);
    } catch (error) {
      console.error("Failed to select directory:", error);
    }
  };

  const selectModStoragePath = async () => {
    try {
      const selected = await openDialog({
        directory: true,
        multiple: false,
        title: "Select Mod Storage Directory",
      });
      if (selected) setModStoragePath(selected);
    } catch (error) {
      console.error("Failed to select directory:", error);
    }
  };

  const autoDetectGamePath = async () => {
    setAutoDetecting(true);
    setDetectionResult("");
    try {
      const detectedPath = await invoke("auto_detect_game_path");
      if (detectedPath) {
        setGamePath(detectedPath);
        setDetectionResult("Game installation detected");
        setTimeout(() => setDetectionResult(""), 5000);
      } else {
        setDetectionResult("Could not detect game installation");
        setTimeout(() => setDetectionResult(""), 5000);
      }
    } catch (error) {
      console.error("Failed to auto-detect game path:", error);
      setDetectionResult("Auto-detection failed");
      setTimeout(() => setDetectionResult(""), 5000);
    } finally {
      setAutoDetecting(false);
    }
  };

  const saveSettings = async () => {
    setLoading(true);
    setSaved(false);
    try {
      await invoke("save_settings", {
        settings: {
          game_path: gamePath,
          mod_storage_path: modStoragePath,
          nexusmods_api_key: nexusmodsApiKey,
          show_splash: showSplash,
        },
      });
      setSaved(true);
      setTimeout(() => setSaved(false), 3000);
    } catch (error) {
      console.error("Failed to save settings:", error);
    } finally {
      setLoading(false);
    }
  };

  const [cleaningTemp, setCleaningTemp] = useState(false);
  const [cleanupResult, setCleanupResult] = useState("");
  const [dedupResult, setDedupResult] = useState("");
  const [validating, setValidating] = useState(false);
  const [validationResult, setValidationResult] = useState(null);
  const [backups, setBackups] = useState([]);
  const [backupMsg, setBackupMsg] = useState("");
  const [confirmAction, setConfirmAction] = useState(null); // { type, name, label }

  const loadBackups = async () => {
    try {
      const list = await invoke("list_backups");
      setBackups(list);
    } catch {}
  };

  useEffect(() => { loadBackups(); }, []);

  const formatBackupDate = (name) => {
    // Parse from filename: mods_YYYYMMDD_HHMMSS.json
    const m = name?.match(/mods_(\d{4})(\d{2})(\d{2})_(\d{2})(\d{2})(\d{2})\.json/);
    if (!m) return name || "";
    return `${m[3]}.${m[2]}.${m[1]}, ${m[4]}:${m[5]}:${m[6]}`;
  };

  const formatSize = (bytes) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const doBackup = async () => {
    try {
      const name = await invoke("backup_database");
      setBackupMsg(`Backup created: ${name}`);
      setTimeout(() => setBackupMsg(""), 5000);
      loadBackups();
    } catch (e) {
      setBackupMsg(`Failed: ${e}`);
      setTimeout(() => setBackupMsg(""), 5000);
    }
  };

  const doRestore = async (name) => {
    try {
      await invoke("restore_backup", { name });
      setBackupMsg("Restored — reloading mods...");
      setTimeout(() => setBackupMsg(""), 5000);
      // Trigger full reload by dispatching event
      window.dispatchEvent(new Event("focus"));
    } catch (e) {
      setBackupMsg(`Restore failed: ${e}`);
      setTimeout(() => setBackupMsg(""), 5000);
    }
  };

  const doDeleteBackup = async (name) => {
    try {
      await invoke("delete_backup", { name });
      setBackupMsg("Backup deleted");
      setTimeout(() => setBackupMsg(""), 3000);
      loadBackups();
    } catch (e) {
      setBackupMsg(`Delete failed: ${e}`);
      setTimeout(() => setBackupMsg(""), 5000);
    }
  };

  const executeConfirm = () => {
    if (!confirmAction) return;
    const { type, name } = confirmAction;
    setConfirmAction(null);
    if (type === "backup") doBackup();
    else if (type === "restore") doRestore(name);
    else if (type === "delete") doDeleteBackup(name);
  };

  const cleanTempFiles = async () => {
    setCleaningTemp(true);
    setCleanupResult("");
    try {
      const result = await invoke("clean_temp_files");
      setCleanupResult(result);
      setTimeout(() => setCleanupResult(""), 5000);
    } catch (error) {
      setCleanupResult(`Failed: ${error}`);
      setTimeout(() => setCleanupResult(""), 5000);
    } finally {
      setCleaningTemp(false);
    }
  };

  return (
    <div className="settings">
      <div className="settings-content">

        <div className="setting-section">
          <h3>Game</h3>

          <div className="setting-row">
            <label>Game path</label>
            <div className="path-selector">
              <input
                type="text"
                value={gamePath}
                onChange={(e) => setGamePath(e.target.value)}
                placeholder="Select your Cyberpunk 2077 installation..."
                {...hint("path to Cyberpunk 2077 in CrossOver bottle")}
              />
              <div className="path-buttons">
                <button
                  onClick={autoDetectGamePath}
                  disabled={autoDetecting}
                  className="auto-detect-button"
                  {...hint("scan CrossOver bottles for Cyberpunk 2077")}
                >
                  {autoDetecting ? "Detecting..." : "Auto-Detect"}
                </button>
                <button onClick={selectGamePath} {...hint("pick game folder manually")}>Browse</button>
              </div>
            </div>
            {detectionResult && (
              <p className="detection-result">{detectionResult}</p>
            )}
          </div>

          <div className="setting-row">
            <label>NexusMods API key</label>
            <div className="path-selector">
              <input
                type="password"
                value={nexusmodsApiKey}
                onChange={(e) => setNexusmodsApiKey(e.target.value)}
                placeholder="Enter your NexusMods API key..."
                {...hint("required for sync and downloading mods via NXM links")}
              />
            </div>
          </div>
        </div>

        <div className="setting-section">
          <h3>Interface</h3>
          <div className="setting-row setting-toggle-row">
            <label className="toggle-label">
              <input
                type="checkbox"
                checked={showSplash}
                onChange={(e) => setShowSplash(e.target.checked)}
              />
              <span {...hint("show boot sequence animation on app launch")}>Show splash screen on startup</span>
            </label>
          </div>
        </div>

        <div className="setting-section">
          <h3>Maintenance</h3>
          <div className="setting-row">
            <button
              onClick={cleanTempFiles}
              className="maintenance-button"
              disabled={cleaningTemp}
              {...hint("remove leftover archives and extraction dirs from /tmp")}
            >
              {cleaningTemp ? "Cleaning..." : "Clean temporary files"}
            </button>
            {cleanupResult && (
              <p className="detection-result">{cleanupResult}</p>
            )}
          </div>
          <div className="setting-row">
            <button
              onClick={async () => {
                setDedupResult("");
                try {
                  const removed = await invoke("deduplicate_mods");
                  setDedupResult(removed.length > 0 ? `Removed ${removed.length} duplicate(s)` : "No duplicates found");
                  setTimeout(() => setDedupResult(""), 5000);
                } catch (e) {
                  setDedupResult(`Failed: ${e}`);
                  setTimeout(() => setDedupResult(""), 5000);
                }
              }}
              className="maintenance-button"
              {...hint("find and remove duplicate mod entries from the database")}
            >
              Remove duplicate records
            </button>
            {dedupResult && (
              <p className="detection-result">{dedupResult}</p>
            )}
          </div>
          <div className="setting-row">
            <button
              onClick={async () => {
                setValidating(true);
                setValidationResult(null);
                try {
                  const r = await invoke("validate_mod_files");
                  setValidationResult(r);
                } catch (e) {
                  setValidationResult({ error: String(e) });
                } finally {
                  setValidating(false);
                }
              }}
              className="maintenance-button"
              disabled={validating}
              {...hint("check that all mod files exist on disk")}
            >
              {validating ? "Validating..." : "Validate mod files"}
            </button>
          </div>
        </div>

        <div className="setting-section">
          <h3>Database Backup</h3>
          <div className="setting-row">
            <button
              onClick={() => setConfirmAction({ type: "backup", label: "Create a backup of the mod database? Only the database record is saved, mod files are not copied." })}
              className="maintenance-button"
              {...hint("create a backup of the mod database (mod files are not copied)")}
            >
              Create backup
            </button>
            {backupMsg && <p className="detection-result">{backupMsg}</p>}
          </div>
          {backups.length > 0 && (
            <div className="backup-list">
              {backups.map((b) => (
                <div key={b.name} className="backup-item">
                  <div className="backup-info">
                    <span className="backup-date">{formatBackupDate(b.name)}</span>
                    <span className="backup-size">{formatSize(b.size)}</span>
                  </div>
                  <div className="backup-actions">
                    <button
                      className="backup-action-btn backup-restore"
                      onClick={() => setConfirmAction({ type: "restore", name: b.name, label: `Restore backup from ${formatBackupDate(b.name)}? This will replace the current database. Mod files on disk are not affected.` })}
                      {...hint("replace current database with this backup")}
                    >
                      Restore
                    </button>
                    <button
                      className="backup-action-btn backup-delete"
                      onClick={() => setConfirmAction({ type: "delete", name: b.name, label: `Delete backup from ${formatBackupDate(b.name)}? This only removes the backup file, not the current database or mod files.` })}
                      {...hint("permanently delete this backup file")}
                    >
                      Delete
                    </button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </div>

        {confirmAction && (
          <div className="backup-confirm-backdrop" onClick={() => setConfirmAction(null)}>
            <div className="backup-confirm" onClick={(e) => e.stopPropagation()}>
              <p className="backup-confirm-text">{confirmAction.label}</p>
              <div className="backup-confirm-actions">
                <button className="backup-confirm-no" onClick={() => setConfirmAction(null)}>Cancel</button>
                <button className="backup-confirm-yes" onClick={executeConfirm}>Confirm</button>
              </div>
            </div>
          </div>
        )}

        <div className="settings-actions">
          <button
            className="save-button"
            onClick={saveSettings}
            disabled={loading || !gamePath}
            {...hint("save all settings to disk")}
          >
            {loading ? "Saving..." : "Save"}
          </button>
          {saved && (
            <span className="save-success">Saved</span>
          )}
        </div>
      </div>

      {validationResult && (
        <div className="validation-backdrop" onClick={() => setValidationResult(null)}>
          <div className="validation-modal" onClick={(e) => e.stopPropagation()}>
            <div className="validation-header">
              <span className="validation-title">File Validation</span>
              <button className="validation-close" onClick={() => setValidationResult(null)}>✕</button>
            </div>
            <div className="validation-body">
              {validationResult.error ? (
                <p className="validation-error">Failed: {validationResult.error}</p>
              ) : validationResult.missing_files === 0 ? (
                <p className="validation-ok">
                  All files OK — {validationResult.total_files} files across {validationResult.total_mods} mods
                </p>
              ) : (
                <>
                  <p className="validation-summary">
                    {validationResult.missing_files} missing file{validationResult.missing_files > 1 ? "s" : ""} in {validationResult.affected_mods.length} mod{validationResult.affected_mods.length > 1 ? "s" : ""} (out of {validationResult.total_files} total)
                  </p>
                  {validationResult.affected_mods.map((m, i) => (
                    <div key={i} className="validation-mod">
                      <div
                        className="validation-mod-name"
                        onClick={() => { setValidationResult(null); onNavigateToMod?.(m.id); }}
                      >
                        {m.name} ({m.missing.length}/{m.total} missing)
                      </div>
                      {m.missing.map((f, j) => (
                        <div key={j} className="validation-file">{f}</div>
                      ))}
                    </div>
                  ))}
                </>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

export default Settings;
