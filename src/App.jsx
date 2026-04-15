import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as openUrl } from "@tauri-apps/plugin-shell";
import ModList from "./components/ModList";
import ModDetails from "./components/ModDetails";
import Settings from "./components/Settings";
import SyncOverlay from "./components/SyncOverlay";
import ConfirmDialog from "./components/ConfirmDialog";
import SplashScreen from "./components/SplashScreen";
import Manifest from "./components/Manifest";
import AppFooter from "./components/AppFooter";
import JackInOverlay from "./components/JackInOverlay";
import "./App.css";

function App() {
  const [mods, setMods] = useState([]);
  const [selectedMod, setSelectedMod] = useState(null);
  const [activeTab, setActiveTab] = useState("mods");
  const [loading, setLoading] = useState(false);
  const [booting, setBooting] = useState(true);
  const [statusMsg, setStatusMsg] = useState(null);
  const [syncProgress, setSyncProgress] = useState(null); // { current, total, modName }
  const [syncSummary, setSyncSummary] = useState(null); // { synced, total, updated, errors, cancelled }
  const [removeConfirm, setRemoveConfirm] = useState(null); // { modId, modName }
  const [forgetConfirm, setForgetConfirm] = useState(null); // { modId, modName }
  const [installProgress, setInstallProgress] = useState(null);
  const [closeConfirm, setCloseConfirm] = useState(false);
  const [nxmInput, setNxmInput] = useState(false);
  const [nxmUrl, setNxmUrl] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [modFilter, setModFilter] = useState("all");
  const [modSort, setModSort] = useState("recent");
  const searchExamples = ["search by name...", "search by author...", "search by version..."];
  const [phIdx, setPhIdx] = useState(0);
  const [phText, setPhText] = useState("");
  const [phTyping, setPhTyping] = useState(true);
  const nxmRef = useRef(null);
  const [searchFocused, setSearchFocused] = useState(false);
  const [hoverHint, setHoverHint] = useState(null);

  const hint = (text) => ({
    onMouseEnter: () => setHoverHint(text),
    onMouseLeave: () => setHoverHint(null),
  });

  useEffect(() => {
    const full = searchExamples[phIdx];
    let timer;
    if (phTyping) {
      if (phText.length < full.length) {
        timer = setTimeout(() => setPhText(full.slice(0, phText.length + 1)), 70);
      } else {
        timer = setTimeout(() => setPhTyping(false), 1800);
      }
    } else {
      if (phText.length > 0) {
        timer = setTimeout(() => setPhText(phText.slice(0, -1)), 35);
      } else {
        setPhIdx((i) => (i + 1) % searchExamples.length);
        setPhTyping(true);
      }
    }
    return () => clearTimeout(timer);
  }, [phText, phTyping, phIdx]);

  useEffect(() => {
    loadMods();

    // Reload mods when window gains focus — debounced, skips during install/sync
    let focusTimer = null;
    let busy = false;
    const onFocus = () => {
      if (busy) return;
      if (focusTimer) clearTimeout(focusTimer);
      focusTimer = setTimeout(() => {
        busy = true;
        loadMods().finally(() => { busy = false; });
      }, 500);
    };
    window.addEventListener("focus", onFocus);

    // Check for startup NXM URL — skip splash, process directly
    invoke("get_startup_nxm_url").then(async (url) => {
      if (url) {
        setBooting(false);
        setTimeout(() => handleInstallUrl(url), 300);
      }
    }).catch(() => {});

    // Check if splash is disabled in settings
    invoke("get_settings").then((settings) => {
      if (settings.show_splash === false) setBooting(false);
    }).catch(() => {});

    // Check and run first setup
    const runFirstSetup = async () => {
      try {
        const result = await invoke("check_and_run_first_setup");
        console.log("First setup check:", result);
      } catch (error) {
        console.error("Failed to run first setup:", error);
      }
    };

    runFirstSetup();

    // Health check — verify game path, permissions, URL handler
    invoke("check_startup_health").then((health) => {
      if (!health.healthy && health.issues?.length > 0) {
        const msgs = health.issues.map(i => i.message);
        setStatusMsg(`⚠ ${msgs.join(" · ")}`);
        console.warn("Startup health issues:", health.issues);
      }
    }).catch(() => {});

    // Listen for NXM URL events from the protocol handler
    const setupNxmListener = async () => {
      try {
        const unlisten = await listen("nxm-url-received", async (event) => {
          console.log("🔵 Received NXM URL event:", event.payload);

          // Log to Tauri backend as well
          try {
            await invoke("add_log_entry", {
              message: `🔵 Frontend: Received NXM URL event, about to call handle_nxm_url`,
              level: "info",
              category: "nxm_protocol",
            });
          } catch (e) {
            console.error("Failed to log to backend:", e);
          }

          // NXM URL received — logs are now in the footer panel

          try {
            console.log("🟡 About to invoke handle_nxm_url...");
            // Process the NXM URL
            await invoke("handle_nxm_url", { nxmUrl: event.payload });
            console.log("🟢 Successfully processed NXM URL from system");
          } catch (error) {
            console.error("🔴 Failed to process NXM URL from system:", error);
            alert("Failed to process NXM URL: " + error);

            // Try to log the error to backend
            try {
              await invoke("add_log_entry", {
                message: `🔴 Frontend error: ${error}`,
                level: "error",
                category: "nxm_protocol",
              });
            } catch (e) {
              console.error("Failed to log error to backend:", e);
            }
          }
        });

        // Cleanup function
        return unlisten;
      } catch (error) {
        console.error("Failed to setup NXM listener:", error);
      }
    };

    setupNxmListener();

    // Listen for mod-toggled events
    const setupModToggledListener = async () => {
      try {
        const unlisten = await listen("mod-toggled", () => {
          loadMods();
        });
        return unlisten;
      } catch (error) {
        console.error("Failed to setup mod-toggled listener:", error);
      }
    };
    setupModToggledListener();

    // Listen for mod-installed events to refresh the mod list (debounced)
    let modInstalledTimer = null;
    const setupModInstalledListener = async () => {
      try {
        const unlisten = await listen("mod-installed", async (event) => {
          // Debounce — backend may emit twice
          if (modInstalledTimer) clearTimeout(modInstalledTimer);
          modInstalledTimer = setTimeout(async () => {
            console.log("🎉 Mod installed event received:", event.payload);
            const modList = await loadMods();
            setActiveTab("mods");
            // Refresh selectedMod with updated data
            setSelectedMod((cur) => {
              if (!cur) return cur;
              return modList?.find(m => m.id === cur.id) || cur;
            });
          }, 300);
        });

        return unlisten;
      } catch (error) {
        console.error("Failed to setup mod-installed listener:", error);
      }
    };

    setupModInstalledListener();

    // Listen for collection-complete events
    const setupCollectionCompleteListener = async () => {
      try {
        const unlisten = await listen("collection-complete", async (event) => {
          console.log("🎉 Collection complete event received:", event.payload);

          // Log to backend
          try {
            await invoke("add_log_entry", {
              message: `🎉 Frontend: Collection installation complete, refreshing mod list`,
              level: "info",
              category: "installation",
            });
          } catch (e) {
            console.error("Failed to log to backend:", e);
          }

          // Refresh the mod list one final time
          await loadMods();

          // Switch to mods tab to show all newly installed mods
          setActiveTab("mods");
        });

        return unlisten;
      } catch (error) {
        console.error("Failed to setup collection-complete listener:", error);
      }
    };

    setupCollectionCompleteListener();

    // Listen for close-requested during installation
    const setupCloseRequestedListener = async () => {
      try {
        return await listen("close-requested", () => {
          setCloseConfirm(true);
        });
      } catch (e) {
        console.error("Failed to setup close-requested listener:", e);
      }
    };
    setupCloseRequestedListener();

    // Listen for install-progress events (verbose install overlay)
    const setupInstallProgressListener = async () => {
      try {
        return await listen("install-progress", (event) => {
          setInstallProgress(event.payload);
        });
      } catch (e) {
        console.error("Failed to setup install-progress listener:", e);
      }
    };
    setupInstallProgressListener();

  }, []);

  const loadMods = async () => {
    try {
      // Auto-deduplicate on load (cleans up legacy duplicates from pre-fix updates)
      try {
        const removed = await invoke("deduplicate_mods");
        if (removed.length > 0) console.log("Deduplicated:", removed);
      } catch {}
      console.log("Loading mods...");
      const modList = await invoke("get_installed_mods");
      console.log("Loaded mods:", modList.length, "mods");
      setMods(modList);
      const slotted = modList.filter(m => m.enabled && !m.removed).length;
      const ghosted = modList.filter(m => !m.enabled && !m.removed).length;
      const updates = modList.filter(m => m.update_available).length;
      let msg = `${modList.length} chrome modules · ${slotted} slotted · ${ghosted} ghosted`;
      if (updates > 0) msg += ` · ${updates} update${updates > 1 ? "s" : ""} available`;
      setStatusMsg(msg);
      return modList;
    } catch (error) {
      console.error("Failed to load mods:", error);

      // Log error to backend
      try {
        await invoke("add_log_entry", {
          message: `❌ Frontend: Failed to load mods: ${error}`,
          level: "error",
          category: "system",
        });
      } catch (e) {
        console.error("Failed to log error to backend:", e);
      }
    }
  };

  const [jackInConfirm, setJackInConfirm] = useState(null); // { mod }

  const handleJackInMod = (mod) => {
    if (!mod.mod_id) return;
    setJackInConfirm({ mod });
  };

  const doJackInMod = async () => {
    if (!jackInConfirm) return;
    const { mod } = jackInConfirm;
    setJackInConfirm(null);

    // If we know the latest file_id for this file, open direct download URL
    const targetFileId = mod.update_available && mod.latest_file_id
      ? mod.latest_file_id
      : null;

    try {
      if (targetFileId) {
        await openUrl(`https://www.nexusmods.com/Core/Libs/Common/Widgets/ModRequirementsPopUp?id=${targetFileId}&game_id=3333&nmm=1`);
        setStatusMsg(`waiting for NXM link · download popup opened on nexusmods.com`);
      } else {
        await openUrl(`https://www.nexusmods.com/cyberpunk2077/mods/${mod.mod_id}?tab=files`);
        setStatusMsg(`waiting for NXM link · click "Download with Mod Manager" on nexusmods.com`);
      }
    } catch {}
  };

  const handleReinstall = async (nxmUrl) => {
    setInstallProgress(null);
    setStatusMsg("reinstalling...");
    try {
      await invoke("set_force_reinstall");
      await invoke("handle_nxm_url", { nxmUrl });
    } catch (error) {
      console.error("Reinstall failed:", error);
      try { await invoke("abort_reinstall"); } catch {}
      setInstallProgress({
        stage: "error",
        message: String(error),
        nxm_url: nxmUrl,
      });
    }
  };

  const refreshStatus = () => {
    const total = mods.length;
    const slotted = mods.filter(m => m.enabled && !m.removed).length;
    const ghosted = mods.filter(m => !m.enabled && !m.removed).length;
    const updates = mods.filter(m => m.update_available).length;
    let msg = `${total} chrome modules · ${slotted} slotted · ${ghosted} ghosted`;
    if (updates > 0) msg += ` · ${updates} update${updates > 1 ? "s" : ""} available`;
    setStatusMsg(msg);
  };

  const handleInstallMod = async (modData) => {
    setLoading(true);
    try {
      await invoke("install_mod", { modData });
      await loadMods();
    } catch (error) {
      console.error("Failed to install mod:", error);
      alert("Failed to install mod: " + error);
    } finally {
      setLoading(false);
    }
  };

  const [syncOpen, setSyncOpen] = useState(false);

  const handleSyncMods = () => {
    setSyncOpen(true);
  };

  const handleInstallUrl = async (url) => {
    setStatusMsg("jacking in · processing NXM URL...");
    let failed = false;
    try {
      await invoke("handle_nxm_url", { nxmUrl: url });
    } catch (error) {
      failed = true;
      console.error("Failed to process NXM URL:", error);
      setStatusMsg(`✗ jack in failed: ${error}`);
      setInstallProgress({
        stage: "error",
        message: String(error),
        nxm_url: url,
      });
      try {
        await invoke("add_log_entry", {
          message: `Jack In failed: ${error}`,
          level: "error",
          category: "nxm_protocol",
        });
      } catch {}
    }
    // Backend may return Ok(()) without emitting progress for invalid URLs
    if (!failed) {
      setTimeout(() => {
        setInstallProgress((prev) => {
          // If still no progress event arrived, force error
          if (!prev || (prev.stage !== "downloading" && prev.stage !== "installing" && prev.stage !== "extracting" && prev.stage !== "registering" && prev.stage !== "done" && prev.stage !== "error")) {
            setStatusMsg("✗ jack in failed: no response from backend");
            return { stage: "error", message: "No response — URL may be invalid or backend failed silently", nxm_url: url };
          }
          return prev;
        });
      }, 2000);
    }
  };

  const handleUpdateAll = async () => {
    const updatable = mods.filter(m => m.update_available);
    if (updatable.length === 0) return;
    setStatusMsg(`blackwall engaged · updating ${updatable.length} mod${updatable.length > 1 ? "s" : ""}...`);
    try {
      await invoke("update_all_mods");
      await loadMods();
      setStatusMsg(`blackwall complete · ${updatable.length} mod${updatable.length > 1 ? "s" : ""} updated`);
    } catch (error) {
      console.error("Blackwall update failed:", error);
      setStatusMsg(`✗ blackwall failed: ${error}`);
    }
  };

  const startSync = async () => {
    setSyncProgress({ current: 0, total: 0, modName: "" });

    const { listen } = await import("@tauri-apps/api/event");

    const unlistenProgress = await listen("sync-progress", (event) => {
      setSyncProgress({
        current: event.payload.current,
        total: event.payload.total,
        modName: event.payload.mod_name,
        error: event.payload.error || null,
        version: event.payload.version || null,
        updateAvailable: event.payload.update_available || false,
      });
    });

    const unlistenComplete = await listen("sync-complete", (event) => {
      setSyncSummary(event.payload);
    });

    setStatusMsg("netrunning · fetching mod data from nexus...");
    try {
      await invoke("sync_mod_data");
      await loadMods();
    } catch (error) {
      console.error("Sync failed:", error);
      setStatusMsg(`✗ netrun failed: ${error}`);
      alert("Sync failed: " + error);
    } finally {
      unlistenProgress();
      unlistenComplete();
    }
  };

  const handleToggleMod = (modId, nowEnabled) => {
    // Оновлюємо стан локально без повного перезавантаження
    setMods((prev) =>
      prev.map((m) => (m.id === modId ? { ...m, enabled: nowEnabled } : m))
    );
    setSelectedMod((prev) =>
      prev?.id === modId ? { ...prev, enabled: nowEnabled } : prev
    );
  };

  const handleRemoveMod = (modId) => {
    const modName = mods.find((m) => m.id === modId)?.name ?? "this mod";
    setRemoveConfirm({ modId, modName });
  };

  const doRemoveMod = async () => {
    if (!removeConfirm) return;
    const { modId, modName } = removeConfirm;
    setRemoveConfirm(null);
    setLoading(true);

    try {
      await invoke("remove_mod", { modId });
      await loadMods();
      setModFilter("removed");
      setMods((cur) => {
        const flatlined = cur.find(m => m.id === modId);
        if (flatlined) setSelectedMod(flatlined);
        else setSelectedMod(null);
        return cur;
      });
      setStatusMsg(`flatlined: ${modName}`);
    } catch (error) {
      console.error("Failed to remove mod:", error);
      alert("Failed to remove mod: " + error);
    } finally {
      setLoading(false);
    }
  };

  const handleForgetMod = (modId, modName) => {
    setForgetConfirm({ modId, modName: modName ?? mods.find((m) => m.id === modId)?.name ?? "this mod" });
  };

  const doForgetMod = async () => {
    if (!forgetConfirm) return;
    const { modId, modName } = forgetConfirm;
    setForgetConfirm(null);
    setLoading(true);

    try {
      await invoke("forget_mod", { modId });
      await loadMods();
      setSelectedMod(null);
      setStatusMsg(`record purged: ${modName}`);
    } catch (error) {
      console.error("Failed to forget mod:", error);
      alert("Failed to forget mod: " + error);
    } finally {
      setLoading(false);
    }
  };

  return (
    <div className="app">
      {booting && <SplashScreen onDone={() => setBooting(false)} />}
      <header className="app-header">
        <div className="app-header-title">
          <h1 className="app-header-game">Cyberpunk 2077</h1>
          <p className="app-header-app">Crossover Mod Manager</p>
        </div>
        <nav className="nav">
          <button onClick={() => setNxmInput(true)} {...hint("install a mod from an nxm:// URL")}>Jack In</button>
          <button className={activeTab === "mods"     ? "active" : ""} onClick={() => setActiveTab("mods")} {...hint("browse and manage installed mods")}>Chrome</button>
          <button className={activeTab === "settings" ? "active" : ""} onClick={() => setActiveTab("settings")} {...hint("game paths, API key, and app settings")}>Config</button>
          <button className={activeTab === "manifest" ? "active" : ""} onClick={() => setActiveTab("manifest")} {...hint("version info, credits, and links")}>About</button>
        </nav>
      </header>

      {activeTab === "mods" && (
        <div className="action-bar">
          <div className="action-search-wrap">
            <input
              type="search"
              className="action-search"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              onFocus={() => setSearchFocused(true)}
              onBlur={() => setSearchFocused(false)}
            />
            {!searchQuery && !searchFocused && (
              <span className="action-search-placeholder">{phText}<span className="type-cursor" /></span>
            )}
          </div>
          <div className="radio-pills">
            {[
              ["all",      "All",       "show all slotted and ghosted chrome"],
              ["enabled",  "Slotted",   "show only slotted (active) mods"],
              ["disabled", "Ghosted",   "show only ghosted (inactive) mods"],
              ["updates",  "Updates",   "show mods with available updates"],
              ["removed",  "Flatlined", "show removed mods still in the database"],
            ].map(([v, l, h]) => (
              <button key={v} className={`pill ${modFilter === v ? "active" : ""}`} onClick={() => setModFilter(v)} {...hint(h)}>{l}</button>
            ))}
          </div>
          <div className="radio-pills">
            {[
              ["recent", "Recent", "sort by install date, newest first"],
              ["name",   "A–Z",   "sort alphabetically by mod name"],
            ].map(([v, l, h]) => (
              <button key={v} className={`pill ${modSort === v ? "active" : ""}`} onClick={() => setModSort(v)} {...hint(h)}>{l}</button>
            ))}
          </div>
        </div>
      )}

      <main className="app-content">
        {activeTab === "mods" ? (
          <div className="mod-manager">
            <div className="mod-list-pane">
              <ModList
                mods={mods}
                selectedMod={selectedMod}
                onSelectMod={setSelectedMod}
                searchQuery={searchQuery}
                filter={modFilter}
                sort={modSort}
                loading={loading}
                syncing={!!syncProgress}
                onSync={handleSyncMods}
                hint={hint}
              />
            </div>
            <div className="mod-details-pane">
              <ModDetails
                mod={selectedMod}
                siblings={selectedMod?._siblings || (selectedMod?.mod_id ? mods.filter(m => m.mod_id === selectedMod.mod_id && !m.removed) : [])}
                onSelectMod={setSelectedMod}
                onRemove={handleRemoveMod}
                onForget={handleForgetMod}
                onToggle={handleToggleMod}
                onJackIn={handleJackInMod}
                loading={loading}
                hint={hint}
              />
            </div>
          </div>
        ) : activeTab === "manifest" ? (
          <Manifest version="1.1.0" />
        ) : (
          <Settings hint={hint} onNavigateToMod={(modId) => {
            const mod = mods.find(m => m.id === modId);
            if (mod) { setSelectedMod(mod); setActiveTab("mods"); }
          }} />
        )}
      </main>

      {loading && (
        <div className="loading-overlay">
          <div className="spinner"></div>
          <p>Processing...</p>
        </div>
      )}

      <SyncOverlay
        open={syncOpen}
        syncProgress={syncProgress}
        mods={mods}
        onStart={startSync}
        onCancel={() => invoke("cancel_sync")}
        onClose={() => { setSyncOpen(false); setSyncProgress(null); setSyncSummary(null); }}
        syncSummary={syncSummary}
      />


      <ConfirmDialog
        open={!!removeConfirm}
        title="Flatline Mod"
        message={`All game files for "${removeConfirm?.modName}" will be deleted from disk. The record stays in your library under Flatlined.`}
        items={[
          { icon: "✗", label: "Deletes game files from disk" },
          { icon: "◈", label: "Keeps metadata record (Flatlined filter)" },
          { icon: "⚿", label: "Cannot be undone" },
        ]}
        confirmText="Flatline"
        cancelText="Cancel"
        danger
        onConfirm={doRemoveMod}
        onCancel={() => setRemoveConfirm(null)}
      />

      <ConfirmDialog
        open={!!forgetConfirm}
        title="Purge Record"
        message={`Permanently delete all metadata for "${forgetConfirm?.modName}" from your library. This cannot be undone.`}
        items={[
          { icon: "✗", label: "Deletes record permanently" },
        ]}
        confirmText="Forget"
        cancelText="Cancel"
        danger
        onConfirm={doForgetMod}
        onCancel={() => setForgetConfirm(null)}
      />

      <ConfirmDialog
        open={closeConfirm}
        title="Installation in Progress"
        message="A mod is currently being installed. Closing now will abort the installation. Are you sure?"
        items={[
          { icon: "⚠", label: "Download/installation will be interrupted" },
          { icon: "✗", label: "Partially copied files may remain" },
        ]}
        confirmText="Close Anyway"
        cancelText="Keep Installing"
        danger
        onConfirm={async () => {
          setCloseConfirm(false);
          try { await invoke("cancel_install"); } catch {}
          const { getCurrentWindow } = await import("@tauri-apps/api/window");
          getCurrentWindow().destroy();
        }}
        onCancel={() => setCloseConfirm(false)}
      />

      <ConfirmDialog
        open={!!jackInConfirm}
        title="Reinstall Mod"
        message={`Open NexusMods page for "${jackInConfirm?.mod?.name}" to download and reinstall.`}
        items={[
          { icon: "↗", label: "NexusMods will open in your browser" },
          { icon: "↓", label: "Click \"Download with Mod Manager\" on the Files tab" },
          { icon: "◈", label: "If already installed, you'll be offered to reinstall" },
        ]}
        confirmText="Open Nexus"
        cancelText="Cancel"
        onConfirm={doJackInMod}
        onCancel={() => setJackInConfirm(null)}
      />

      <JackInOverlay
        open={nxmInput || !!installProgress}
        progress={installProgress}
        onSubmit={handleInstallUrl}
        onRetry={() => setInstallProgress(null)}
        onReinstall={handleReinstall}
        onCancel={() => setNxmInput(false)}
        onDismiss={(reason) => {
          const wasSuccess = installProgress?.stage === "done";
          const modName = installProgress?.mod_name;
          setNxmInput(false);
          setInstallProgress(null);
          if (reason === "conflict-cancel") {
            setStatusMsg("installation skipped · mod already jacked in");
          } else if (wasSuccess && modName) {
            setStatusMsg(`✓ ${modName} jacked in successfully`);
          } else if (wasSuccess) {
            setStatusMsg("✓ mod jacked in successfully");
          } else {
            refreshStatus();
          }
          loadMods().then(() => {
            if (wasSuccess && modName) {
              // Select the newly installed mod and switch to Active filter
              setModFilter("all");
              setMods((cur) => {
                const installed = cur.find(m => m.name === modName && !m.removed);
                if (installed) setSelectedMod(installed);
                return cur;
              });
            } else {
              // Clear selection if it was a flatlined mod that got reinstalled
              setSelectedMod((cur) => {
                if (cur?.removed || cur?.reinstall_status) return null;
                return cur;
              });
            }
          });
        }}
      />

      <AppFooter version="1.1.0" build={__BUILD_ID__} status={statusMsg} hoverHint={hoverHint} />

    </div>
  );
}

export default App;
