import { useState, useEffect, useRef } from "react";
import { formerly } from "./ModList";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
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

/** A file description as plain text with its line breaks kept: BBCode and
 *  HTML dropped, images removed, entities decoded. Rendered as text, never
 *  as HTML — it's whatever markup the author typed. */
function descriptionText(raw) {
  if (!raw) return null;
  const text = decodeEntities(
    raw
      .replace(/\[img\].*?\[\/img\]/gi, '')
      .replace(/\[url=.*?\](.*?)\[\/url\]/gi, '$1')
      .replace(/\[\/?\w+(=[^\]]*)?\]/gi, '')
      // authors often end a line both ways ("…\n<br />"): one break, not two
      .replace(/\r?\n?<br\s*\/?>/gi, '\n')
      .replace(/<[^>]+>/g, '')
  );
  return text.replace(/[ \t]+\n/g, '\n').replace(/\n{3,}/g, '\n\n').trim() || null;
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

// Load state belongs to a src, not to the component: resetting it in an
// effect when src changed could run after a cached image had already fired
// onLoad, and left the placeholder up for good (seen after an update swapped
// the file-description image for the mod's picture; reselecting the mod
// brought it back).
function Thumbnail({ src, alt }) {
  const [loadedSrc, setLoadedSrc] = useState(null);
  const [failedSrc, setFailedSrc] = useState(null);

  if (!src || failedSrc === src) return null;
  const loaded = loadedSrc === src;

  return (
    <div className={`mod-thumbnail ${loaded ? '' : 'mod-thumbnail-loading'}`}>
      {!loaded && <div className="mod-thumbnail-placeholder" />}
      <img
        src={src}
        alt={alt}
        onLoad={() => setLoadedSrc(src)}
        onError={() => setFailedSrc(src)}
        style={{ display: loaded ? 'block' : 'none' }}
      />
    </div>
  );
}

// Shows the NexusMods account that uploaded the mod, linked to its mods list.
// The mod's own "author" field is free text that needn't match any account
// (Native Interactions Framework credits "keanuWheeze", uploaded by
// "NexusGuy999"), so it only appears in the hint when it differs. Before
// NETRUN has seen the mod there's no account yet, only that text.
function AuthorValue({ mod, hint }) {
  if (!mod.uploader_id) return <span className="value">{mod.author || "Unknown"}</span>;
  const account = mod.uploader_name || mod.author || "Unknown";
  const credited = mod.author && mod.author !== account ? ` · credited as ${mod.author}` : "";
  return (
    <a
      className="value nexus-link"
      href="#"
      onClick={(e) => {
        e.preventDefault();
        // The uploader's mods list; the bare member-id URL is only a fallback
        openUrl(mod.uploader_name
          ? `https://www.nexusmods.com/profile/${encodeURIComponent(mod.uploader_name)}/mods`
          : `https://www.nexusmods.com/users/${mod.uploader_id}`);
      }}
      {...hint(`open ${account}'s mods on NexusMods in browser${credited}`)}
    >
      {account}
      <span className="nexus-link-icon" aria-hidden="true">↗</span>
    </a>
  );
}

// When a row unfolds below the visible part of the details pane, scroll just
// enough to show what it opened — but never so far that the row itself goes
// off the top. Nothing moves when it already fits.
function useRevealOnOpen(open, rowRef, contentRef) {
  useEffect(() => {
    if (!open) return;
    const frame = requestAnimationFrame(() => {
      const row = rowRef.current;
      const content = contentRef.current;
      const pane = row?.closest(".mod-details-content");
      if (!row || !content || !pane) return;
      const view = pane.getBoundingClientRect();
      const hidden = content.getBoundingClientRect().bottom - view.bottom;
      if (hidden <= 0) return;
      const headroom = row.getBoundingClientRect().top - view.top - 8;
      const reduce = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
      pane.scrollBy({ top: Math.min(hidden, headroom), behavior: reduce ? "auto" : "smooth" });
    });
    return () => cancelAnimationFrame(frame);
  }, [open]);
}

const normVersion = (v) => String(v ?? "").trim().replace(/^v/i, "");

