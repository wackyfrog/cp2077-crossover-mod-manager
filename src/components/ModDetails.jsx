import { useState, useEffect, useRef } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open as openUrl } from "@tauri-apps/plugin-shell";
import './ModDetails.css'

function relativeDate(dateStr) {
  if (!dateStr) return null;
  try {
    // Parse "DD Mon YYYY" format
    const d = new Date(dateStr);
    if (isNaN(d)) return dateStr;
    const now = new Date();
    const diffMs = now - d;
    const days = Math.floor(diffMs / 86400000);
    if (days === 0) return "today";
    if (days === 1) return "yesterday";
    if (days < 30) return `${days} days ago`;
    const months = Math.floor(days / 30);
    if (months < 12) return `${months} month${months > 1 ? "s" : ""} ago`;
    const years = Math.floor(days / 365);
    return `${years}+ year${years > 1 ? "s" : ""} ago`;
  } catch { return dateStr; }
}

function formatDate(raw) {
  if (!raw) return null;
  try {
    const d = new Date(raw);
    return d.toLocaleString('en-GB', {
      day: '2-digit', month: 'short', year: 'numeric',
      hour: '2-digit', minute: '2-digit',
    });
  } catch {
    return raw;
  }
}

/** Decode HTML entities like &#52; &amp; &lt; etc. */
function decodeEntities(str) {
  const el = document.createElement('textarea');
  el.innerHTML = str;
  return el.value;
}

/** Strip BBCode, HTML tags, and decode entities from raw text */
function stripMarkup(raw) {
  return raw
    .replace(/\[img\].*?\[\/img\]/gi, '')
    .replace(/\[url=.*?\](.*?)\[\/url\]/gi, '$1')
    .replace(/\[\/?\w+\]/gi, '')
    .replace(/<br\s*\/?>/gi, ' ')
    .replace(/<[^>]+>/g, '')
    .replace(/\s+/g, ' ')
    .trim();
}

/** Parse Nexus BBCode file description: extract first image URL and clean text */
function parseFileDescription(raw) {
  if (!raw) return { image: null, text: null };
  const imgMatch = raw.match(/\[img\](.*?)\[\/img\]/i);
  const image = imgMatch ? imgMatch[1] : null;
  const text = decodeEntities(stripMarkup(raw)) || null;
  return { image: image || null, text };
}

/** Strip HTML/BBCode tags from summary text */
function cleanSummary(raw) {
  if (!raw) return null;
  return decodeEntities(stripMarkup(raw)) || null;
}

function Thumbnail({ src, alt }) {
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState(false);

  useEffect(() => {
    setLoaded(false);
    setError(false);
  }, [src]);

  if (!src || error) return null;

  return (
    <div className={`mod-thumbnail ${loaded ? '' : 'mod-thumbnail-loading'}`}>
      {!loaded && <div className="mod-thumbnail-placeholder" />}
      <img
        src={src}
        alt={alt}
        onLoad={() => setLoaded(true)}
        onError={() => setError(true)}
        style={{ display: loaded ? 'block' : 'none' }}
      />
    </div>
  );
}

const normVersion = (v) => String(v ?? "").trim().replace(/^v/i, "");

// Numeric collation orders "1.10" after "1.9"; the API's object keys arrive
// sorted as plain strings, so their order can't be trusted.
const compareVersions = (a, b) =>
  normVersion(a).localeCompare(normVersion(b), undefined, { numeric: true });

// A version counts as new when it's past the installed one but not past the
// latest release NETRUN saw — authors sometimes write notes before uploading,
// and those shouldn't claim an update the UPD badge doesn't show.
const isNewVersion = (ver, installed, latest) =>
  compareVersions(ver, installed) > 0 && (!latest || compareVersions(ver, latest) <= 0);

