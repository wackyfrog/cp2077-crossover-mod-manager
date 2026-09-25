import { useEffect, useMemo, useRef, useState } from "react";
import "./ModList.css";

function groupMods(mods) {
  const groups = new Map();
  const singletons = [];

  for (const mod of mods) {
    if (mod.mod_id) {
      if (!groups.has(mod.mod_id)) groups.set(mod.mod_id, []);
      groups.get(mod.mod_id).push(mod);
    } else {
      singletons.push([null, [mod]]);
    }
  }

  return [
    ...[...groups.entries()].map(([id, items]) => [id, items]),
    ...singletons,
  ];
}

// OUTDATED: Nexus has a newer version. LATEST: updated this session, so the
// row that just lost OUTDATED says why instead of looking unchanged.
function StatusBadge({ outdated, latest, title }) {
  if (outdated) return <span className="mod-badge mod-badge-update" title={title}>OUTDATED</span>;
  if (latest) return <span className="mod-badge mod-badge-latest" title="Updated to the latest version this session">LATEST</span>;
  return null;
}

/** Hover text naming the titles a mod had on Nexus before, if any. */
export const formerly = (names) => {
  const unique = [...new Set(names ?? [])];
  return unique.length ? `formerly: ${unique.join(" · ")}` : undefined;
};

/**
 * The mods the list shows for a filter and search, unsorted. App uses it too:
 * a selection the list hides isn't kept on the details pane.
 */
export function filterMods(mods, { filter = "all", searchQuery = "", justUpdated = {} }) {
  let result = mods;

  if (filter === "removed")        result = result.filter((m) => m.removed);
  else                             result = result.filter((m) => !m.removed);

  if (filter === "enabled")        result = result.filter((m) => m.enabled);
  else if (filter === "disabled")  result = result.filter((m) => !m.enabled && !m.removed);
  // Mods updated this session stay, so the list doesn't shift under you
  else if (filter === "updates")   result = result.filter((m) => (m.update_available || justUpdated[m.id]) && !m.removed);

  const q = searchQuery.trim().toLowerCase();
  if (q) {
    result = result.filter(
      (m) =>
        m.name?.toLowerCase().includes(q) ||
        // A part of a multi-part mod is known by its file's name
        m.file_name?.toLowerCase().includes(q) ||
        // The name it was installed under, before the author renamed it
        m.former_names?.some((n) => n.toLowerCase().includes(q)) ||
        m.author?.toLowerCase().includes(q) ||
        m.uploader_name?.toLowerCase().includes(q) ||
        m.version?.toLowerCase().includes(q)
    );
  }
  return result;
}

/** Whether the list shows this selection: a mod, a part, or a group of parts. */
export function isShown(selected, shown) {
  if (!selected) return false;
  if (selected._isGroup) return shown.some((m) => m.mod_id === selected.mod_id);
  return shown.some((m) => m.id === selected.id);
}