// Numeric collation ("1.10" after "1.9"); only breaks ties between uploads
// that share a timestamp.
const compareVersions = (a, b) =>
  normVersion(a).localeCompare(normVersion(b), undefined, { numeric: true });

// Seconds since epoch: the exact upload time when the cache has it, else the
// day it shows (caches written before `uploaded` existed).
const uploadedAt = (entry) =>
  entry?.uploaded ?? (entry?.date ? Date.parse(entry.date) / 1000 : null);

// Without the installed version to split at, this many stay open.
const CHANGELOG_OPEN_WITHOUT_INSTALLED = 3;

// Newest upload first. Version numbers on Nexus don't order reliably ("0.65"
// comes after "0.7", "2.3.2b" may be a variant rather than a successor), so
// nothing here says how far behind the install is: the installed version is
// marked where it falls, and what sits above it was simply uploaded later.
// It's found by the installed file's version, then the mod's.
//
// A mod's files form lines (`lines`: file id → line id, from NETRUN) — a core
// and its packs, each with versions of its own. A part shows its own line;
// the other files' versions fold away under one row. Without lines (an older
// cache, or a group) every version is the mod's own, as before.
function readChangelog(changelog, lines, mod) {
  const entries = Object.entries(changelog)
    .map(([ver, entry]) => ({
      ver,
      lines: entry?.lines ?? (Array.isArray(entry) ? entry : []),
      date: entry?.date,
      at: uploadedAt(entry),
      description: descriptionText(entry?.description),
      files: entry?.files ?? [],
    }))
    .sort((a, b) => (b.at ?? 0) - (a.at ?? 0) || compareVersions(b.ver, a.ver));
  const ownLine = !mod._isGroup && lines && mod.file_id != null ? lines[String(mod.file_id)] : undefined;
  const byLine = ownLine !== undefined;
  const inLine = (e) => !byLine || e.files.length === 0 || e.files.some((f) => lines[String(f.id)] === ownLine);
  const own = entries.filter(inLine);
  const others = entries.filter((e) => !inLine(e));
  const find = (v) => (v ? own.find((e) => normVersion(e.ver) === normVersion(v)) : null);
  const installed = find(mod.file_version) || find(mod.version) || null;
  const split = installed ? own.indexOf(installed) + 1 : CHANGELOG_OPEN_WITHOUT_INSTALLED;
  return {
    installed,
    shown: own.slice(0, split),
    older: own.slice(split),
    others,
    // The newest version of this part's own file, when its line is known
    lineLatest: byLine ? own[0]?.ver ?? null : null,
  };
}

// "LUT Pack - Misc" — which file(s) a version came from
const fileLabel = (entry) => [...new Set(entry.files.map((f) => f.name).filter(Boolean))].join(" · ");

function ChangelogVersion({ entry, kind, showFile = false }) {
  return (
    <div className={`changelog-version changelog-version--${kind}`}>
      <div className="changelog-ver-label">
        v{normVersion(entry.ver)}
        {kind === "installed" && <span className="changelog-installed-badge">installed</span>}
        {showFile && fileLabel(entry) && <span className="changelog-file-name">{fileLabel(entry)}</span>}
        {entry.date && <span className="changelog-date">{entry.date}</span>}
      </div>
      {entry.lines.length > 0 && (
        <div className="changelog-entries">
          {entry.lines.map((line, i) => (
            <div key={i} className="changelog-entry" dangerouslySetInnerHTML={{ __html: line }} />
          ))}
        </div>
      )}
      {/* No notes for this version: the author's file description stands in,
          set apart because it's as often install steps as it is changes */}
      {entry.lines.length === 0 && entry.description && (
        <div className="changelog-file-description">
          <span className="changelog-file-description-label">file description</span>
          <div className="changelog-file-description-text">{entry.description}</div>
        </div>
      )}
    </div>
  );
}