// Newest first, split around the installed version: what an update brings
// stays open, the history below it folds away.
function splitChangelog(changelog, installed, latest) {
  const entries = Object.entries(changelog)
    .map(([ver, entry]) => ({
      ver,
      lines: entry?.lines ?? (Array.isArray(entry) ? entry : []),
      date: entry?.date,
    }))
    .sort((a, b) => compareVersions(b.ver, a.ver));
  const ahead = entries.filter((e) => compareVersions(e.ver, installed) > 0);
  const newer = ahead.filter((e) => isNewVersion(e.ver, installed, latest));
  const rest = entries.slice(ahead.length);
  const current = rest[0] && compareVersions(rest[0].ver, installed) === 0 ? rest[0] : null;
  // Nothing newer and the installed version isn't listed: keep the latest open.
  const head = current ? [current] : newer.length ? [] : rest.slice(0, 1);
  return { ahead, newer, head, older: rest.slice(head.length) };
}

function ChangelogVersion({ entry, kind }) {
  return (
    <div className={`changelog-version changelog-version--${kind}`}>
      <div className="changelog-ver-label">
        v{normVersion(entry.ver)}
        {kind === "installed" && <span className="changelog-installed-badge">installed</span>}
        {entry.date && <span className="changelog-date">{entry.date}</span>}
      </div>
      {entry.lines.length > 0 && (
        <div className="changelog-entries">
          {entry.lines.map((line, i) => (
            <div key={i} className="changelog-entry" dangerouslySetInnerHTML={{ __html: line }} />
          ))}
        </div>
      )}
    </div>
  );
}

function ChangelogPanel({ changelog, installed, latest }) {
  const [olderOpen, setOlderOpen] = useState(false);
  const { ahead, newer, head, older } = splitChangelog(changelog, installed, latest);
  const isInstalled = (e) => compareVersions(e.ver, installed) === 0;
  return (
    <div className="changelog-panel">
      {ahead.map((e) => (
        <ChangelogVersion key={e.ver} entry={e} kind={newer.includes(e) ? "new" : "old"} />
      ))}
      {head.map((e) => (
        <ChangelogVersion key={e.ver} entry={e} kind={isInstalled(e) ? "installed" : "old"} />
      ))}
      {older.length > 0 && (olderOpen
        ? older.map((e) => <ChangelogVersion key={e.ver} entry={e} kind="old" />)
        : (
          <button className="changelog-older-toggle" onClick={() => setOlderOpen(true)}>
            <span className="files-arrow">▶</span>
            {older.length} older {older.length === 1 ? "version" : "versions"}
          </button>
        ))}
    </div>
  );
}

// Toggle row + panel, placed right under the Version row in every view.
// The count shows before expanding, like the Files row's.
function ChangelogRow({ mod, state, onToggle, hint }) {
  const { status, data, open } = state;
  const count = data ? Object.keys(data).length : 0;
  const newer = data
    ? Object.keys(data).filter((v) => isNewVersion(v, mod.version, mod.latest_version)).length
    : 0;
  const summary =
    status === "loading" ? "fetching from NexusMods…"
    : status === "error" ? "couldn't fetch — click to retry"
    : status === "ready" && count === 0 ? "none on NexusMods"
    : status === "ready" && newer > 0 ? `${newer} newer`
    : status === "ready" ? `${count} ${count === 1 ? "version" : "versions"}`
    : null;
  const expanded = open && status === "ready" && count > 0;
  return (
    <>
      <div
        className="detail-row files-toggle-row"
        onClick={onToggle}
        {...hint(
          expanded ? "collapse changelog"
          : status === "ready" ? "show what changed in each version"
          : "fetch this mod's changelog and latest version from NexusMods"
        )}
      >
        <span className="label">Changelog</span>
        <span className="value files-toggle-value">
          {summary}
          <span className="files-arrow">{expanded ? "▼" : "▶"}</span>
        </span>
      </div>
      {expanded && <ChangelogPanel changelog={data} installed={mod.version} latest={mod.latest_version} />}
    </>
  );
}

