import { useState, useEffect, useRef, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open as openUrl } from "@tauri-apps/plugin-shell";
import { open as openDialog } from "@tauri-apps/plugin-dialog";
import ModList from "./components/ModList";
import ModDetails from "./components/ModDetails";
import DevRelayOverlay from "./components/DevRelayOverlay";
import Settings from "./components/Settings";
import SyncOverlay from "./components/SyncOverlay";
import ConfirmDialog from "./components/ConfirmDialog";
import SplashScreen from "./components/SplashScreen";
import Manifest from "./components/Manifest";
import AppFooter from "./components/AppFooter";
import JackInOverlay from "./components/JackInOverlay";
import SideloadOverlay from "./components/SideloadOverlay";
import useEscape from "./hooks/useEscape";
import "./App.css";

const SIDELOAD_EXTENSIONS = ["zip", "7z", "rar"];

/// Name a turned-away request without printing its NXM link in full: the query
/// string carries a download key, and this line is exactly what ends up in the
/// screenshots people attach to bug reports. Anything that isn't an NXM link
/// (a sideload path) is shown by file name only, for the same reason.
function shortenTurnedAway(detail) {
  if (!detail) return null;
  const nxm = detail.match(/^nxm:\/\/([^/]+)\/mods\/(\d+)(?:\/files\/(\d+))?/i);
  if (nxm) {
    const [, game, modId, fileId] = nxm;
    return `${game} mod ${modId}${fileId ? ` · file ${fileId}` : ""}`;
  }
  return detail.split("/").pop() || null;
}