function ChangelogPanel({ changelog, lines, mod, ref }) {
  const [olderOpen, setOlderOpen] = useState(false);
  const [othersOpen, setOthersOpen] = useState(false);
  const { installed, shown, older, others } = readChangelog(changelog, lines, mod);
  return (
    <div className="changelog-panel" ref={ref}>
      {shown.map((e) => (
        <ChangelogVersion key={e.ver} entry={e} kind={e === installed ? "installed" : "other"} />
      ))}
      {older.length > 0 && (olderOpen
        ? older.map((e) => <ChangelogVersion key={e.ver} entry={e} kind="other" />)
        : (
          <button className="changelog-older-toggle" onClick={() => setOlderOpen(true)}>
            <span className="files-arrow">▶</span>
            {older.length} earlier {older.length === 1 ? "version" : "versions"}
          </button>
        ))}
      {others.length > 0 && (
        <>
          <button className="changelog-older-toggle changelog-others-toggle" onClick={() => setOthersOpen((v) => !v)}>
            <span className="files-arrow">{othersOpen ? "▼" : "▶"}</span>
            {others.length} {others.length === 1 ? "version" : "versions"} of other files of this mod
          </button>
          {othersOpen && others.map((e) => <ChangelogVersion key={e.ver} entry={e} kind="foreign" showFile />)}
        </>
      )}
    </div>
  );
}

// OUTDATED while Nexus has something newer; LATEST once it was updated this
// session. For a multi-part mod, any part decides.
function DetailsStatusBadge({ mod, justUpdated, siblings = [] }) {
  const members = mod._isGroup ? siblings : [mod];
  if (mod.update_available) return <span className="details-upd-badge">OUTDATED</span>;
  if (members.some((m) => justUpdated[m.id])) return <span className="details-upd-badge details-latest-badge">LATEST</span>;
  return null;
}

// The Version row doubles as the changelog toggle: it already says how far
// behind the install is (→ latest), and unfolding it shows what's in between.
// The whole row is the target, with the arrow at its end, like Files.
// `brief` is the flatlined view, which shows the version and nothing else.
// `version` overrides what is shown as installed: a part of a multi-part mod
// shows its own file's version ("1.4.1n"), not the mod's ("3.3.0").
function VersionRow({ mod, state, onToggle, hint, updatedFrom, brief = false, version }) {
  const shownVersion = version ?? mod.version;
  const { status, data, lines, open } = state;
  // A part's arrow points at the newest version of its own file, not the
  // mod's: LUT Switcher's Nova pack 1.4.0n goes to 1.4.1n, not the core's 3.3.0
  const latest = (status === "ready" && data && readChangelog(data, lines, mod).lineLatest) || mod.latest_version;
  const count = data ? Object.keys(data).length : 0;
  // No Nexus id, or Nexus has no history for it: a plain row
  const toggles = !!mod.mod_id && !(status === "ready" && count === 0);
  const expanded = toggles && open && status === "ready";
  const rowRef = useRef(null);
  const panelRef = useRef(null);
  useRevealOnOpen(expanded, rowRef, panelRef);
  const note =
    status === "loading" ? "fetching changelog…"
    : status === "error" ? "couldn't fetch changelog — click to retry"
    : null;
  return (
    <>
      <div
        ref={rowRef}
        className={`detail-row ${toggles ? "files-toggle-row" : ""}`}
        onClick={toggles ? onToggle : undefined}
        {...(toggles
          ? hint(
              expanded ? "collapse changelog"
              : status === "ready" ? "show what changed in each version"
              : "fetch this mod's changelog and latest version from NexusMods"
            )
          : {})}
      >
        <span className="label">Version</span>
        <span className="value files-toggle-value">
          {shownVersion ?? <span className="version-date">parts differ</span>}
          {!brief && !mod.update_available && updatedFrom && updatedFrom !== shownVersion && (
            <span className="version-updated-from"> · updated from {updatedFrom}</span>
          )}
          {!brief && mod.update_available && shownVersion == null && mod.latest_version && (
            // A group whose parts disagree: nothing to point an arrow from
            <span className="version-date"> · <span className="version-update-badge">v{mod.latest_version}</span> on Nexus</span>
          )}
          {!brief && mod.update_available && shownVersion != null && (
            // The arrow only when the declared version really is newer. A mod
            // can be OUTDATED because its author retired the installed file
            // while the mod's own version stayed put ("1.0.1 → v1.0.1") or
            // fell behind ("1.3.2 → v1") — then say what actually happened.
            compareVersions(latest, shownVersion) > 0 ? (
              <>
                <span className="version-arrow"> → </span>
                <span className="version-update-badge">v{latest}</span>
              </>
            ) : (
              <span
                className="version-update-note"
                title="The author moved your file to Old versions on NexusMods"
              > · newer file on Nexus</span>
            )
          )}
          {!brief && mod.nexus_updated_at && (
            <span className="version-date"> · {relativeDate(mod.nexus_updated_at)}</span>
          )}
          {note && <span className="version-date"> · {note}</span>}
          {toggles && <span className="files-arrow">{expanded ? "▼" : "▶"}</span>}
        </span>
      </div>
      {expanded && <ChangelogPanel changelog={data} lines={lines} mod={mod} ref={panelRef} />}
    </>
  );
}

