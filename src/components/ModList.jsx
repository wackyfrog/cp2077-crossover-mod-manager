import { invoke } from "@tauri-apps/api/core";
import "./ModList.css";

function ModList({ mods, selectedMod, onSelectMod, loading }) {
  const testInstallation = async () => {
    try {
      const testModData = {
        name: "Test Mod",
        version: "1.0.0",
        author: "Test Author",
        download_url: "https://example.com/test-mod.zip",
      };

      await invoke("install_mod", { modData: testModData });
    } catch (error) {
      console.error("Test installation failed:", error);
    }
  };

  return (
    <div className="mod-list">
      <div className="mod-list-header">
        <h2>Installed Mods</h2>
        <div className="mod-list-actions">
          <span className="mod-count">
            {mods.length} mod{mods.length !== 1 ? "s" : ""}
          </span>
          <button
            onClick={testInstallation}
            className="test-button"
            disabled={loading}
          >
            🧪 Test Installation
          </button>
        </div>
      </div>

      <div className="mod-list-content">
        {mods.length === 0 ? (
          <div className="empty-state">
            <p>No mods installed yet</p>
            <p className="help-text">
              Click "Download with Mod Manager" on NexusMods to install mods
            </p>
          </div>
        ) : (
          mods.map((mod) => (
            <div
              key={mod.id}
              className={`mod-item ${
                selectedMod?.id === mod.id ? "selected" : ""
              }`}
              onClick={() => onSelectMod(mod)}
            >
              <div className="mod-info">
                <h3>{mod.name}</h3>
                <p className="mod-version">v{mod.version}</p>
              </div>
              <div
                className={`mod-status ${mod.enabled ? "enabled" : "disabled"}`}
              >
                {mod.enabled ? "✓" : "○"}
              </div>
            </div>
          ))
        )}
      </div>
    </div>
  );
}

export default ModList;
