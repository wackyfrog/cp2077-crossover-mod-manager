import { useMemo } from "react";
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

function ModList({
  mods, selectedMod, onSelectMod, searchQuery = "", filter = "all", sort = "recent",
  loading, syncing, onSync,
  hint = () => ({}),
}) {
  const filtered = useMemo(() => {
    let result = mods;

    if (filter === "removed")        result = result.filter((m) => m.removed);
    else                             result = result.filter((m) => !m.removed);

    if (filter === "enabled")        result = result.filter((m) => m.enabled);
    else if (filter === "disabled")  result = result.filter((m) => !m.enabled && !m.removed);
    else if (filter === "updates")   result = result.filter((m) => m.update_available && !m.removed);

    const q = searchQuery.trim().toLowerCase();
    if (q) {
      result = result.filter(
        (m) =>
          m.name?.toLowerCase().includes(q) ||
          m.author?.toLowerCase().includes(q) ||
          m.version?.toLowerCase().includes(q)
      );
    }

    result = [...result].sort((a, b) => {
      if (sort === "name") {
        return (a.name ?? "").localeCompare(b.name ?? "", undefined, { sensitivity: "base" });
      }
      const ta = a.installed_at ? new Date(a.installed_at).getTime() : 0;
      const tb = b.installed_at ? new Date(b.installed_at).getTime() : 0;
      return tb - ta;
    });

    return result;
  }, [mods, searchQuery, filter, sort]);

  const groups = useMemo(() => groupMods(filtered), [filtered]);

  const renderMod = (mod) => (
    <div
      key={mod.id}
      className={`mod-item ${selectedMod?.id === mod.id ? "selected" : ""} ${mod.removed ? "mod-item-removed" : !mod.enabled ? "mod-item-disabled" : ""}`}
      onClick={() => onSelectMod(mod)}
    >
      <div className="mod-info">
        <h3>{mod.name}</h3>
        <p className="mod-version">
          {!mod.removed && !mod.enabled && (
            <span className="mod-badge mod-badge-ghosted">GHOSTED</span>
          )}
          {mod.removed && <span className="mod-badge mod-badge-flatlined">FLATLINED</span>}
          {!mod.removed && mod.update_available && <span className="mod-badge mod-badge-update" title={`Update available: v${mod.latest_version}`}>UPD</span>}
          v{mod.version}
        </p>
      </div>
    </div>
  );

  const renderPartMod = (mod, siblings) => (
    <div
      key={mod.id}
      className={`mod-item mod-item-part ${selectedMod?.id === mod.id ? "selected" : ""} ${!mod.enabled ? "mod-item-disabled" : ""}`}
      onClick={() => onSelectMod({ ...mod, _siblings: siblings })}
    >
      <div className="mod-info">
        <p className="mod-part-name">{mod.file_name || `File #${mod.file_id || "?"}`}</p>
        <p className="mod-part-meta">
          {!mod.enabled && <span className="mod-badge mod-badge-ghosted">GHOSTED</span>}
          {mod.update_available && <span className="mod-badge mod-badge-update" title={`Update available: v${mod.latest_version}`}>UPD</span>}
          {mod.files?.length || 0} files
        </p>
      </div>
    </div>
  );

  return (
    <div className="mod-list">
      <div className="mod-list-content">
        {mods.length === 0 ? (
          <div className="empty-state">
            <p className="empty-title">No chrome installed</p>
            <p className="empty-hint">Hit "Download with Mod Manager" on NexusMods to jack in</p>
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
          groups.map(([modId, items]) => {
            if (items.length === 1) return renderMod(items[0]);

            const sortedParts = [...items].sort((a, b) => (a.file_name ?? '').localeCompare(b.file_name ?? ''));
            const allEnabled  = items.every((m) => m.enabled);
            const anyEnabled  = items.some((m) => m.enabled);
            const anyUpdate   = items.some((m) => m.update_available);
            const label       = items[0].name;

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
                    version: items[0].version,
                    author: items[0].author,
                    summary: items[0].summary,
                    picture_url: items[0].picture_url,
                    nexus_updated_at: items[0].nexus_updated_at,
                    update_available: anyUpdate,
                    latest_version: items.find(m => m.update_available)?.latest_version,
                    enabled: allEnabled,
                    _siblings: sortedParts,
                  })}
                >
                  <div className="mod-info">
                    <h3>{label}</h3>
                    <p className="mod-version mod-group-meta">
                      {!allEnabled && (
                        <span className={`mod-badge ${anyEnabled ? "mod-badge-partial" : "mod-badge-ghosted"}`}>
                          {anyEnabled ? "PARTIAL" : "GHOSTED"}
                        </span>
                      )}
                      {anyUpdate && <span className="mod-badge mod-badge-update" title="Update available">UPD</span>}
                      v{items[0].version} · {items.length} parts
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
          })
        )}
      </div>

      <div className="mod-list-count">
        {filtered.length}/{mods.filter(m => !m.removed).length}
      </div>

      <div className="mod-list-footer">
        <button
          onClick={onSync}
          className={`footer-btn sync ${syncing ? "syncing" : ""}`}
          disabled={loading || syncing}
          {...hint("check for mod updates, fetch details and thumbnails from Nexus")}
        >
          {syncing ? "Netrunning…" : "Netrun"}
        </button>
      </div>
    </div>
  );
}

export default ModList;