const REQUIREMENT_STATE = {
  installed: { mark: "◆", label: "slotted", cls: "ok" },
  on_disk: { mark: "◆", label: "on disk, no record", cls: "ok" },
  ghosted: { mark: "◇", label: "ghosted — not loaded", cls: "warn" },
  flatlined: { mark: "✗", label: "flatlined — files removed", cls: "bad" },
  missing: { mark: "✗", label: "not installed", cls: "bad" },
  unchecked: { mark: "·", label: "not checked", cls: "dim" },
};

// What the mod's page lists under Requirements, each checked against this
// install. Nexus gives no versions, only the author's note, shown as written.
function RequirementsRow({ mod, allMods, onSelectMod, hint }) {
  const [checks, setChecks] = useState(null);
  const [open, setOpen] = useState(false);
  const rowRef = useRef(null);
  const listRef = useRef(null);
  useRevealOnOpen(open, rowRef, listRef);
  const recordId = mod._isGroup ? mod._siblings?.[0]?.id : mod.id;

  useEffect(() => {
    let current = true;
    setChecks(null);
    if (!recordId || mod.removed) return;
    const read = () =>
      invoke("get_requirements", { modId: recordId })
        .then((list) => current && setChecks(list))
        .catch((err) => console.error("get_requirements:", err));
    read();
    // Another mod's install, Ghost or Flatline changes what's satisfied
    const unlisteners = ["mod-installed", "mod-toggled", "mod-removed", "sync-complete"].map((n) => listen(n, read));
    return () => {
      current = false;
      unlisteners.forEach((p) => p.then((unlisten) => unlisten()));
    };
  }, [recordId, mod.removed]);

  if (!checks || checks.length === 0) return null;
  const problems = checks.filter((c) => !c.optional && ["ghosted", "flatlined", "missing"].includes(c.state));

  const openRequirement = (c) => {
    const record = c.record_id && allMods?.find((m) => m.id === c.record_id);
    if (record) onSelectMod?.(record);
    else if (c.url) openUrl(c.url);
    else if (c.mod_id) openUrl(`https://www.nexusmods.com/cyberpunk2077/mods/${c.mod_id}?tab=files`);
  };

  return (
    <>
      <div
        ref={rowRef}
        className="detail-row files-toggle-row"
        onClick={() => setOpen((v) => !v)}
        {...hint(open ? "collapse requirements" : "what this mod needs, per its NexusMods page")}
      >
        <span className="label">Requires</span>
        <span className="value files-toggle-value">
          {checks.length} {checks.length === 1 ? "item" : "items"}
          {problems.length > 0 && <span className="req-problem-count"> · {problems.length} not loaded</span>}
          <span className="files-arrow">{open ? "▼" : "▶"}</span>
        </span>
      </div>
      {open && (
        <div className="req-list" ref={listRef}>
          {checks.map((c, i) => {
            const st = REQUIREMENT_STATE[c.state] ?? REQUIREMENT_STATE.unchecked;
            const linkable = !c.dlc && (c.record_id || c.url || c.mod_id);
            return (
              <div key={i} className={`req-item req-${st.cls} ${c.optional ? "req-optional" : ""}`}>
                <span className="req-mark">{st.mark}</span>
                <span className="req-body">
                  <span
                    className={`req-name ${linkable ? "req-link" : ""}`}
                    onClick={linkable ? () => openRequirement(c) : undefined}
                  >
                    {c.name}{c.dlc && <span className="req-tag">DLC</span>}
                  </span>
                  <span className="req-state">{st.label}</span>
                  {c.notes && <span className="req-notes">{c.notes}</span>}
                </span>
              </div>
            );
          })}
        </div>
      )}
    </>
  );
}

