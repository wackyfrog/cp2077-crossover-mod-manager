import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./SideloadOverlay.css";

/**
 * Metadata confirmation step for installing a mod from a local archive.
 *
 * Only the input phase lives here — once the backend starts emitting
 * `install-progress`, JackInOverlay takes over and shows the transfer, exactly
 * as it does for an NXM install.
 */
export default function SideloadOverlay({ open, archivePath, onSubmit, onCancel }) {
  const [fileLabel, setFileLabel] = useState(null);
  const [sizeLabel, setSizeLabel] = useState(null);
  const [name, setName] = useState("");
  const [version, setVersion] = useState("");
  const [author, setAuthor] = useState("");
  const [modId, setModId] = useState("");
  const [fileStamp, setFileStamp] = useState(null);
  const [error, setError] = useState(null);
  const [parsing, setParsing] = useState(false);
  const nameRef = useRef(null);

  // Re-parse whenever a new archive is picked
  useEffect(() => {
    if (!open || !archivePath) return;
    let cancelled = false;

    setParsing(true);
    setError(null);
    invoke("parse_local_archive_name", { path: archivePath })
      .then((meta) => {
        if (cancelled) return;
        setFileLabel(meta.file_name || archivePath);
        setSizeLabel(meta.size_label || null);
        setName(meta.name || "");
        setVersion(meta.version || "");
        setModId(meta.mod_id || "");
        setFileStamp(meta.file_stamp || null);
        setAuthor("");
      })
      .catch((e) => {
        if (cancelled) return;
        // Parsing is a convenience, not a gate — fall back to a blank form.
        setFileLabel(archivePath.split("/").pop() || archivePath);
        setSizeLabel(null);
        setName("");
        setVersion("");
        setModId("");
        setFileStamp(null);
        setError(`Could not read the file name: ${e}`);
      })
      .finally(() => {
        if (!cancelled) {
          setParsing(false);
          setTimeout(() => nameRef.current?.focus(), 80);
        }
      });

    return () => {
      cancelled = true;
    };
  }, [open, archivePath]);

  if (!open) return null;

  const handleInstall = () => {
    const trimmedName = name.trim();
    if (!trimmedName) {
      setError("Mod name is required");
      return;
    }
    if (modId.trim() && !/^\d+$/.test(modId.trim())) {
      setError("Mod ID must be a number, or left empty");
      return;
    }
    setError(null);
    onSubmit({
      archive_path: archivePath,
      mod_name: trimmedName,
      mod_version: version.trim(),
      mod_author: author.trim(),
      mod_id: modId.trim(),
      file_stamp: fileStamp,
    });
  };

  const onKeyDown = (e) => {
    if (e.key === "Enter") handleInstall();
    if (e.key === "Escape") onCancel();
  };

  return (
    <div className="sideload-overlay" onKeyDown={onKeyDown}>
      <div className="sideload-chrome-bar">
        <span className="sideload-chrome-tag">LOCAL SHARD</span>
        <span className="sideload-chrome-divider" />
        <span className="sideload-chrome-sys">CROSSOVER MOD MANAGER v{__APP_VERSION__}</span>
        <span className="sideload-chrome-divider" />
        <span className="sideload-chrome-status">AWAITING CONFIRMATION</span>
      </div>

      <div className="sideload-body">
        <div className="sideload-title">Sideload</div>
        <p className="sideload-desc">
          Installing from an archive on disk — for mods that have no{" "}
          <span className="sideload-desc-em">"Download with Mod Manager"</span> button.
          Details below are guessed from the file name; correct anything that looks wrong.
        </p>

        <div className="sideload-file">
          <span className="sideload-file-icon">◈</span>
          <span className="sideload-file-name">{fileLabel || archivePath}</span>
          {sizeLabel && <span className="sideload-file-size">{sizeLabel}</span>}
        </div>

        {error && <p className="sideload-error">{error}</p>}

        <div className="sideload-fields">
          <label className="sideload-field">
            <span className="sideload-label">Name</span>
            <input
              ref={nameRef}
              className="sideload-input"
              value={name}
              disabled={parsing}
              onChange={(e) => {
                setName(e.target.value);
                setError(null);
              }}
            />
          </label>

          <div className="sideload-field-row">
            <label className="sideload-field">
              <span className="sideload-label">Version</span>
              <input
                className="sideload-input"
                value={version}
                placeholder="1.0"
                disabled={parsing}
                onChange={(e) => setVersion(e.target.value)}
              />
            </label>

            <label className="sideload-field">
              <span className="sideload-label">Author</span>
              <input
                className="sideload-input"
                value={author}
                placeholder="optional"
                disabled={parsing}
                onChange={(e) => setAuthor(e.target.value)}
              />
            </label>

            <label className="sideload-field">
              <span className="sideload-label">Mod ID</span>
              <input
                className="sideload-input"
                value={modId}
                placeholder="optional"
                disabled={parsing}
                onChange={(e) => {
                  setModId(e.target.value);
                  setError(null);
                }}
              />
            </label>
          </div>

          <p className="sideload-hint">
            A Nexus Mod ID lets Netrun fetch the thumbnail, summary, and update alerts for
            this mod. Clear it to keep the mod fully local.
          </p>
        </div>

        <div className="sideload-actions">
          <button className="sideload-btn" onClick={onCancel}>
            Cancel
          </button>
          <button
            className="sideload-btn primary"
            disabled={parsing || !name.trim()}
            onClick={handleInstall}
          >
            Install
          </button>
        </div>
      </div>
    </div>
  );
}
