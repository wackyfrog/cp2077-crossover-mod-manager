import { useState, useEffect } from "react";
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

function ModDetails({ mod, siblings = [], onSelectMod, onRemove, onForget, onToggle, onJackIn, loading }) {
  const [filesOpen, setFilesOpen] = useState(false);
  const [changelog, setChangelog] = useState(null);
  const [changelogOpen, setChangelogOpen] = useState(false);
  const hasChangelog = changelog && Object.keys(changelog).length > 0;

  // Reset changelog when mod changes
  useEffect(() => {
    setChangelog(null);
    setChangelogOpen(false);
  }, [mod?.id, mod?.mod_id]);

  // Lazy load changelog on demand
  const openChangelog = () => {
    if (!mod?.mod_id) return;
    if (changelog) {
      setChangelogOpen(true);
      return;
    }
    invoke("get_mod_changelog", { modId: mod.mod_id })
      .then((data) => {
        setChangelog(data);
        if (data && Object.keys(data).length > 0) setChangelogOpen(true);
      })
      .catch(() => {});
  };

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
          <h2>{mod.name}</h2>
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
                <span
                  className={mod.mod_id ? "version-clickable" : ""}
                  onClick={() => mod.mod_id && openChangelog()}
                  title={mod.mod_id ? "Click to view changelog" : ""}
                >
                  {mod.version}
                </span>
                {mod.update_available && (
                  <>
                    <span className="version-arrow"> → </span>
                    <span
                      className={`version-update-badge ${mod.mod_id ? "version-clickable" : ""}`}
                      onClick={() => mod.mod_id && openChangelog()}
                      title={mod.mod_id ? "View changelog" : `v${mod.latest_version} available`}
                    >
                      v{mod.latest_version}
                    </span>
                  </>
                )}
                {mod.nexus_updated_at && (
                  <span className="version-date"> · {relativeDate(mod.nexus_updated_at)}</span>
                )}
              </span>
            </div>
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
                  nexusmods.com
                </a>
              </div>
            )}
          </div>

          {/* Changelog modal */}
          {changelogOpen && hasChangelog && (
            <div className="changelog-backdrop" onClick={() => setChangelogOpen(false)}>
              <div className="changelog-modal" onClick={(e) => e.stopPropagation()}>
                <div className="changelog-header">
                  <span className="changelog-title">Changelog — {mod.name}</span>
                  <button className="changelog-close" onClick={() => setChangelogOpen(false)}>✕</button>
                </div>
                <div className="changelog-body">
                  {Object.entries(changelog).reverse().map(([ver, entry]) => {
                    const isCurrent = ver === mod.version;
                    const lines = entry?.lines ?? (Array.isArray(entry) ? entry : []);
                    const date = entry?.date;
                    return (
                    <div key={ver} className={`changelog-version ${isCurrent ? "changelog-current" : ""}`}>
                      <div className="changelog-ver-label">
                        v{ver}
                        {date && <span className="changelog-date">{date}</span>}
                        {isCurrent && <span className="changelog-installed-badge">installed</span>}
                      </div>
                      <div className="changelog-entries">
                        {(Array.isArray(lines) ? lines : []).map((line, i) => (
                          <div key={i} className="changelog-entry" dangerouslySetInnerHTML={{ __html: line }} />
                        ))}
                      </div>
                    </div>
                    );
                  })}
                </div>
              </div>
            </div>
          )}
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
                  nexusmods.com
                </a>
              </div>
            )}
          </div>

          <p className="removed-hint">
            Game files have been deleted. This record is kept for reference only.
          </p>
        </div>

        <div className="mod-details-footer">
          {mod.mod_id && (
            <button
              className="jackin-detail-button"
              onClick={() => onJackIn?.(mod)}
              disabled={loading}
            >
              Jack In
            </button>
          )}
          <button
            className="forget-button"
            onClick={() => onForget(mod.id, mod.name)}
            disabled={loading}
          >
            Forget
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="mod-details">
      <div className="mod-details-header">
        <h2>{mod.name}</h2>
        <label className={`cyber-toggle ${loading ? 'cyber-toggle--disabled' : ''}`} title={mod.enabled ? 'Ghost this mod' : 'Slot in this mod'}>
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
              <span
                className={mod.mod_id ? "version-clickable" : ""}
                onClick={() => mod.mod_id && openChangelog()}
                title={mod.mod_id ? "Click to view changelog" : ""}
              >
                {mod.version}
              </span>
              {mod.update_available && (
                <>
                  <span className="version-arrow"> → </span>
                  <span
                    className={`version-update-badge ${mod.mod_id ? "version-clickable" : ""}`}
                    onClick={() => mod.mod_id && openChangelog()}
                    title={mod.mod_id ? "View changelog" : `v${mod.latest_version} available`}
                  >
                    v{mod.latest_version}
                  </span>
                </>
              )}
              {mod.nexus_updated_at && (
                <span className="version-date"> · {relativeDate(mod.nexus_updated_at)}</span>
              )}
            </span>
          </div>
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
              >
                nexusmods.com
              </a>
            </div>
          )}

          {/* Files row — inline toggle inside the info section */}
          <div
            className="detail-row files-toggle-row"
            onClick={() => setFilesOpen(v => !v)}
            title={filesOpen ? 'Collapse file list' : 'Expand file list'}
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

        {/* Changelog modal */}
        {changelogOpen && hasChangelog && (
          <div className="changelog-backdrop" onClick={() => setChangelogOpen(false)}>
            <div className="changelog-modal" onClick={(e) => e.stopPropagation()}>
              <div className="changelog-header">
                <span className="changelog-title">Changelog — {mod.name}</span>
                <button className="changelog-close" onClick={() => setChangelogOpen(false)}>✕</button>
              </div>
              <div className="changelog-body">
                {Object.entries(changelog).reverse().map(([ver, lines]) => {
                  const isCurrent = ver === mod.version;
                  return (
                  <div key={ver} className={`changelog-version ${isCurrent ? "changelog-current" : ""}`}>
                    <div className="changelog-ver-label">v{ver}{isCurrent && <span className="changelog-installed-badge">installed</span>}</div>
                    <div className="changelog-entries">
                      {(Array.isArray(lines) ? lines : []).map((line, i) => (
                        <div key={i} className="changelog-entry" dangerouslySetInnerHTML={{ __html: line }} />
                      ))}
                    </div>
                  </div>
                  );
                })}
              </div>
            </div>
          </div>
        )}
      </div>

      <div className="mod-details-footer">
        {mod.mod_id && (
          <button
            className="jackin-detail-button"
            onClick={() => onJackIn?.(mod)}
            disabled={loading}
          >
            {mod.update_available ? "Update" : "Reinstall"}
          </button>
        )}
        <button
          className="remove-button"
          onClick={() => onRemove(mod.id)}
          disabled={loading}
        >
          Flatline
        </button>
      </div>
    </div>
  )
}

export default ModDetails