function ModDetails({ mod, siblings = [], onSelectMod, onRemove, onForget, onToggle, onJackIn, onModsChanged, loading, hint = () => ({}) }) {
  const [filesOpen, setFilesOpen] = useState(false);
  // Changelogs come from NETRUN's cache, so they match the UPD badge.
  // status: missing (never fetched) | ready | loading | error
  const [changelog, setChangelog] = useState({ status: "missing", data: null, open: false });
  const changelogFor = useRef(null);

  // Cache lookup is local — no NexusMods request while browsing the list
  useEffect(() => {
    const modId = mod?.mod_id ?? null;
    changelogFor.current = modId;
    setChangelog({ status: "missing", data: null, open: false });
    if (!modId) return;
    invoke("get_mod_changelog", { modId })
      .then((cached) => {
        if (changelogFor.current !== modId || !cached) return;
        setChangelog({ status: "ready", data: cached.versions, open: false });
      })
      .catch((err) => console.error("get_mod_changelog:", err));
  }, [mod?.id, mod?.mod_id]);

  const toggleChangelog = () => {
    if (!mod?.mod_id) return;
    if (changelog.status === "loading") return;
    if (changelog.status === "ready") {
      setChangelog((c) => ({ ...c, open: !c.open }));
      return;
    }
    // Not fetched yet: refresh this one mod the way NETRUN would, so its
    // version and UPD badge update together with the changelog.
    const modId = mod.mod_id;
    setChangelog({ status: "loading", data: null, open: true });
    invoke("refresh_mod", { modId })
      .then((cached) => {
        if (changelogFor.current !== modId) return;
        setChangelog({ status: "ready", data: cached?.versions ?? {}, open: true });
        onModsChanged?.();
      })
      .catch((err) => {
        if (changelogFor.current !== modId) return;
        console.error("refresh_mod:", err);
        setChangelog({ status: "error", data: null, open: false, error: String(err) });
      });
  };

  const changelogRow = mod?.mod_id ? (
    <ChangelogRow mod={mod} state={changelog} onToggle={toggleChangelog} hint={hint} />
  ) : null;

  if (!mod) {
    return (
      <div className="mod-details">
        <div className="empty-state">
          <p>Pick some chrome to inspect</p>
        </div>
      </div>
    )
  }

  const fileParsed = parseFileDescription(mod.file_description);
  const displayImage = fileParsed.image || mod.picture_url;
  const displaySummary = fileParsed.text || cleanSummary(mod.summary);

  const handleToggle = async () => {
    try {
      const nowEnabled = await invoke("toggle_mod", { modId: mod.id });
      onToggle?.(mod.id, nowEnabled);
    } catch (error) {
      console.error("Failed to toggle mod:", error);
      alert("Failed to toggle mod: " + error);
    }
  };

  const fileCount = mod.files?.length ?? 0;

  const parts = [...siblings].sort((a, b) => (a.file_name ?? '').localeCompare(b.file_name ?? ''));

  if (mod._isGroup) {
    return (
      <div className="mod-details">
        <div className="mod-details-header">
          <h2>{mod.update_available && <span className="details-upd-badge">UPD</span>}{mod.name}</h2>
          <span className="group-badge">{parts.length} parts</span>
        </div>

        <div className="mod-details-content">
          <Thumbnail src={mod.picture_url} alt={mod.name} />

          {cleanSummary(mod.summary) && (
            <p className="mod-summary">{cleanSummary(mod.summary)}</p>
          )}

          <div className="submod-list submod-list-standalone">
            {parts.map((s) => {
              const p = parseFileDescription(s.file_description);
              const showDesc = p.text && p.text !== cleanSummary(mod.summary);
              return (
                <span
                  key={s.id}
                  className="submod-item"
                  onClick={() => onSelectMod?.({ ...s, _siblings: parts })}
                  title={p.text || s.file_name || s.name}
                >
                  <span className={`submod-status ${s.enabled ? "enabled" : "disabled"}`}>
                    {s.enabled ? "◆" : "◇"}
                  </span>
                  <span className="submod-item-content">
                    <span className="submod-item-name">{s.file_name || `File #${s.file_id || "?"}`}</span>
                    {showDesc && <span className="submod-item-desc">{p.text}</span>}
                  </span>
                </span>
              );
            })}
          </div>

          <div className="detail-section">
            <div className="detail-row">
              <span className="label">Version</span>
              <span className="value">
                {mod.version}
                {mod.update_available && (
                  <>
                    <span className="version-arrow"> → </span>
                    <span className="version-update-badge">v{mod.latest_version}</span>
                  </>
                )}
                {mod.nexus_updated_at && (
                  <span className="version-date"> · {relativeDate(mod.nexus_updated_at)}</span>
                )}
              </span>
            </div>
            {changelogRow}
            <div className="detail-row">
              <span className="label">Author</span>
              <span className="value">{mod.author || 'Unknown'}</span>
            </div>
            <div className="detail-row">
              <span className="label">Mod ID</span>
              <span className="value">{mod.mod_id || 'N/A'}</span>
            </div>
            {mod.mod_id && (
              <div className="detail-row">
                <span className="label">Mod Page</span>
                <a
                  className="value nexus-link"
                  href="#"
                  onClick={(e) => {
                    e.preventDefault();
                    openUrl(`https://www.nexusmods.com/cyberpunk2077/mods/${mod.mod_id}`);
                  }}
                >
                  Open on NexusMods <span className="nexus-link-icon" aria-hidden="true">↗</span>
                </a>
              </div>
            )}
          </div>
        </div>

        <div className="mod-details-footer">
          {mod.mod_id && mod.update_available && (
            <button
              className="jackin-detail-button"
              onClick={() => onJackIn?.(mod)}
              disabled={loading}
            >
              Update
            </button>
          )}
        </div>
      </div>
    );
  }

  if (mod.removed) {
    return (
      <div className="mod-details mod-details-removed">
        <div className="mod-details-header">
          <h2 className="removed-title">{mod.name}</h2>
          <span className="removed-badge">FLATLINED</span>
        </div>

        <div className="mod-details-content">
          {cleanSummary(mod.summary) && (
            <p className="mod-summary mod-summary-dim">{cleanSummary(mod.summary)}</p>
          )}

          <div className="detail-section">
            <div className="detail-row">
              <span className="label">Version</span>
              <span className="value">{mod.version}</span>
            </div>
            {changelogRow}
            <div className="detail-row">
              <span className="label">Author</span>
              <span className="value">{mod.author || 'Unknown'}</span>
            </div>
            {mod.installed_at && (
              <div className="detail-row">
                <span className="label">Jacked in</span>
                <span className="value">{formatDate(mod.installed_at)}</span>
              </div>
            )}
            {mod.removed_at && (
              <div className="detail-row">
                <span className="label">Flatlined</span>
                <span className="value">{formatDate(mod.removed_at)}</span>
              </div>
            )}
            {mod.mod_id && (
              <div className="detail-row">
                <span className="label">Mod Page</span>
                <a
                  className="value nexus-link"
                  href="#"
                  onClick={(e) => {
                    e.preventDefault();
                    openUrl(`https://www.nexusmods.com/cyberpunk2077/mods/${mod.mod_id}`);
                  }}
                >
                  Open on NexusMods <span className="nexus-link-icon" aria-hidden="true">↗</span>
                </a>
              </div>
            )}
          </div>

          <p className="removed-hint">
            Game files have been deleted. This record is kept for reference only.
          </p>
        </div>

        <div className="mod-details-footer">
          <button
            className="forget-button"
            onClick={() => onForget(mod.id, mod.name)}
            disabled={loading}
            {...hint("permanently delete this record from database")}
          >
            Forget
          </button>
          {mod.mod_id && (
            <button
              className="jackin-detail-button"
              onClick={() => onJackIn?.(mod)}
              disabled={loading}
              {...hint("reinstall this mod from NexusMods")}
            >
              Jack In
            </button>
          )}
        </div>
      </div>
    );
  }

  return (
    <div className="mod-details">
      <div className="mod-details-header">
        <h2>{mod.update_available && <span className="details-upd-badge">UPD</span>}{mod.name}</h2>
        <label className={`cyber-toggle ${loading ? 'cyber-toggle--disabled' : ''}`} {...hint(mod.enabled ? 'disable this mod without removing files' : 'enable this mod')}>
          <input
            type="checkbox"
            checked={mod.enabled}
            onChange={handleToggle}
            disabled={loading}
          />
          <span className={`cyber-toggle-label ${mod.enabled ? 'enabled' : 'disabled'}`}>
            {mod.enabled ? 'Slotted' : 'Ghosted'}
          </span>
          <span className="cyber-toggle-track">
            <span className="cyber-toggle-knob" />
          </span>
        </label>
      </div>

      <div className="mod-details-content">
        <Thumbnail src={displayImage} alt={mod.name} />

        {displaySummary && (
          <p className="mod-summary">{displaySummary}</p>
        )}

        <div className="detail-section">
          <div className="detail-row">
            <span className="label">Version</span>
            <span className="value">
              {mod.version}
              {mod.update_available && (
                <>
                  <span className="version-arrow"> → </span>
                  <span className="version-update-badge">v{mod.latest_version}</span>
                </>
              )}
              {mod.nexus_updated_at && (
                <span className="version-date"> · {relativeDate(mod.nexus_updated_at)}</span>
              )}
            </span>
          </div>
          {changelogRow}
          <div className="detail-row">
            <span className="label">Author</span>
            <span className="value">{mod.author || 'Unknown'}</span>
          </div>
          <div className="detail-row">
            <span className="label">Mod ID</span>
            <span className="value">{mod.mod_id || 'N/A'}</span>
          </div>
          <div className="detail-row">
            <span className="label">File ID</span>
            <span className="value">
              {mod.file_id || 'N/A'}
              {mod.file_version && mod.file_version !== mod.version && (
                <span className="file-version-badge"> (file v{mod.file_version})</span>
              )}
            </span>
          </div>
          {mod.installed_at && (
            <div className="detail-row">
              <span className="label">Jacked in</span>
              <span className="value">{formatDate(mod.installed_at)}</span>
            </div>
          )}
          {mod.mod_id && (
            <div className="detail-row">
              <span className="label">Mod Page</span>
              <a
                className="value nexus-link"
                href="#"
                onClick={(e) => {
                  e.preventDefault();
                  openUrl(`https://www.nexusmods.com/cyberpunk2077/mods/${mod.mod_id}`);
                }}
                {...hint("open mod page on NexusMods in browser")}
              >
                Open on NexusMods <span className="nexus-link-icon" aria-hidden="true">↗</span>
              </a>
            </div>
          )}

          {/* Files row — inline toggle inside the info section */}
          <div
            className="detail-row files-toggle-row"
            onClick={() => setFilesOpen(v => !v)}
            {...hint(filesOpen ? "collapse file list" : "show installed files")}
          >
            <span className="label">Files</span>
            <span className="value files-toggle-value">
              {fileCount} {fileCount === 1 ? 'file' : 'files'}
              <span className="files-arrow">{filesOpen ? '▼' : '▶'}</span>
            </span>
          </div>
        </div>

        {filesOpen && (
          <div className="file-list">
            {fileCount > 0 ? (
              mod.files.map((file, index) => (
                <div key={index} className="file-item" title={file}>
                  <span className="file-path">{file.replace(/^.*?Cyberpunk 2077\//, '')}</span>
                  <button
                    className="reveal-button"
                    title="Show in Finder"
                    onClick={(e) => {
                      e.stopPropagation();
                      invoke("reveal_in_finder", { path: file }).catch((err) =>
                        console.error("reveal_in_finder:", err)
                      );
                    }}
                  >
                    ⌘
                  </button>
                </div>
              ))
            ) : (
              <p className="no-files">No file information available</p>
            )}
          </div>
        )}
      </div>

      <div className="mod-details-footer">
        <button
          className="remove-button"
          onClick={() => onRemove(mod.id)}
          disabled={loading}
          {...hint("remove mod files from game directory")}
        >
          Flatline
        </button>
        {mod.mod_id && (
          <button
            className="jackin-detail-button"
            onClick={() => onJackIn?.(mod)}
            disabled={loading}
            {...hint(mod.update_available ? "download and install the latest version from NexusMods" : "re-download and reinstall this mod from NexusMods")}
          >
            {mod.update_available ? "Update" : "Reinstall"}
          </button>
        )}
      </div>
    </div>
  )
}

export default ModDetails