function ModList({
  mods, justUpdated = {}, reqProblems = {}, selectedMod, onSelectMod, searchQuery = "", filter = "all", sort = "recent",
  loading, dragActive = false,
}) {
  const contentRef = useRef(null);
  const filtered = useMemo(() => {
    let result = filterMods(mods, { filter, searchQuery, justUpdated });

    result = [...result].sort((a, b) => {
      if (sort === "name") {
        return (a.name ?? "").localeCompare(b.name ?? "", undefined, { sensitivity: "base" });
      }
      // An update resets installed_at; this session it sorts by the old one
      const at = (m) => justUpdated[m.id]?.sortAt ?? m.installed_at;
      const ta = at(a) ? new Date(at(a)).getTime() : 0;
      const tb = at(b) ? new Date(at(b)).getTime() : 0;
      return tb - ta;
    });

    return result;
  }, [mods, justUpdated, searchQuery, filter, sort]);

  // Keep the selection on screen — after an update it's picked for you
  useEffect(() => {
    const el = contentRef.current?.querySelector(".mod-item.selected, .mod-group-header.selected");
    el?.scrollIntoView({ block: "nearest" });
  }, [selectedMod?.id, selectedMod?.mod_id, filter, justUpdated]);

  const groups = useMemo(() => groupMods(filtered), [filtered]);

  // Requirements from Nexus the game won't load (missing, ghosted, flatlined)
  const ReqBadge = ({ names }) =>
    names?.length ? <span className="mod-badge mod-badge-req" title={`Requires: ${names.join(", ")}`}>REQ</span> : null;

  const renderMod = (mod) => (
    <div
      key={mod.id}
      className={`mod-item ${selectedMod?.id === mod.id ? "selected" : ""} ${mod.removed ? "mod-item-removed" : !mod.enabled ? "mod-item-disabled" : ""}`}
      onClick={() => onSelectMod(mod)}
    >
      <div className="mod-info">
        <h3 title={formerly(mod.former_names)}>{mod.name}</h3>
        <p className="mod-version">
          {!mod.removed && !mod.enabled && (
            <span className="mod-badge mod-badge-ghosted">GHOSTED</span>
          )}
          {mod.removed && <span className="mod-badge mod-badge-flatlined">FLATLINED</span>}
          {!mod.removed && <StatusBadge outdated={mod.update_available} latest={justUpdated[mod.id]} title={`v${mod.latest_version} available`} />}
          {!mod.removed && <ReqBadge names={reqProblems[mod.id]} />}
          v{mod.version}
        </p>
      </div>
    </div>
  );

  const renderPartMod = (mod, siblings) => (
    <div
      key={mod.id}
      className={`mod-item mod-item-part ${selectedMod?.id === mod.id ? "selected" : ""} ${mod.removed ? "mod-item-removed" : !mod.enabled ? "mod-item-disabled" : ""}`}
      onClick={() => onSelectMod({ ...mod, _siblings: siblings })}
    >
      <div className="mod-info">
        <p className="mod-part-name">{mod.file_name || `File #${mod.file_id || "?"}`}</p>
        <p className="mod-part-meta">
          {/* A flatlined part is neither ghosted nor outdated — it's gone */}
          {mod.removed && <span className="mod-badge mod-badge-flatlined">FLATLINED</span>}
          {!mod.removed && !mod.enabled && <span className="mod-badge mod-badge-ghosted">GHOSTED</span>}
          {/* The mod's version is its core's; a part's own newer file may be numbered apart */}
          {!mod.removed && <StatusBadge outdated={mod.update_available} latest={justUpdated[mod.id]} title="newer file on NexusMods" />}
          {!mod.removed && <ReqBadge names={reqProblems[mod.id]} />}
          {mod.version && `v${mod.version} · `}{mod.files?.length || 0} files
        </p>
      </div>
    </div>
  );

  const renderGroup = ([modId, items]) => {
    if (items.length === 1) return renderMod(items[0]);

    const sortedParts = [...items].sort((a, b) => (a.file_name ?? '').localeCompare(b.file_name ?? ''));
    const allEnabled  = items.every((m) => m.enabled);
    const anyEnabled  = items.some((m) => m.enabled);
    const anyUpdate   = items.some((m) => m.update_available);
    const anyJust     = items.some((m) => justUpdated[m.id]);
    // The Flatlined filter lists only removed records, so a group there is all flatlined
    const allRemoved  = items.every((m) => m.removed);
    const label       = items[0].name;
    // Parts are separate files with versions of their own ("3.0.0" and
    // "3.0.0s"); a version for the whole mod only when they all agree,
    // never whichever part the current sort put first
    const sharedVersion = items.every((m) => m.version === items[0].version) ? items[0].version : null;

    const isGroupSelected = selectedMod?._isGroup && selectedMod?.mod_id === modId;
    const anyChildSelected = items.some((m) => m.id === selectedMod?.id);
    const isOpen = isGroupSelected || anyChildSelected;

    return (
      <div key={modId} className={`mod-group ${isOpen ? "has-selected" : ""}`}>
        <div
          className={`mod-group-header ${isGroupSelected ? "selected" : ""}`}
          onClick={() => onSelectMod({
            _isGroup: true,
            mod_id: modId,
            name: label,
            version: sharedVersion,
            author: items[0].author,
            summary: items[0].summary,
            picture_url: items[0].picture_url,
            nexus_updated_at: items[0].nexus_updated_at,
            update_available: anyUpdate && !allRemoved,
            removed: allRemoved,
            latest_version: items.find(m => m.update_available)?.latest_version,
            enabled: allEnabled,
            former_names: [...new Set(items.flatMap((m) => m.former_names ?? []))],
            _siblings: sortedParts,
          })}
        >
          <div className="mod-info">
            <h3 title={formerly(items.flatMap((m) => m.former_names ?? []))}>{label}</h3>
            <p className="mod-version mod-group-meta">
              {allRemoved && <span className="mod-badge mod-badge-flatlined">FLATLINED</span>}
              {!allRemoved && !allEnabled && (
                <span className={`mod-badge ${anyEnabled ? "mod-badge-partial" : "mod-badge-ghosted"}`}>
                  {anyEnabled ? "PARTIAL" : "GHOSTED"}
                </span>
              )}
              {!allRemoved && <StatusBadge outdated={anyUpdate} latest={anyJust} title="Update available" />}
              {!allRemoved && <ReqBadge names={[...new Set(items.flatMap((m) => reqProblems[m.id] ?? []))]} />}
              {sharedVersion && `v${sharedVersion} · `}{items.length} parts
            </p>
          </div>
        </div>
        {isOpen && (
          <div className="mod-group-parts">
            {sortedParts.map(m => renderPartMod(m, sortedParts))}
          </div>
        )}
      </div>
    );
  };

  // In UPDATES, what still needs updating stays on top; what was updated this
  // session folds away below, so it neither clutters the list nor vanishes
  // from under the cursor — the section opens by itself while it holds the
  // selection.
  const [doneOpen, setDoneOpen] = useState(false);
  const isDone = ([, items]) => filter === "updates" && !items.some((m) => m.update_available);
  const pending = groups.filter((g) => !isDone(g));
  const done = groups.filter(isDone);
  const holdsSelection = done.some(([modId, items]) =>
    (selectedMod?._isGroup && selectedMod.mod_id === modId) || items.some((m) => m.id === selectedMod?.id));
  const showDone = doneOpen || holdsSelection;

  return (
    <div className={`mod-list ${dragActive ? "drag-active" : ""}`}>
      <div className="mod-list-content" ref={contentRef}>
        {mods.length === 0 ? (
          <div className="empty-state">
            <p className="empty-title">{dragActive ? "Drop to sideload" : "No chrome installed"}</p>
            <p className="empty-hint">
              Hit "Download with Mod Manager" on NexusMods to jack in — or drop a
              .zip/.7z/.rar here to sideload a manual download
            </p>
          </div>
        ) : filtered.length === 0 ? (
          <div className="empty-state">
            <p className="empty-title">
              {searchQuery
               ? "Zero hits, choom"
               : filter === "enabled"   ? "Nothing slotted"
               : filter === "disabled" ? "Nothing ghosted"
               : filter === "removed"  ? "No flatlined chrome"
               : filter === "updates" ? "All chrome up to date"
               : "Nothing here"}
            </p>
            {searchQuery && (
              <p className="empty-hint">No match for "<span className="empty-query">{searchQuery}</span>" — try another search, choom</p>
            )}
          </div>
        ) : (
          <>
            {pending.map(renderGroup)}
            {filter === "updates" && pending.length === 0 && (
              <p className="mod-list-section-note">All chrome up to date</p>
            )}
            {done.length > 0 && (
              <>
                <button className="mod-list-section-toggle" onClick={() => setDoneOpen((v) => !v)}>
                  <span className="files-arrow">{showDone ? "▼" : "▶"}</span>
                  Updated this session ({done.length})
                </button>
                {showDone && done.map(renderGroup)}
              </>
            )}
          </>
        )}
      </div>

      <div className="mod-list-count">
        {filtered.length}/{mods.filter(m => !m.removed).length}
      </div>

      {/* Netrun and Sideload now live in the main nav and the Jack In screen. */}
    </div>
  );
}

export default ModList;