function ModDetails({ mod, justUpdated = {}, siblings = [], allMods = [], onSelectMod, onRemove, onForget, onToggle, onJackIn, onModsChanged, loading, hint = () => ({}) }) {
  const [filesOpen, setFilesOpen] = useState(false);
  // Files other live records hold too — Ghost and Flatline leave them in place.
  const [shared, setShared] = useState({});
  const filesRowRef = useRef(null);
  const fileListRef = useRef(null);
  useRevealOnOpen(filesOpen, filesRowRef, fileListRef);
  // Changelogs come from NETRUN's cache, so they match the UPD badge.
  // status: missing (never fetched) | ready | loading | error
  const [changelog, setChangelog] = useState({ status: "missing", data: null, open: false });
  const changelogFor = useRef(null);

  // Cache lookup is local — no NexusMods request while browsing the list.
  // Keeps the panel open or closed as it was.
  const readCachedChangelog = (modId) =>
    invoke("get_mod_changelog", { modId })
      .then((cached) => {
        if (changelogFor.current !== modId || !cached) return;
        setChangelog((c) => ({ status: "ready", data: cached.versions, lines: cached.lines, open: c.open }));
      })
      .catch((err) => console.error("get_mod_changelog:", err));

  useEffect(() => {
    const modId = mod?.mod_id ?? null;
    changelogFor.current = modId;
    setChangelog({ status: "missing", data: null, open: false });
    if (modId) readCachedChangelog(modId);
  }, [mod?.id, mod?.mod_id]);

  const sharedFor = useRef(null);
  useEffect(() => {
    const id = mod && !mod._isGroup && !mod.removed ? mod.id : null;
    sharedFor.current = id;
    setShared({});
    if (!id) return;
    const read = () =>
      invoke("get_shared_files", { modId: id })
        .then((list) => {
          if (sharedFor.current !== id) return;
          setShared(Object.fromEntries(list.map((s) => [s.path, s.holders])));
        })
        .catch((err) => console.error("get_shared_files:", err));
    read();
    // Another mod's Ghost, Flatline or install changes who holds what
    const unlisteners = ["mod-toggled", "mod-removed", "mod-installed"].map((name) => listen(name, read));
    return () => unlisteners.forEach((p) => p.then((unlisten) => unlisten()));
  }, [mod?.id, mod?._isGroup, mod?.removed]);

  // NETRUN and installs rewrite the cache while the same mod stays selected
  useEffect(() => {
    const unlisteners = ["sync-complete", "mod-installed"].map((name) =>
      listen(name, () => {
        if (changelogFor.current) readCachedChangelog(changelogFor.current);
      })
    );
    return () => unlisteners.forEach((p) => p.then((unlisten) => unlisten()));
  }, []);

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
        setChangelog({ status: "ready", data: cached?.versions ?? {}, lines: cached?.lines, open: true });
        onModsChanged?.();
      })
      .catch((err) => {
        if (changelogFor.current !== modId) return;
        console.error("refresh_mod:", err);
        setChangelog({ status: "error", data: null, open: false, error: String(err) });
      });
  };

  // One part of a mod installed as several: its record's `version` is the
  // mod's at install time ("3.3.0" for every LUT Switcher pack), so the part
  // is shown by its own file's version
  const isPart = !!mod && !mod._isGroup && siblings.filter((m) => m.id !== mod.id).length > 0;
  const versionRow = (props) =>
    mod && (
      <VersionRow
        mod={mod} state={changelog} onToggle={toggleChangelog} hint={hint}
        updatedFrom={justUpdated[mod.id]?.from}
        version={isPart && mod.file_version ? mod.file_version : undefined}
        {...props}
      />
    );

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
      const { enabled, kept } = await invoke("toggle_mod", { modId: mod.id });
      onToggle?.(mod.id, enabled, kept);
    } catch (error) {
      console.error("Failed to toggle mod:", error);
      alert("Failed to toggle mod: " + error);
    }
  };

  const fileCount = mod.files?.length ?? 0;
  const sharedCount = Object.keys(shared).length;

  const parts = [...siblings].sort((a, b) => (a.file_name ?? '').localeCompare(b.file_name ?? ''));

  if (mod._isGroup) {
    return (
      <div className="mod-details">
        <div className="mod-details-header">
          <h2 title={formerly(mod.former_names)}><DetailsStatusBadge mod={mod} justUpdated={justUpdated} siblings={parts} />{mod.name}</h2>
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
                    <span className="submod-item-name">
                      {s.file_name || `File #${s.file_id || "?"}`}
                      {s.update_available && <span className="submod-outdated">OUTDATED</span>}
                    </span>
                    {showDesc && <span className="submod-item-desc">{p.text}</span>}
                  </span>
                </span>
              );
            })}
          </div>

          <div className="detail-section">
            {versionRow()}
            <RequirementsRow mod={mod} allMods={allMods} onSelectMod={onSelectMod} hint={hint} />
            <div className="detail-row">
              <span className="label">Author</span>
              <AuthorValue mod={mod} hint={hint} />
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

        {/* No Update for the whole group: each part is its own file on Nexus,
            fetched with its own click there, and a group has no file to ask
            for — it opened the Files tab and the pick could land as a new
            part beside the old one. Parts are updated from their own row. */}
        {mod.update_available && (
          <div className="mod-details-footer">
            <span className="group-update-note">Update each OUTDATED part from its own row</span>
          </div>
        )}
      </div>
    );
  }

  if (mod.removed) {
    return (
      <div className="mod-details mod-details-removed">
        <div className="mod-details-header">
          <h2 className="removed-title" title={formerly(mod.former_names)}>{mod.name}</h2>
          <span className="removed-badge">FLATLINED</span>
        </div>

        <div className="mod-details-content">
          {cleanSummary(mod.summary) && (
            <p className="mod-summary mod-summary-dim">{cleanSummary(mod.summary)}</p>
          )}

          <div className="detail-section">
            {versionRow({ brief: true })}
            <div className="detail-row">
              <span className="label">Author</span>
              <AuthorValue mod={mod} hint={hint} />
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
        <h2 title={formerly(mod.former_names)}><DetailsStatusBadge mod={mod} justUpdated={justUpdated} />{mod.name}</h2>
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
          {versionRow()}
          <div className="detail-row">
            <span className="label">Author</span>
            <AuthorValue mod={mod} hint={hint} />
          </div>
          <div className="detail-row">
            <span className="label">Mod ID</span>
            <span className="value">{mod.mod_id || 'N/A'}</span>
          </div>
          <div className="detail-row">
            <span className="label">File ID</span>
            <span className="value">
              {mod.file_id || 'N/A'}
              {!isPart && mod.file_version && mod.file_version !== mod.version && (
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

          <RequirementsRow mod={mod} allMods={allMods} onSelectMod={onSelectMod} hint={hint} />

          {/* Files row — inline toggle inside the info section */}
          <div
            ref={filesRowRef}
            className="detail-row files-toggle-row"
            onClick={() => setFilesOpen(v => !v)}
            {...hint(filesOpen ? "collapse file list" : "show installed files")}
          >
            <span className="label">Files</span>
            <span className="value files-toggle-value">
              {fileCount} {fileCount === 1 ? 'file' : 'files'}
              {sharedCount > 0 && <span className="files-shared-count"> · {sharedCount} shared</span>}
              <span className="files-arrow">{filesOpen ? '▼' : '▶'}</span>
            </span>
          </div>
        </div>

        {filesOpen && (
          <div className="file-list" ref={fileListRef}>
            {fileCount > 0 ? (
              mod.files.map((file, index) => (
                <div key={index} className="file-item" title={file}>
                  <span className="file-path">{file.replace(/^.*?Cyberpunk 2077\//, '')}</span>
                  {shared[file] && (
                    <span
                      className="file-shared"
                      title={`Also installed by ${shared[file].map((h) => `${h.mod_name}${h.enabled ? "" : " (unslotted)"}`).join(", ")} — Ghost and Flatline leave it in place while that mod needs it`}
                    >
                      shared · {shared[file].map((h) => h.mod_name).join(", ")}
                    </span>
                  )}
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