function App() {
  const [mods, setMods] = useState([]);
  const [selectedMod, setSelectedMod] = useState(null);
  const [activeTab, setActiveTab] = useState("mods");
  const [loading, setLoading] = useState(false);
  const [booting, setBooting] = useState(true);
  const [statusMsg, setStatusMsg] = useState(null);
  const [healthIssues, setHealthIssues] = useState([]); // persistent startup/self-check warnings
  const [syncProgress, setSyncProgress] = useState(null); // { current, total, modName }
  const [syncSummary, setSyncSummary] = useState(null); // { synced, total, updated, errors, cancelled }
  const [removeConfirm, setRemoveConfirm] = useState(null); // { modId, modName }
  const [forgetConfirm, setForgetConfirm] = useState(null); // { modId, modName }
  const [installProgress, setInstallProgress] = useState(null);
  const [busyNotice, setBusyNotice] = useState(null); // { text, seq } — a turned-away request
  const [relayStatus, setRelayStatus] = useState(null);
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
  const [sideloadPath, setSideloadPath] = useState(null); // archive awaiting metadata confirmation
  const [dragActive, setDragActive] = useState(false);

  const hint = (text) => ({
    onMouseEnter: () => setHoverHint(text),
    onMouseLeave: () => setHoverHint(null),
  });

  // Bottom of the Escape stack: with no overlay open, Escape clears the search.
  // Registered first, so any overlay mounted later takes the key ahead of it.
  useEscape(!!searchQuery, () => setSearchQuery(""));

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

    // Self-check: verify game path is a valid, writable Cyberpunk 2077 install,
    // API key is set, and the NXM handler is registered. Surfaced as a persistent
    // banner (not a transient footer status) so it can't be missed or overwritten.
    const runHealthCheck = () =>
      invoke("check_startup_health")
        .then((health) => {
          const issues = !health.healthy && health.issues?.length > 0 ? health.issues : [];
          setHealthIssues(issues);
          if (issues.length > 0) console.warn("Self-check issues:", issues);
        })
        .catch(() => {});

    // Reload mods and re-run the self-check when the window gains focus —
    // debounced, skips during install/sync. Re-checking on focus means fixing
    // the game path in Config clears the banner without a restart.
    let focusTimer = null;
    let busy = false;
    const onFocus = () => {
      if (busy) return;
      if (focusTimer) clearTimeout(focusTimer);
      focusTimer = setTimeout(() => {
        busy = true;
        loadMods().finally(() => { busy = false; });
        runHealthCheck();
      }, 500);
    };
    window.addEventListener("focus", onFocus);

    // Check for startup NXM URL — skip splash, try relay, or process locally
    invoke("get_startup_nxm_url").then(async (url) => {
      if (url) {
        setBooting(false);
        try {
          const relayed = await invoke("try_relay", { nxmUrl: url });
          if (relayed) { setStatusMsg("relayed to dev instance"); return; }
        } catch {}
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

    // Run the self-check once on startup.
    runHealthCheck();

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
            // Select the mod that was just installed/updated
            const installedId = event.payload?.id;
            if (installedId && modList) {
              const mod = modList.find(m => m.id === installedId);
              if (mod) setSelectedMod(mod);
            }
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

    const setupRelayListener = async () => {
      try {
        return await listen("relay-status", (event) => {
          setRelayStatus(event.payload);
        });
      } catch {}
    };
    setupRelayListener();

    // A second install request was turned away while one was running. It goes
    // to the status bar *and* to the Jack In overlay: the overlay covers the
    // footer whenever an install is running, which is the only time this can
    // happen, so the status bar alone told the user nothing (docs/bugs.md B6).
    // The overlay renders it as its own kind of line, never as a failure of the
    // install that is still going.
    const setupBusyListener = async () => {
      try {
        return await listen("install-busy", (event) => {
          const { source, detail } = event.payload || {};
          const what = source === "sideload" ? "sideload" : "download";
          const label = shortenTurnedAway(detail);
          const text = `${what} ignored · already jacking in${label ? ` · queued: ${label}` : ""}`;
          setStatusMsg(`⏳ ${text}`);
          // seq, not the text, is what makes a repeated rejection show again.
          setBusyNotice((prev) => ({ text, seq: (prev?.seq ?? 0) + 1 }));
        });
      } catch (e) {
        console.error("Failed to setup install-busy listener:", e);
      }
    };
    setupBusyListener();
  }, []);

  // "An install is already under way, or about to be." Kept in a ref so the
  // drag&drop handler can consult it without being re-subscribed on every
  // progress tick. A pending sideload form counts: the user has picked an
  // archive and is one click from starting it.
  const installBusy = !!installProgress || !!syncProgress || !!sideloadPath;

  // Narrower than installBusy: work is *actually under way* right now. Once an
  // install ends, its overlay stays open showing the outcome — that state is
  // still "busy" for the header buttons, but it is exactly when Retry must
  // work. Anything mid-flight makes Retry refuse instead (docs/bugs.md B8).
  const installRunning =
    (!!installProgress &&
      installProgress.stage !== "done" &&
      installProgress.stage !== "error") ||
    !!syncProgress;

  // What actually blocks starting another install from an archive. A finished
  // run whose result is still on screen does NOT: dropping a second archive
  // after a failed one used to be swallowed, with the reason printed to the
  // footer the overlay covers (docs/bugs.md B9). A sideload form already open
  // does block — the user is one click from starting that one.
  const acceptingArchives = !installRunning && !sideloadPath;
  const acceptingRef = useRef(false);
  useEffect(() => {
    acceptingRef.current = acceptingArchives;
  }, [acceptingArchives]);

  // Native drag & drop. Tauri 2 intercepts file drops before the webview sees
  // them, so HTML5 onDrop/dataTransfer never fires — this is the only way to
  // get real on-disk paths.
  useEffect(() => {
    let unlisten = null;
    let disposed = false;

    (async () => {
      try {
        const { getCurrentWebview } = await import("@tauri-apps/api/webview");
        const un = await getCurrentWebview().onDragDropEvent((event) => {
          const { type, paths } = event.payload || {};

          if (type === "enter" || type === "over") {
            if (acceptingRef.current) setDragActive(true);
            return;
          }
          if (type === "leave") {
            setDragActive(false);
            return;
          }
          if (type !== "drop") return;

          setDragActive(false);
          if (!acceptingRef.current) {
            const text = "drop ignored · finish the current install first";
            setStatusMsg(`⏳ ${text}`);
            // The overlay covers the footer whenever this can happen.
            setBusyNotice((prev) => ({ text, seq: (prev?.seq ?? 0) + 1 }));
            return;
          }

          const dropped = paths || [];
          const archive = dropped.find((p) =>
            SIDELOAD_EXTENSIONS.includes(p.split(".").pop()?.toLowerCase())
          );

          if (!archive) {
            setStatusMsg(
              dropped.length
                ? `✗ unsupported file · sideload accepts ${SIDELOAD_EXTENSIONS.join(", ")}`
                : "✗ nothing to sideload"
            );
            return;
          }
          // Starting a new install supersedes whatever the overlay still shows,
          // including the NXM input screen: dropping an archive onto it left the
          // sideload form stacked underneath, invisible (docs/bugs.md B9).
          setInstallProgress(null);
          setNxmInput(false);
          setSideloadPath(archive);
        });
        if (disposed) un();
        else unlisten = un;
      } catch (e) {
        console.error("Failed to setup drag&drop listener:", e);
      }
    })();

    return () => {
      disposed = true;
      if (unlisten) unlisten();
    };
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
      // Refresh selectedMod with updated data from new list
      setSelectedMod((cur) => {
        if (!cur) return null;
        const updated = modList.find(m => m.id === cur.id);
        return updated ? { ...updated, _siblings: cur._siblings } : cur;
      });
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

  // Every outcome now arrives from the backend — a progress event, or a rejected
  // promise. There is deliberately no timer watching for silence: the one that
  // used to live here fired *after* the install had already finished and
  // overwrote its result with a fabricated "no response" error (docs/bugs.md
  // B11). An unparseable link is an Err from `handle_nxm_url`, like any other
  // failure, so nothing here has to infer anything from a lack of events.
  const handleInstallUrl = async (url) => {
    setStatusMsg("jacking in · processing NXM URL...");
    try {
      await invoke("handle_nxm_url", { nxmUrl: url });
    } catch (error) {
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
  };

  const handleSideloadPick = async () => {
    if (!acceptingArchives) {
      const text = "already jacking in · finish or cancel the current install first";
      setStatusMsg(`⏳ ${text}`);
      setBusyNotice((prev) => ({ text, seq: (prev?.seq ?? 0) + 1 }));
      return;
    }
    try {
      const selected = await openDialog({
        multiple: false,
        title: "Select a mod archive",
        filters: [{ name: "Mod archives", extensions: SIDELOAD_EXTENSIONS }],
      });
      if (selected) {
        // Same hand-off as a drop: the picked archive takes over the screen.
        setInstallProgress(null);
        setNxmInput(false);
        setSideloadPath(selected);
      }
    } catch (error) {
      console.error("Failed to open file picker:", error);
      setStatusMsg(`✗ could not open file picker: ${error}`);
    }
  };

  const handleSideloadInstall = async (params) => {
    setSideloadPath(null);
    setStatusMsg(`sideloading · ${params.mod_name}...`);
    try {
      await invoke("install_mod_from_local", { params });
    } catch (error) {
      console.error("Sideload failed:", error);
      setStatusMsg(`✗ sideload failed: ${error}`);
      setInstallProgress({ stage: "error", message: String(error) });
      try {
        await invoke("add_log_entry", {
          message: `Sideload failed: ${error}`,
          level: "error",
          category: "installation",
        });
      } catch {}
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
      const report = await invoke("remove_mod", { modId });
      await loadMods();
      setMods((cur) => {
        const target = cur.find((m) => m.id === modId) ?? null;
        setSelectedMod(target);
        if (target && !target.removed) {
          // Files survived the removal (locked, no permission), so the record
          // stays live — don't send the user to the flatlined list for a mod
          // that isn't there.
          setStatusMsg(report);
        } else {
          setModFilter("removed");
          setStatusMsg(`flatlined: ${modName}`);
        }
        return cur;
      });
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

  // Re-run the self-check on demand (after saving settings or registering the
  // NXM handler) so the banner reflects the new state without a restart.
  const recheckHealth = () => {
    invoke("check_startup_health")
      .then((h) => setHealthIssues(!h.healthy && h.issues?.length > 0 ? h.issues : []))
      .catch(() => {});
  };

  const handleRegisterNxm = async () => {
    try {
      await invoke("register_nxm_handler");
    } catch (e) {
      console.error("register_nxm_handler failed:", e);
    }
    recheckHealth();
  };

  const hasNxmIssue = healthIssues.some((i) => i.code?.startsWith("nxm"));
  // Both silent-breakage issues are fixed the same way — a repair in Config.
  const hasBrokenInstalls = healthIssues.some(
    (i) => i.code === "wrapped_mods" || i.code === "mangled_paths"
  );

  return (
    <div className="app">
      {booting && <SplashScreen onDone={() => setBooting(false)} />}
      <header className="app-header">
        <div className="app-header-title">
          <h1 className="app-header-game">Cyberpunk 2077</h1>
          <p className="app-header-app">Crossover Mod Manager</p>
        </div>
        <nav className="nav">
          <button className={activeTab === "mods" ? "active" : ""} onClick={() => setActiveTab("mods")} {...hint("browse and manage installed mods")}>Chrome</button>
          <button
            onClick={() => setNxmInput(true)}
            disabled={installBusy}
            {...hint(installBusy ? "an install is already running" : "install a mod — from an nxm:// link or an archive on disk")}
          >
            Jack In
          </button>
          <button
            onClick={handleSyncMods}
            disabled={loading || !!syncProgress}
            {...hint("check for mod updates, fetch details and thumbnails from Nexus")}
          >
            {syncProgress ? "Netrunning…" : "Netrun"}
          </button>
          <button className={activeTab === "settings" ? "active" : ""} onClick={() => setActiveTab("settings")} {...hint("game paths, API key, and app settings")}>Config</button>
          <button className={activeTab === "manifest" ? "active" : ""} onClick={() => setActiveTab("manifest")} {...hint("version info, credits, and links")}>About</button>
        </nav>
      </header>

      {healthIssues.length > 0 && (
        <div className="health-banner">
          <div className="health-banner-body">
            <span className="health-banner-icon">⚠</span>
            <div className="health-banner-msgs">
              {healthIssues.map((issue, i) => (
                <div key={i} className={`health-banner-line health-${issue.type || "warning"}`}>
                  {issue.message}
                </div>
              ))}
            </div>
          </div>
          <div className="health-banner-actions">
            {hasNxmIssue && (
              <button onClick={handleRegisterNxm}>Register NXM handler</button>
            )}
            {hasBrokenInstalls && (
              <button onClick={() => setActiveTab("settings")}>Repair mods</button>
            )}
            <button onClick={() => setActiveTab("settings")}>Open Config</button>
            <button className="health-banner-dismiss" onClick={() => setHealthIssues([])}>Dismiss</button>
          </div>
        </div>
      )}

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
                dragActive={dragActive}
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
          <Manifest version={__APP_VERSION__} />
        ) : (
          <Settings hint={hint} onSaved={recheckHealth} onNavigateToMod={(modId) => {
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
        open={(nxmInput || !!installProgress) && !relayStatus}
        progress={installProgress}
        busy={installRunning}
        notice={busyNotice}
        onSubmit={handleInstallUrl}
        onSideload={handleSideloadPick}
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
          const prevSelectedId = selectedMod?.id;
          loadMods().then(() => {
            if (wasSuccess) {
              setModFilter((cur) => cur === "removed" ? "all" : cur);
              // loadMods already refreshes selectedMod by id — only select by name if nothing was selected
              if (!prevSelectedId && modName) {
                setMods((cur) => {
                  const installed = cur.find(m => m.name === modName && !m.removed);
                  if (installed) setSelectedMod(installed);
                  return cur;
                });
              }
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

      <SideloadOverlay
        open={!!sideloadPath && !installProgress}
        archivePath={sideloadPath}
        onSubmit={handleSideloadInstall}
        onCancel={() => setSideloadPath(null)}
      />

      {relayStatus && (
        <DevRelayOverlay
          stage={relayStatus.stage}
          message={relayStatus.message}
          nxmUrl={relayStatus.nxm_url ?? null}
          coldStart={relayStatus.cold_start ?? false}
          onDismiss={() => setRelayStatus(null)}
        />
      )}

      <AppFooter version={__APP_VERSION__} build={__BUILD_ID__} status={statusMsg} hoverHint={hoverHint} />

    </div>
  );
}

export default App;
