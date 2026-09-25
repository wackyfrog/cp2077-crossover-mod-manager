use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModInfo {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: Option<String>,
    pub description: Option<String>,
    pub mod_id: Option<String>,
    pub file_id: Option<String>,
    pub enabled: bool,
    pub files: Vec<String>,

    // File ownership tracking for conflict detection
    // Map of relative file path -> conflict info
    #[serde(default)]
    pub file_conflicts: HashMap<String, FileConflictInfo>,

    // Install timestamp for determining which mod was installed first
    #[serde(default)]
    pub installed_at: Option<String>,

    // Synced from Nexus API
    #[serde(default)]
    pub picture_url: Option<String>,
    #[serde(default)]
    pub update_available: Option<bool>,
    #[serde(default)]
    pub latest_version: Option<String>,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub nexus_updated_at: Option<String>, // ISO 8601 date string from Nexus updated_timestamp
    /// The Nexus account that uploaded the mod. `author` is free text the
    /// uploader types in and needn't match any account, so the profile link
    /// is built from this member id instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uploader_id: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub uploader_name: Option<String>,

    // Soft-delete: mod is removed from game files but record kept
    #[serde(default)]
    pub removed: bool,
    #[serde(default)]
    pub removed_at: Option<String>,

    // Human-readable file name from Nexus (e.g. "Main File", "Optional Addon")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_name: Option<String>,

    // File-level version from Nexus (may differ from mod-level version)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_version: Option<String>,

    // Per-file description from Nexus (may contain HTML)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub file_description: Option<String>,

    // Latest file_id from Nexus for this file's name (used to generate update download URL)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub latest_file_id: Option<String>,

    // Reinstall state machine: None = normal, Some("prepare"|"removing"|"installing")
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reinstall_status: Option<String>,

    /// Names the mod had on Nexus before, oldest first. Authors put the
    /// version in the title ("Nova LUT 3.0 …" became "Nova LUT 4.0 …"), and
    /// the user still searches by the name they installed it under.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub former_names: Vec<String>,

    /// The mod's Requirements on Nexus, from NETRUN. None until fetched.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requirements: Option<Requirements>,
}

/// What a mod's page lists under Requirements. Nexus gives no versions — only
/// the mod and the author's free-text note ("MANDATORY", "Optional",
/// "v1.30.0 or newer").
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
pub struct Requirements {
    #[serde(default)]
    pub mods: Vec<ModRequirement>,
    #[serde(default)]
    pub dlc: Vec<DlcRequirement>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModRequirement {
    /// Nexus mod id; None for an off-site requirement.
    pub mod_id: Option<String>,
    pub name: String,
    pub notes: Option<String>,
    /// Hosted elsewhere (`url`) — shown, not checked.
    pub external: bool,
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DlcRequirement {
    pub name: String,
    pub notes: Option<String>,
}

/// Where a requirement stands in this install.
#[derive(Debug, Clone, Copy, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RequirementState {
    /// A live, switched-on record of that mod.
    Installed,
    /// Recorded but switched off: on disk, not loaded.
    Ghosted,
    /// Only a flatlined record — its files are gone.
    Flatlined,
    /// No record, but the framework's own file is in the game folder
    /// (installed by hand or by another tool).
    OnDisk,
    Missing,
    /// Off-site or a DLC this app can't look for: shown, not judged.
    Unchecked,
}

#[derive(Debug, Clone, Serialize)]
pub struct RequirementCheck {
    pub name: String,
    pub mod_id: Option<String>,
    pub notes: Option<String>,
    pub url: Option<String>,
    pub dlc: bool,
    /// The author's note calls it optional.
    pub optional: bool,
    pub state: RequirementState,
    /// The record that satisfies it, to link to.
    pub record_id: Option<String>,
}

impl RequirementCheck {
    /// A requirement the mod may not work without: not optional, and not
    /// loaded by the game (missing, ghosted or flatlined).
    pub fn is_problem(&self) -> bool {
        !self.optional
            && matches!(self.state, RequirementState::Ghosted | RequirementState::Flatlined | RequirementState::Missing)
    }
}

/// A file each common framework puts in the game folder, relative to it —
/// taken from their records on a real install. They are often installed by
/// hand, so no record doesn't mean not installed.
const FRAMEWORK_FILES: &[(&str, &str)] = &[
    ("107", "bin/x64/plugins/cyber_engine_tweaks.asi"),
    ("1511", "engine/tools/scc.exe"),
    ("2380", "red4ext/RED4ext.dll"),
    ("4197", "red4ext/plugins/TweakXL/TweakXL.dll"),
    ("4198", "red4ext/plugins/ArchiveXL/ArchiveXL.dll"),
    ("7780", "red4ext/plugins/Codeware/Codeware.dll"),
];

/// DLC whose presence shows in the game folder.
const DLC_FOLDERS: &[(&str, &str)] = &[("phantom liberty", "archive/pc/ep1")];

/// "Optional", "Optional - MANDATORY for LEO addon": the author marks the
/// requirement as not needed for the mod itself.
fn notes_say_optional(notes: Option<&str>) -> bool {
    notes.is_some_and(|n| n.to_lowercase().contains("optional"))
}

impl ModInfo {
    /// Take the name the mod has on Nexus now, keeping the old one findable.
    /// Returns whether anything changed.
    pub fn follow_nexus_name(&mut self, nexus_name: &str) -> bool {
        let nexus_name = nexus_name.trim();
        if nexus_name.is_empty() || nexus_name == self.name {
            return false;
        }
        let old = std::mem::replace(&mut self.name, nexus_name.to_string());
        self.former_names.retain(|n| n != nexus_name && *n != old);
        self.former_names.push(old);
        true
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileConflictInfo {
    // The mod ID that originally owned this file (if any)
    pub previous_owner: Option<String>,
    // The mod name for user-friendly display
    pub previous_owner_name: Option<String>,
    // Whether this is an archive file (important for load order)
    pub is_archive: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct ModDatabase {
    mods: Vec<ModInfo>,
}

/// Result of relocating tracked mod files from one game root to another.
#[derive(Debug, Serialize)]
pub struct RelocateReport {
    /// Files physically moved/copied to the new root.
    pub moved: usize,
    /// Tracked files that no longer exist on disk (neither enabled nor ghosted).
    pub not_found: usize,
    /// Files that failed to copy, with a reason each.
    pub failed: Vec<String>,
    /// Number of mods that had at least one file relocated.
    pub mods_affected: usize,
}

/// A file of one mod that other live records hold too.
#[derive(Debug, Clone, Serialize)]
pub struct SharedFile {
    pub path: String,
    pub holders: Vec<FileHolder>,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileHolder {
    pub mod_id: String,
    pub mod_name: String,
    pub enabled: bool,
}

/// What a Ghost or Enable did.
pub struct ToggleOutcome {
    pub enabled: bool,
    pub log_entries: Vec<String>,
    /// Files left active on Ghost because an enabled record holds them.
    pub kept: Vec<SharedFile>,
}

/// What re-ghosting a switched-off mod's update did.
pub struct GhostOutcome {
    pub renamed: usize,
    /// Left active because an enabled record holds them too.
    pub kept: Vec<SharedFile>,
    pub failed: Vec<String>,
}

/// What a Flatline did.
pub struct RemoveOutcome {
    pub mod_name: String,
    pub removed: Vec<String>,
    pub failed: Vec<String>,
    /// Files left on disk because another live record holds them.
    pub kept: Vec<SharedFile>,
}

/// What became of a mod's tracked files when we tried to delete them.
pub struct DeleteOutcome {
    /// Paths actually deleted; may carry the `.disabled` suffix.
    pub removed: Vec<String>,
    /// Tracked paths that were already absent — not a failure.
    pub already_gone: Vec<String>,
    /// Tracked path + reason, for everything still on disk.
    pub failed: Vec<(String, String)>,
}

impl DeleteOutcome {
    /// Failures rendered for the log and the UI.
    pub fn failure_reports(&self) -> Vec<String> {
        self.failed
            .iter()
            .map(|(file, why)| format!("{}: {}", file, why))
            .collect()
    }
}

/// Delete a mod's tracked files, matching both the active name and the ghosted
/// (`.disabled`) one.
///
/// A disabled mod keeps every file on disk under the `.disabled` suffix, so
/// matching only the active name deleted nothing while the record was still
/// flatlined — the files stayed behind, permanently out of the manager's sight
/// (see docs/bugs.md B2). Both variants go when both exist, which happens if a
/// mod was reinstalled on top of an earlier orphan.
///
/// A file that is already absent is reported separately from a genuine failure:
/// only the latter means something survived the removal.
pub fn delete_tracked_files(files: &[String]) -> DeleteOutcome {
    let mut outcome = DeleteOutcome {
        removed: Vec::new(),
        already_gone: Vec::new(),
        failed: Vec::new(),
    };

    for tracked in files {
        if let Err(reason) = check_safe_to_delete(tracked) {
            eprintln!("⛔ Skipping {}: {}", tracked, reason);
            outcome.failed.push((tracked.clone(), reason));
            continue;
        }

        let variants: Vec<PathBuf> = [
            PathBuf::from(tracked),
            PathBuf::from(format!("{}.disabled", tracked)),
        ]
        .into_iter()
        .filter(|p| p.exists())
        .collect();

        if variants.is_empty() {
            outcome.already_gone.push(tracked.clone());
            continue;
        }

        for path in variants {
            match fs::remove_file(&path) {
                Ok(_) => outcome.removed.push(path.display().to_string()),
                Err(e) => {
                    eprintln!("Failed to remove file {}: {}", path.display(), e);
                    outcome.failed.push((tracked.clone(), e.to_string()));
                }
            }
        }
    }

    outcome
}

/// How a tracked file on disk compares with the state its record calls for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FileState {
    /// Present, in the state the record calls for.
    AsExpected,
    /// Present, but in the opposite state — the mod does not behave as listed.
    Mismatched,
    /// Neither spelling exists.
    Missing,
}

/// Check a tracked file against its mod's enabled state.
///
/// "Does this path exist?" is the wrong question on its own: a switched-off mod
/// keeps its files under a `.disabled` suffix, so asking only about the active
/// name reports every file of every disabled mod as gone (docs/bugs.md B4).
///
/// Simply accepting either spelling would hide the case worth knowing about,
/// though. What decides the verdict is whether the file is in the state that
/// makes the mod behave as listed — `enabled == active`:
///
/// - a mod listed as enabled whose file is ghosted does nothing in-game
/// - a mod listed as disabled whose file is active runs anyway
///
/// Both are real, both are invisible everywhere else, and neither is a missing
/// file. A stray extra copy of the *other* spelling is ignored: no loader reads
/// `.disabled`, so an enabled mod with a leftover ghost still works correctly.
pub fn file_state(path: &str, enabled: bool) -> FileState {
    let active = Path::new(path).exists();
    let ghosted = PathBuf::from(format!("{}.disabled", path)).exists();

    if !active && !ghosted {
        FileState::Missing
    } else if enabled == active {
        FileState::AsExpected
    } else {
        FileState::Mismatched
    }
}

/// Refuse to delete anything that is not an absolute path inside a Cyberpunk
/// 2077 install.
fn check_safe_to_delete(file_path: &str) -> Result<(), String> {
    let path = Path::new(file_path);
    if !path.is_absolute() || file_path.contains("..") {
        return Err("path safety check failed".to_string());
    }
    if !file_path.to_lowercase().contains("cyberpunk 2077") {
        return Err("outside game directory".to_string());
    }
    Ok(())
}

pub struct ModManager {
    database_path: PathBuf,
    mods: Vec<ModInfo>,
    last_modified: Option<std::time::SystemTime>,
}

impl ModManager {
    /// Build a manager over a throwaway database, so tests never touch the
    /// user's real `~/.crossover-mod-manager/mods.json`.
    #[cfg(test)]
    fn with_database(database_path: PathBuf, mods: Vec<ModInfo>) -> Self {
        Self {
            database_path,
            mods,
            last_modified: None,
        }
    }

    pub fn new() -> Self {
        let database_path = Self::get_database_path();
        let mods = Self::load_database(&database_path);
        let last_modified = fs::metadata(&database_path).ok().and_then(|m| m.modified().ok());

        Self {
            database_path,
            mods,
            last_modified,
        }
    }

    /// Reload from disk if the file was modified by another process
    pub fn reload_if_changed(&mut self) {
        let current = fs::metadata(&self.database_path).ok().and_then(|m| m.modified().ok());
        if current != self.last_modified {
            self.mods = Self::load_database(&self.database_path);
            self.last_modified = current;
        }
    }

    fn get_database_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let app_dir = home.join(".crossover-mod-manager");

        if !app_dir.exists() {
            fs::create_dir_all(&app_dir).ok();
        }

        app_dir.join("mods.json")
    }

    fn load_database(path: &Path) -> Vec<ModInfo> {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(db) = serde_json::from_str::<ModDatabase>(&content) {
                    return db.mods;
                }
            }
        }
        Vec::new()
    }

    pub fn save_database(&mut self) -> Result<(), String> {
        let db = ModDatabase {
            mods: self.mods.clone(),
        };

        let json = serde_json::to_string_pretty(&db)
            .map_err(|e| format!("Failed to serialize database: {}", e))?;

        fs::write(&self.database_path, json)
            .map_err(|e| format!("Failed to write database: {}", e))?;

        self.last_modified = fs::metadata(&self.database_path).ok().and_then(|m| m.modified().ok());

        Ok(())
    }

    pub fn get_installed_mods(&self) -> Vec<ModInfo> {
        self.mods.clone()
    }

    /// Move (or copy) all tracked mod files from `old_root` to `new_root` and
    /// rewrite the database paths accordingly.
    ///
    /// Handles ghosted mods: `ModInfo.files` stores the logical (enabled) path,
    /// while a disabled file lives on disk with a `.disabled` suffix (see
    /// `toggle_mod`). We relocate whichever variant actually exists and keep the
    /// record pointing at the logical path under the new root.
    ///
    /// With `delete_source == true` the original file is removed after a
    /// successful copy (a "move"); otherwise the old files are left in place.
    pub fn relocate_mods(
        &mut self,
        old_root: &str,
        new_root: &str,
        delete_source: bool,
    ) -> Result<RelocateReport, String> {
        let old = Path::new(old_root);
        let new = Path::new(new_root);

        // Nothing to do (and copying a file onto itself would truncate it).
        if old == new {
            return Ok(RelocateReport {
                moved: 0,
                not_found: 0,
                failed: Vec::new(),
                mods_affected: 0,
            });
        }

        let mut moved = 0usize;
        let mut not_found = 0usize;
        let mut failed: Vec<String> = Vec::new();
        let mut mods_affected = 0usize;

        for mod_info in &mut self.mods {
            if mod_info.removed || mod_info.files.is_empty() {
                continue;
            }

            let mut new_files: Vec<String> = Vec::with_capacity(mod_info.files.len());
            let mut touched = false;

            for f in &mod_info.files {
                let fp = Path::new(f);

                // Only relocate paths that actually live under the old root.
                let rel = match fp.strip_prefix(old) {
                    Ok(r) => r,
                    Err(_) => {
                        new_files.push(f.clone());
                        continue;
                    }
                };

                let dest = new.join(rel);
                let dest_str = dest.to_string_lossy().to_string();

                // Which on-disk variant exists: enabled or ghosted (.disabled)?
                let src_enabled = fp.to_path_buf();
                let src_disabled = PathBuf::from(format!("{}.disabled", f));

                let (src, dst) = if src_enabled.exists() {
                    (src_enabled, dest.clone())
                } else if src_disabled.exists() {
                    (src_disabled, PathBuf::from(format!("{}.disabled", dest_str)))
                } else {
                    // File missing on disk — record the new logical path but flag it.
                    not_found += 1;
                    touched = true;
                    new_files.push(dest_str);
                    continue;
                };

                if let Some(parent) = dst.parent() {
                    if let Err(e) = fs::create_dir_all(parent) {
                        failed.push(format!("{}: {}", dst.display(), e));
                        new_files.push(f.clone()); // keep old path on failure
                        continue;
                    }
                }

                match fs::copy(&src, &dst) {
                    Ok(_) => {
                        if delete_source {
                            fs::remove_file(&src).ok();
                        }
                        moved += 1;
                        touched = true;
                        new_files.push(dest_str);
                    }
                    Err(e) => {
                        failed.push(format!("{}: {}", dst.display(), e));
                        new_files.push(f.clone()); // keep old path on failure
                    }
                }
            }

            if touched {
                mods_affected += 1;
            }
            mod_info.files = new_files;
        }

        self.save_database()?;

        Ok(RelocateReport {
            moved,
            not_found,
            failed,
            mods_affected,
        })
    }

    pub fn add_mod(&mut self, mod_info: ModInfo) {
        self.mods.push(mod_info);
    }

    /// Check if a mod is already installed based on mod_id and file_id
    pub fn find_existing_mod(&self, mod_id: &str, file_id: &str) -> Option<&ModInfo> {
        self.mods.iter().find(|mod_info| {
            if let (Some(existing_mod_id), Some(existing_file_id)) =
                (&mod_info.mod_id, &mod_info.file_id)
            {
                existing_mod_id == mod_id && existing_file_id == file_id
            } else {
                false
            }
        })
    }

    /// Check if a mod with the same name and version is already installed
    pub fn find_existing_mod_by_name(&self, name: &str, version: &str) -> Option<&ModInfo> {
        self.mods
            .iter()
            .find(|mod_info| mod_info.name == name && mod_info.version == version)
    }

    #[allow(dead_code)]
    /// Check if any version of a mod is already installed (by mod_id only)
    pub fn find_existing_mod_by_id(&self, mod_id: &str) -> Option<&ModInfo> {
        self.mods.iter().find(|mod_info| {
            if let Some(existing_mod_id) = &mod_info.mod_id {
                existing_mod_id == mod_id
            } else {
                false
            }
        })
    }

    #[allow(dead_code)]
    pub async fn install_mod(
        &mut self,
        mod_data: serde_json::Value,
        settings: &crate::settings::Settings,
    ) -> Result<(), String> {
        // Extract mod information from the data
        let name = mod_data
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown Mod")
            .to_string();

        let version = mod_data
            .get("version")
            .and_then(|v| v.as_str())
            .unwrap_or("1.0.0")
            .to_string();

        let download_url = mod_data
            .get("download_url")
            .and_then(|v| v.as_str())
            .ok_or("No download URL provided")?;

        // Download the mod file
        let mod_file = self.download_mod(download_url).await?;

        // Extract the archive
        let extracted_files = self.extract_mod(&mod_file, &settings.game_path)?;

        // Install files to game directory
        let installed_files = self.install_files(&extracted_files, &settings.game_path)?;

        // Create mod entry
        let mod_id = uuid::Uuid::new_v4().to_string();
        let mod_info = ModInfo {
            id: mod_id.clone(),
            name,
            version,
            author: mod_data
                .get("author")
                .and_then(|v| v.as_str())
                .map(String::from),
            description: mod_data
                .get("description")
                .and_then(|v| v.as_str())
                .map(String::from),
            mod_id: mod_data
                .get("mod_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            file_id: mod_data
                .get("file_id")
                .and_then(|v| v.as_str())
                .map(String::from),
            enabled: true,
            files: installed_files,
            file_conflicts: HashMap::new(),
            installed_at: Some(chrono::Utc::now().to_rfc3339()),
            picture_url: None,
            update_available: None,
            latest_version: None,
            summary: None,
            nexus_updated_at: None,
            uploader_id: None,
            uploader_name: None,
            removed: false,
            removed_at: None,
            file_name: None,
            file_version: None,
            file_description: None,
            latest_file_id: None,
            reinstall_status: None,
            former_names: Vec::new(),
            requirements: None,
        };

        self.mods.push(mod_info);
        self.save_database()?;

        // Clean up temporary files
        fs::remove_file(mod_file).ok();

        Ok(())
    }

    pub fn update_mod_sync_data(
        &mut self,
        mod_id: &str,
        summary: Option<String>,
        picture_url: Option<String>,
        update_available: bool,
        latest_version: Option<String>,
        nexus_updated_at: Option<String>,
        uploader: Option<(u64, String)>,
        corrected_version: Option<String>,
        update_target: Option<String>,
        nexus_name: Option<&str>,
        requirements: Option<&Requirements>,
    ) -> Result<(), String> {
        let mod_info = self
            .mods
            .iter_mut()
            .find(|m| m.id == mod_id)
            .ok_or("Mod not found")?;

        if let Some(r) = requirements {
            mod_info.requirements = Some(r.clone());
        }
        if let Some(n) = nexus_name {
            mod_info.follow_nexus_name(n);
        }
        if let Some(v) = corrected_version {
            mod_info.version = v;
        }
        mod_info.latest_file_id = update_target;
        mod_info.summary = summary;
        mod_info.picture_url = picture_url;
        mod_info.update_available = Some(update_available);
        mod_info.latest_version = latest_version;
        mod_info.nexus_updated_at = nexus_updated_at;
        if let Some((id, name)) = uploader {
            mod_info.uploader_id = Some(id);
            mod_info.uploader_name = Some(name);
        }

        self.save_database()
    }

    /// Which of `mod_id`'s files other live records hold too, and who holds
    /// them. Flatlined records own nothing. Paths compare case-insensitively,
    /// as on the APFS volume the game lives on.
    pub fn shared_files(&self, mod_id: &str) -> Vec<SharedFile> {
        let Some(target) = self.mods.iter().find(|m| m.id == mod_id) else {
            return Vec::new();
        };
        target
            .files
            .iter()
            .filter_map(|file| {
                let lower = file.to_lowercase();
                let holders: Vec<FileHolder> = self
                    .mods
                    .iter()
                    .filter(|m| m.id != mod_id && !m.removed)
                    .filter(|m| m.files.iter().any(|f| f.to_lowercase() == lower))
                    .map(|m| FileHolder {
                        mod_id: m.id.clone(),
                        mod_name: m.name.clone(),
                        enabled: m.enabled,
                    })
                    .collect();
                (!holders.is_empty()).then(|| SharedFile { path: file.clone(), holders })
            })
            .collect()
    }

    /// Ghost the files an update just installed for a record that stays
    /// switched off. A file another enabled record holds stays active, as on
    /// Ghost: the update may bring a file that mod also ships.
    pub fn reghost_after_update(&self, mod_id: &str, installed: &[String]) -> GhostOutcome {
        let shared: HashMap<String, SharedFile> = self
            .shared_files(mod_id)
            .into_iter()
            .map(|s| (s.path.to_lowercase(), s))
            .collect();
        let mut outcome = GhostOutcome { renamed: 0, kept: Vec::new(), failed: Vec::new() };
        for file in installed {
            if let Some(s) = shared.get(&file.to_lowercase()).filter(|s| s.holders.iter().any(|h| h.enabled)) {
                outcome.kept.push(s.clone());
                continue;
            }
            let original = Path::new(file);
            if !original.exists() {
                continue;
            }
            match fs::rename(original, format!("{}.disabled", file)) {
                Ok(()) => outcome.renamed += 1,
                Err(e) => outcome.failed.push(format!("{}: {}", file, e)),
            }
        }
        outcome
    }

    /// Each of the record's Nexus requirements against this install. Parts of
    /// one mod count as that mod: any live, switched-on part satisfies it.
    pub fn check_requirements(&self, record_id: &str, game_dir: &Path) -> Vec<RequirementCheck> {
        let Some(record) = self.mods.iter().find(|m| m.id == record_id) else {
            return Vec::new();
        };
        let Some(reqs) = &record.requirements else {
            return Vec::new();
        };
        let own = record.mod_id.as_deref().map(str::trim);
        let mut out = Vec::new();

        for r in &reqs.mods {
            let id = r.mod_id.as_deref().map(str::trim);
            if id.is_some() && id == own {
                continue;
            }
            let (state, record_id) = match id {
                None => (RequirementState::Unchecked, None),
                Some(_) if r.external => (RequirementState::Unchecked, None),
                Some(id) => {
                    let of_mod: Vec<&ModInfo> =
                        self.mods.iter().filter(|m| m.mod_id.as_deref().map(str::trim) == Some(id)).collect();
                    let live = |on: bool| of_mod.iter().find(|m| !m.removed && m.enabled == on);
                    if let Some(m) = live(true) {
                        (RequirementState::Installed, Some(m.id.clone()))
                    } else if FRAMEWORK_FILES
                        .iter()
                        .any(|(fid, file)| *fid == id && game_dir.join(file).exists())
                    {
                        (RequirementState::OnDisk, None)
                    } else if let Some(m) = live(false) {
                        (RequirementState::Ghosted, Some(m.id.clone()))
                    } else if let Some(m) = of_mod.first() {
                        (RequirementState::Flatlined, Some(m.id.clone()))
                    } else {
                        (RequirementState::Missing, None)
                    }
                }
            };
            out.push(RequirementCheck {
                name: r.name.clone(),
                mod_id: r.mod_id.clone(),
                notes: r.notes.clone(),
                url: r.url.clone(),
                dlc: false,
                optional: notes_say_optional(r.notes.as_deref()),
                state,
                record_id,
            });
        }

        for d in &reqs.dlc {
            let folder = DLC_FOLDERS.iter().find(|(name, _)| *name == d.name.to_lowercase()).map(|(_, f)| f);
            let state = match folder {
                Some(f) if game_dir.join(f).is_dir() => RequirementState::Installed,
                Some(_) => RequirementState::Missing,
                None => RequirementState::Unchecked,
            };
            out.push(RequirementCheck {
                name: d.name.clone(),
                mod_id: None,
                notes: d.notes.clone(),
                url: None,
                dlc: true,
                optional: notes_say_optional(d.notes.as_deref()),
                state,
                record_id: None,
            });
        }
        out
    }

    pub fn toggle_mod(&mut self, mod_id: &str) -> Result<ToggleOutcome, String> {
        let shared: HashMap<String, SharedFile> = self
            .shared_files(mod_id)
            .into_iter()
            .map(|s| (s.path.clone(), s))
            .collect();

        let mod_info = self
            .mods
            .iter_mut()
            .find(|m| m.id == mod_id)
            .ok_or("Mod not found")?;

        let enabling = !mod_info.enabled;
        let mut log_entries: Vec<String> = Vec::new();
        let mut kept: Vec<SharedFile> = Vec::new();

        for file_path in &mod_info.files {
            let original = Path::new(file_path);
            let disabled = PathBuf::from(format!("{}.disabled", file_path));

            if enabling {
                if original.exists() {
                    // An enabled mod kept it live while this one was ghosted;
                    // renaming the `.disabled` copy over it would replace that
                    // mod's file.
                    if disabled.exists() {
                        log_entries.push(format!("· Already active, left {}.disabled as is", file_path));
                    }
                } else if disabled.exists() {
                    fs::rename(&disabled, original).map_err(|e| {
                        format!("Failed to enable file {}: {}", file_path, e)
                    })?;
                    log_entries.push(format!("✓ Renamed: {}.disabled → {}", file_path, file_path));
                }
            } else if let Some(s) = shared.get(file_path).filter(|s| s.holders.iter().any(|h| h.enabled)) {
                // Ghosting it would switch it off for the enabled holder too.
                kept.push(s.clone());
            } else if original.exists() {
                fs::rename(original, &disabled).map_err(|e| {
                    format!("Failed to disable file {}: {}", file_path, e)
                })?;
                log_entries.push(format!("✓ Renamed: {} → {}.disabled", file_path, file_path));
            }
        }

        mod_info.enabled = enabling;
        self.save_database()?;
        Ok(ToggleOutcome { enabled: enabling, log_entries, kept })
    }

    pub fn remove_mod(&mut self, mod_id: &str) -> Result<RemoveOutcome, String> {
        let mod_index = self
            .mods
            .iter()
            .position(|m| m.id == mod_id)
            .ok_or("Mod not found")?;

        let mod_name = self.mods[mod_index].name.clone();
        let shared = self.shared_files(mod_id);
        let shared_paths: std::collections::HashSet<&str> =
            shared.iter().map(|s| s.path.as_str()).collect();
        let tracked: Vec<String> = self.mods[mod_index]
            .files
            .iter()
            .filter(|f| !shared_paths.contains(f.as_str()))
            .cloned()
            .collect();

        let mut outcome = delete_tracked_files(&tracked);

        // A file another live record holds stays on disk: deleting it would
        // take it from that mod too. When every other holder is ghosted, the
        // file is ghosted with them rather than left live for nobody.
        let mut kept: Vec<SharedFile> = Vec::new();
        for s in shared {
            let original = Path::new(&s.path);
            let disabled = PathBuf::from(format!("{}.disabled", s.path));
            if !s.holders.iter().any(|h| h.enabled) && original.exists() && !disabled.exists() {
                if let Err(e) = fs::rename(original, &disabled) {
                    outcome.failed.push((s.path.clone(), format!("could not ghost shared file: {}", e)));
                    continue;
                }
            }
            kept.push(s);
        }

        for gone in &outcome.already_gone {
            eprintln!("· Nothing to delete, already absent: {}", gone);
        }

        let mod_info = &mut self.mods[mod_index];
        if outcome.failed.is_empty() {
            // Soft-delete: keep record but clear file list and mark as removed
            mod_info.files = Vec::new();
            mod_info.file_conflicts = HashMap::new();
            mod_info.removed = true;
            mod_info.removed_at = Some(chrono::Utc::now().to_rfc3339());
            mod_info.enabled = false;
        } else {
            // Files are still on disk, so the record must not claim otherwise:
            // narrow the manifest to exactly what is left and keep the mod
            // listed, which leaves the user a removal to retry. `enabled` stays
            // as it was — the on-disk state is unchanged for those files.
            mod_info.files = outcome.failed.iter().map(|(f, _)| f.clone()).collect();
        }

        self.save_database()?;

        let failure_reports = outcome.failure_reports();
        Ok(RemoveOutcome { mod_name, removed: outcome.removed, failed: failure_reports, kept })
    }

    /// Update file_name, file_version, file_description, and latest_file_id for all mods with given mod_id
    pub fn update_file_info(&mut self, mod_id: &str, file_info: &HashMap<String, crate::nexusmods_api::FileInfo>) -> Result<(), String> {
        // Build reverse map: name -> highest file_id (latest version of that named file)
        let mut name_to_latest: HashMap<String, String> = HashMap::new();
        for (fid, (name, _, _)) in file_info {
            let fid_num: u64 = fid.parse().unwrap_or(0);
            let is_newer = name_to_latest.get(name)
                .map(|cur| fid_num > cur.parse::<u64>().unwrap_or(0))
                .unwrap_or(true);
            if is_newer {
                name_to_latest.insert(name.clone(), fid.clone());
            }
        }

        let mut changed = false;
        for mod_info in &mut self.mods {
            if mod_info.mod_id.as_deref() == Some(mod_id) {
                if let Some(file_id) = &mod_info.file_id {
                    if let Some((name, version, description)) = file_info.get(file_id) {
                        if mod_info.file_name.as_deref() != Some(name) {
                            mod_info.file_name = Some(name.clone());
                            changed = true;
                        }
                        if mod_info.file_version.as_deref() != version.as_deref() {
                            mod_info.file_version = version.clone();
                            changed = true;
                        }
                        if mod_info.file_description.as_deref() != description.as_deref() {
                            mod_info.file_description = description.clone();
                            changed = true;
                        }
                        // Find latest file_id for this named file
                        let latest = name_to_latest.get(name).cloned();
                        if mod_info.latest_file_id != latest {
                            mod_info.latest_file_id = latest;
                            changed = true;
                        }
                    }
                }
            }
        }
        if changed {
            self.save_database()?;
        }
        Ok(())
    }

    /// Set reinstall status on a mod and save DB
    /// Replace recorded file paths for one mod, using an old -> new mapping.
    /// Paths not present in the map are left untouched. Used by the wrapper-folder
    /// repair, which moves files on disk and must keep the database in step.
    pub fn rewrite_mod_files(
        &mut self,
        mod_id: &str,
        remap: &HashMap<String, String>,
    ) -> Result<(), String> {
        let mod_info = self
            .mods
            .iter_mut()
            .find(|m| m.id == mod_id)
            .ok_or("Mod not found")?;

        let mut changed = false;
        for file in mod_info.files.iter_mut() {
            if let Some(new_path) = remap.get(file.as_str()) {
                *file = new_path.clone();
                changed = true;
            }
        }

        // Conflict bookkeeping is keyed by path, so it has to follow the move.
        if !mod_info.file_conflicts.is_empty() {
            let remapped: HashMap<String, FileConflictInfo> = mod_info
                .file_conflicts
                .drain()
                .map(|(path, info)| (remap.get(&path).cloned().unwrap_or(path), info))
                .collect();
            mod_info.file_conflicts = remapped;
            changed = true;
        }

        if changed {
            self.save_database()?;
        }
        Ok(())
    }

    pub fn set_reinstall_status(&mut self, mod_id: &str, status: Option<&str>) -> Result<(), String> {
        let mod_info = self.mods.iter_mut().find(|m| m.id == mod_id).ok_or("Mod not found")?;
        mod_info.reinstall_status = status.map(|s| s.to_string());
        self.save_database()?;
        Ok(())
    }

    /// Remove mod files from disk but keep the record with reinstall_status
    #[allow(dead_code)]
    pub fn remove_mod_files(&mut self, mod_id: &str) -> Result<(String, Vec<String>, Vec<String>), String> {
        let mod_info = self.mods.iter_mut().find(|m| m.id == mod_id).ok_or("Mod not found")?;
        let mod_name = mod_info.name.clone();
        let tracked = mod_info.files.clone();

        // Ghost-aware, same as remove_mod: a disabled mod's files are on disk
        // under `.disabled` and would otherwise survive the reinstall.
        let outcome = delete_tracked_files(&tracked);

        let mod_info = self.mods.iter_mut().find(|m| m.id == mod_id).ok_or("Mod not found")?;
        mod_info.files = Vec::new();
        mod_info.file_conflicts = HashMap::new();
        self.save_database()?;

        let failure_reports = outcome.failure_reports();
        Ok((mod_name, outcome.removed, failure_reports))
    }

    /// Update mod record after successful reinstall (new files, version, file_id, etc).
    /// Returns the final `enabled` state so the caller can sync the on-disk files
    /// (a mod that stays ghosted must have its freshly installed files re-disabled).
    ///
    /// `same_file` is a reinstall of the very file the record already had —
    /// what an update turns into when Nexus hands back the installed file. The
    /// files are refreshed, but the version and the update flag stay as they
    /// were: nothing got newer.
    pub fn complete_reinstall(
        &mut self,
        mod_id: &str,
        new_files: Vec<String>,
        new_version: &str,
        new_file_id: Option<&str>,
        new_file_name: Option<String>,
        new_file_version: Option<String>,
        new_file_description: Option<String>,
        same_file: bool,
        nexus_name: Option<&str>,
    ) -> Result<bool, String> {
        let mod_info = self.mods.iter_mut().find(|m| m.id == mod_id).ok_or("Mod not found")?;
        if let Some(n) = nexus_name.filter(|_| mod_info.mod_id.is_some()) {
            mod_info.follow_nexus_name(n);
        }
        // Preserve the user's slot state across an update: an active mod keeps
        // its prior enabled/ghosted state, while reinstalling a flatlined
        // (removed) mod re-slots it.
        let target_enabled = if mod_info.removed { true } else { mod_info.enabled };
        mod_info.files = new_files;
        if !same_file {
            mod_info.version = new_version.to_string();
        }
        if let Some(fid) = new_file_id {
            mod_info.file_id = Some(fid.to_string());
        }
        if new_file_name.is_some() {
            mod_info.file_name = new_file_name;
        }
        if new_file_version.is_some() {
            mod_info.file_version = new_file_version;
        }
        if new_file_description.is_some() {
            mod_info.file_description = new_file_description;
        }
        mod_info.reinstall_status = None;
        mod_info.enabled = target_enabled;
        mod_info.removed = false;
        mod_info.removed_at = None;
        if !same_file {
            mod_info.update_available = Some(false);
        }
        mod_info.installed_at = Some(chrono::Utc::now().to_rfc3339());
        self.save_database()?;
        Ok(target_enabled)
    }

    /// Abort reinstall — restore mod to normal state (files may be gone)
    pub fn abort_reinstall(&mut self, mod_id: &str) -> Result<(), String> {
        let mod_info = self.mods.iter_mut().find(|m| m.id == mod_id).ok_or("Mod not found")?;
        mod_info.reinstall_status = None;
        // If files were removed, mark as removed
        if mod_info.files.is_empty() {
            mod_info.removed = true;
            mod_info.removed_at = Some(chrono::Utc::now().to_rfc3339());
            mod_info.enabled = false;
        }
        self.save_database()?;
        Ok(())
    }

    /// Permanently delete a removed mod's record from the database
    pub fn forget_mod(&mut self, mod_id: &str) -> Result<String, String> {
        let mod_index = self
            .mods
            .iter()
            .position(|m| m.id == mod_id)
            .ok_or("Mod not found")?;

        let mod_name = self.mods[mod_index].name.clone();
        self.mods.remove(mod_index);
        self.save_database()?;

        Ok(mod_name)
    }

    /// Remove duplicate records: same mod_id + file_name, keep newest by installed_at.
    /// Only removes DB records, does NOT delete files from disk.
    pub fn deduplicate_mods(&mut self) -> Result<Vec<String>, String> {
        let mut seen: HashMap<(String, String), usize> = HashMap::new(); // (mod_id, file_name) -> index of newest
        let mut to_remove: Vec<usize> = Vec::new();
        let mut removed_names: Vec<String> = Vec::new();

        for (i, m) in self.mods.iter().enumerate() {
            if let (Some(mod_id), Some(file_name)) = (&m.mod_id, &m.file_name) {
                let key = (mod_id.clone(), file_name.clone());
                if let Some(&existing_idx) = seen.get(&key) {
                    // Compare installed_at — keep newer, mark older for removal
                    let existing_time = self.mods[existing_idx].installed_at.as_deref().unwrap_or("");
                    let current_time = m.installed_at.as_deref().unwrap_or("");
                    if current_time > existing_time {
                        // Current is newer — remove the old one
                        to_remove.push(existing_idx);
                        removed_names.push(format!("{} (file_id: {})", self.mods[existing_idx].name, self.mods[existing_idx].file_id.as_deref().unwrap_or("?")));
                        seen.insert(key, i);
                    } else {
                        // Existing is newer — remove current
                        to_remove.push(i);
                        removed_names.push(format!("{} (file_id: {})", m.name, m.file_id.as_deref().unwrap_or("?")));
                    }
                } else {
                    seen.insert(key, i);
                }
            }
        }

        if !to_remove.is_empty() {
            to_remove.sort_unstable();
            to_remove.dedup();
            for &idx in to_remove.iter().rev() {
                self.mods.remove(idx);
            }
            self.save_database()?;
        }

        Ok(removed_names)
    }

    #[allow(dead_code)]
    async fn download_mod(&self, url: &str) -> Result<PathBuf, String> {
        let temp_dir = std::env::temp_dir();
        let filename = format!("mod_{}.zip", uuid::Uuid::new_v4());
        let file_path = temp_dir.join(filename);

        let response = reqwest::get(url)
            .await
            .map_err(|e| format!("Failed to download mod: {}", e))?;

        let bytes = response
            .bytes()
            .await
            .map_err(|e| format!("Failed to read download: {}", e))?;

        fs::write(&file_path, bytes).map_err(|e| format!("Failed to save download: {}", e))?;

        Ok(file_path)
    }

    #[allow(dead_code)]
    fn extract_mod(&self, archive_path: &Path, _game_path: &str) -> Result<PathBuf, String> {
        let temp_dir = std::env::temp_dir();
        let extract_dir = temp_dir.join(format!("mod_extract_{}", uuid::Uuid::new_v4()));

        fs::create_dir_all(&extract_dir)
            .map_err(|e| format!("Failed to create extraction directory: {}", e))?;

        let file =
            fs::File::open(archive_path).map_err(|e| format!("Failed to open archive: {}", e))?;

        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Failed to read archive: {}", e))?;

        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("Failed to read archive entry: {}", e))?;

            let outpath = extract_dir.join(file.name());

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath).ok();
            } else {
                if let Some(p) = outpath.parent() {
                    fs::create_dir_all(p).ok();
                }
                let mut outfile = fs::File::create(&outpath)
                    .map_err(|e| format!("Failed to create file: {}", e))?;
                io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Failed to extract file: {}", e))?;
            }
        }

        Ok(extract_dir)
    }

    #[allow(dead_code)]
    fn install_files(&self, extracted_dir: &Path, game_path: &str) -> Result<Vec<String>, String> {
        let game_dir = Path::new(game_path);
        if !game_dir.exists() {
            return Err("Game directory does not exist".to_string());
        }

        let mut installed_files = Vec::new();

        // Walk through extracted files and install them
        for entry in WalkDir::new(extracted_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                let relative_path = entry
                    .path()
                    .strip_prefix(extracted_dir)
                    .map_err(|e| e.to_string())?;

                // Determine installation path based on file structure
                let install_path = self.determine_install_path(game_dir, relative_path)?;

                // Create parent directories
                if let Some(parent) = install_path.parent() {
                    fs::create_dir_all(parent)
                        .map_err(|e| format!("Failed to create directory: {}", e))?;
                }

                // Copy file
                fs::copy(entry.path(), &install_path)
                    .map_err(|e| format!("Failed to copy file: {}", e))?;

                installed_files.push(install_path.to_string_lossy().to_string());
            }
        }

        // Clean up extraction directory
        fs::remove_dir_all(extracted_dir).ok();

        Ok(installed_files)
    }

    #[allow(dead_code)]
    fn determine_install_path(
        &self,
        game_dir: &Path,
        relative_path: &Path,
    ) -> Result<PathBuf, String> {
        // Try to detect common mod structure patterns
        let path_str = relative_path.to_string_lossy().to_lowercase();

        // Check for common Cyberpunk 2077 mod directories
        if path_str.contains("archive") || path_str.contains("archives") {
            Ok(game_dir
                .join("archive")
                .join("pc")
                .join("mod")
                .join(relative_path.file_name().unwrap()))
        } else if path_str.contains("bin") {
            Ok(game_dir
                .join("bin")
                .join("x64")
                .join(relative_path.file_name().unwrap()))
        } else if path_str.contains("r6") {
            Ok(game_dir
                .join("r6")
                .join("scripts")
                .join(relative_path.file_name().unwrap()))
        } else {
            // Default to archive/pc/mod for unknown files
            Ok(game_dir
                .join("archive")
                .join("pc")
                .join("mod")
                .join(relative_path.file_name().unwrap()))
        }
    }

    /// Check for file conflicts with already installed mods
    /// Returns a map of file paths to conflicting mod info
    /// Files other records already hold. `updating` is the record an update
    /// is replacing: its own files coming back are not a conflict.
    pub fn check_file_conflicts(
        &self,
        files_to_install: &[String],
        updating: Option<&str>,
    ) -> HashMap<String, Vec<ConflictDetails>> {
        let mut conflicts: HashMap<String, Vec<ConflictDetails>> = HashMap::new();

        for file_path in files_to_install {
            // Check if this file is already installed by another mod
            for existing_mod in self.mods.iter().filter(|m| Some(m.id.as_str()) != updating) {
                if existing_mod.files.contains(file_path) {
                    conflicts
                        .entry(file_path.clone())
                        .or_default()
                        .push(ConflictDetails {
                            mod_id: existing_mod.id.clone(),
                            mod_name: existing_mod.name.clone(),
                            mod_version: existing_mod.version.clone(),
                            is_archive: file_path.ends_with(".archive"),
                        });
                }
            }
        }

        conflicts
    }

    // TODO: Implement active load order management UI
    // Currently unused - load order detection is done inline during installation
    /*
    /// Analyze archive file load order conflicts
    /// Returns warnings about which archive will override which
    #[allow(dead_code)]
    pub fn analyze_archive_load_order(&self, archive_files: &[String]) -> Vec<LoadOrderWarning> {
        let mut warnings = Vec::new();

        // Get all installed archive files from all mods
        let mut all_archives: Vec<(String, String, String)> = Vec::new(); // (filename, mod_name, mod_id)

        for existing_mod in &self.mods {
            for file in &existing_mod.files {
                if file.ends_with(".archive") {
                    if let Some(filename) = Path::new(file).file_name() {
                        all_archives.push((
                            filename.to_string_lossy().to_string(),
                            existing_mod.name.clone(),
                            existing_mod.id.clone(),
                        ));
                    }
                }
            }
        }

        // Add new archives being installed
        for file in archive_files {
            if let Some(filename) = Path::new(file).file_name() {
                all_archives.push((
                    filename.to_string_lossy().to_string(),
                    "NEW MOD".to_string(),
                    "new".to_string(),
                ));
            }
        }

        // Sort archives alphabetically (this is how CP2077 loads them)
        all_archives.sort_by(|a, b| a.0.cmp(&b.0));

        // Check for archives that might conflict
        // Group by basegame_ prefix or other common patterns
        let mut basegame_archives = Vec::new();
        let mut patch_archives = Vec::new();

        for (filename, mod_name, mod_id) in &all_archives {
            if filename.starts_with("basegame_") || filename.starts_with("basegame-") {
                basegame_archives.push((filename.clone(), mod_name.clone(), mod_id.clone()));
            } else if filename.starts_with("patch_") || filename.starts_with("patch-") {
                patch_archives.push((filename.clone(), mod_name.clone(), mod_id.clone()));
            }
        }

        // Warn if multiple mods modify basegame
        if basegame_archives.len() > 1 {
            let last_loaded = basegame_archives.last().unwrap();
            warnings.push(LoadOrderWarning {
                warning_type: LoadOrderWarningType::MultipleBasegameArchives,
                message: format!(
                    "Multiple mods modify basegame archives. '{}' will load last and override others.",
                    last_loaded.0
                ),
                affected_archives: basegame_archives.iter().map(|a| a.0.clone()).collect(),
                suggestion: Some(
                    "Consider renaming archives to control load order:\n\
                     - Prefix with '0-' to load first (e.g., '0-basegame_textures.archive')\n\
                     - Prefix with 'z-' to load last (e.g., 'z-basegame_final.archive')"
                        .to_string(),
                ),
            });
        }

        warnings
    }
    */
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConflictDetails {
    pub mod_id: String,
    pub mod_name: String,
    pub mod_version: String,
    pub is_archive: bool,
}

// TODO: Implement active load order management UI
// Currently unused - kept for future feature implementation
#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoadOrderWarning {
    pub warning_type: LoadOrderWarningType,
    pub message: String,
    pub affected_archives: Vec<String>,
    pub suggestion: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LoadOrderWarningType {
    MultipleBasegameArchives,
    MultiplePatchArchives,
    ConflictingMods,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A scratch game dir. The name must contain "Cyberpunk 2077" — the delete
    /// guard refuses anything outside an install.
    fn game_dir(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir()
            .join(format!("cmm_{}_{}", tag, uuid::Uuid::new_v4()))
            .join("Cyberpunk 2077");
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(game: &Path) {
        fs::remove_dir_all(game.parent().unwrap()).ok();
    }

    fn write(path: &Path, body: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, body).unwrap();
    }

    fn fixture(id: &str, files: Vec<String>) -> ModInfo {
        ModInfo {
            id: id.to_string(),
            name: format!("Mod {}", id),
            version: "1.0".to_string(),
            author: None,
            description: None,
            mod_id: None,
            file_id: None,
            enabled: false,
            files,
            file_conflicts: HashMap::new(),
            installed_at: None,
            picture_url: None,
            update_available: None,
            latest_version: None,
            summary: None,
            nexus_updated_at: None,
            uploader_id: None,
            uploader_name: None,
            removed: false,
            removed_at: None,
            file_name: None,
            file_version: None,
            file_description: None,
            latest_file_id: None,
            reinstall_status: None,
            former_names: Vec::new(),
            requirements: None,
        }
    }

    fn manager_with(tag: &str, mods: Vec<ModInfo>) -> (ModManager, PathBuf) {
        let game = game_dir(tag);
        let manager = ModManager {
            database_path: game.join("mods.json"),
            mods,
            last_modified: None,
        };
        (manager, game)
    }

    #[test]
    fn an_update_does_not_conflict_with_the_record_it_replaces() {
        let shared = "/g/Cyberpunk 2077/r6/scripts/Sleeves.reds".to_string();
        let own = "/g/Cyberpunk 2077/archive/pc/mod/sleeves.archive".to_string();
        let (manager, game) = manager_with(
            "conflicts",
            vec![fixture("sleeves", vec![shared.clone(), own.clone()]), fixture("other", vec![shared.clone()])],
        );
        let conflicts = manager.check_file_conflicts(&[shared.clone(), own.clone()], Some("sleeves"));
        assert_eq!(conflicts.len(), 1);
        assert_eq!(conflicts[&shared].iter().map(|c| c.mod_id.as_str()).collect::<Vec<_>>(), ["other"]);
        // a fresh install still sees every holder
        assert_eq!(manager.check_file_conflicts(&[own], None).len(), 1);
        cleanup(&game);
    }

    #[test]
    fn reinstalling_the_same_file_keeps_version_and_update_flag() {
        let mut outdated = fixture("a", vec![]);
        outdated.version = "2.30".into();
        outdated.file_id = Some("42".into());
        outdated.update_available = Some(true);
        let (mut manager, game) = manager_with("samefile", vec![outdated]);

        manager
            .complete_reinstall("a", vec!["x".into()], "2.40", Some("42"), None, Some("2.30".into()), None, true, None)
            .unwrap();

        let m = &manager.mods[0];
        assert_eq!(m.version, "2.30", "nothing newer was installed");
        assert_eq!(m.update_available, Some(true), "the update is still pending");
        assert_eq!(m.files, vec!["x".to_string()], "files are refreshed all the same");
        cleanup(&game);
    }

    #[test]
    fn a_real_update_takes_the_new_version() {
        let mut outdated = fixture("a", vec![]);
        outdated.version = "2.30".into();
        outdated.file_id = Some("42".into());
        outdated.update_available = Some(true);
        let (mut manager, game) = manager_with("newfile", vec![outdated]);

        manager
            .complete_reinstall("a", vec![], "2.40", Some("43"), None, Some("2.40".into()), None, false, None)
            .unwrap();

        let m = &manager.mods[0];
        assert_eq!(m.version, "2.40");
        assert_eq!(m.update_available, Some(false));
        assert_eq!(m.file_id.as_deref(), Some("43"));
        cleanup(&game);
    }

    #[test]
    fn deletes_the_ghosted_variant_the_active_name_would_miss() {
        let game = game_dir("ghost");
        let tracked = game.join("mods/Ghosted/init.lua");
        write(&PathBuf::from(format!("{}.disabled", tracked.display())), "-- off");

        let outcome = delete_tracked_files(&[tracked.display().to_string()]);

        assert_eq!(outcome.removed.len(), 1, "the .disabled file must be deleted");
        assert!(outcome.failed.is_empty());
        assert!(outcome.already_gone.is_empty());
        assert!(!PathBuf::from(format!("{}.disabled", tracked.display())).exists());
        cleanup(&game);
    }

    #[test]
    fn deletes_both_variants_when_both_exist() {
        let game = game_dir("both");
        let tracked = game.join("mods/Twice/init.lua");
        write(&tracked, "-- on");
        write(&PathBuf::from(format!("{}.disabled", tracked.display())), "-- off");

        let outcome = delete_tracked_files(&[tracked.display().to_string()]);

        assert_eq!(outcome.removed.len(), 2);
        assert!(!tracked.exists());
        assert!(!PathBuf::from(format!("{}.disabled", tracked.display())).exists());
        cleanup(&game);
    }

    #[test]
    fn a_file_already_absent_is_not_a_failure() {
        let game = game_dir("absent");
        let tracked = game.join("mods/Vanished/init.lua");

        let outcome = delete_tracked_files(&[tracked.display().to_string()]);

        assert!(outcome.removed.is_empty());
        assert!(outcome.failed.is_empty(), "ENOENT must not read as a failure");
        assert_eq!(outcome.already_gone.len(), 1);
        cleanup(&game);
    }

    #[test]
    fn a_disabled_mods_ghosted_file_is_not_missing() {
        // The B4 case: 910 files reported gone that were all on disk, ghosted.
        let game = game_dir("state_ghost");
        let tracked = game.join("mods/Off/init.lua");
        write(&PathBuf::from(format!("{}.disabled", tracked.display())), "-- off");

        let path = tracked.display().to_string();
        assert_eq!(file_state(&path, false), FileState::AsExpected);
        assert_eq!(
            file_state(&path, true),
            FileState::Mismatched,
            "listed as enabled but ghosted: the mod does nothing in-game"
        );
        cleanup(&game);
    }

    #[test]
    fn an_enabled_mods_active_file_is_as_expected() {
        let game = game_dir("state_on");
        let tracked = game.join("mods/On/init.lua");
        write(&tracked, "-- on");

        let path = tracked.display().to_string();
        assert_eq!(file_state(&path, true), FileState::AsExpected);
        assert_eq!(
            file_state(&path, false),
            FileState::Mismatched,
            "listed as disabled but active: the mod runs anyway"
        );
        cleanup(&game);
    }

    #[test]
    fn a_stray_ghost_beside_an_active_file_is_harmless() {
        let game = game_dir("state_both");
        let tracked = game.join("mods/Both/init.lua");
        write(&tracked, "-- on");
        write(&PathBuf::from(format!("{}.disabled", tracked.display())), "-- leftover");

        // No loader reads `.disabled`, so the enabled mod still works.
        assert_eq!(file_state(&tracked.display().to_string(), true), FileState::AsExpected);
        cleanup(&game);
    }

    #[test]
    fn neither_spelling_on_disk_is_missing() {
        let game = game_dir("state_gone");
        let path = game.join("mods/Gone/init.lua").display().to_string();

        assert_eq!(file_state(&path, true), FileState::Missing);
        assert_eq!(file_state(&path, false), FileState::Missing);
        cleanup(&game);
    }

    #[test]
    fn refuses_a_path_outside_the_game_directory() {
        let outside = std::env::temp_dir().join("cmm_outside_marker.txt");
        fs::write(&outside, "keep me").unwrap();

        let outcome = delete_tracked_files(&[outside.display().to_string()]);

        assert!(outcome.removed.is_empty());
        assert_eq!(outcome.failed.len(), 1);
        assert!(outside.exists(), "a path outside the install must survive");
        fs::remove_file(&outside).ok();
    }

    #[test]
    fn a_clean_sweep_flatlines_the_record() {
        let game = game_dir("flatline");
        let tracked = game.join("mods/Clean/init.lua");
        write(&PathBuf::from(format!("{}.disabled", tracked.display())), "-- off");

        let mut manager = ModManager::with_database(
            game.join("mods.json"),
            vec![fixture("m1", vec![tracked.display().to_string()])],
        );
        let RemoveOutcome { removed, failed, .. } = manager.remove_mod("m1").unwrap();

        assert_eq!(removed.len(), 1);
        assert!(failed.is_empty());
        let record = &manager.mods[0];
        assert!(record.removed, "nothing left on disk, so the record flatlines");
        assert!(record.files.is_empty());
        cleanup(&game);
    }

    #[test]
    fn a_file_left_behind_keeps_the_record_alive() {
        let game = game_dir("leftover");
        // A directory where a file is tracked: remove_file cannot delete it, so
        // this stands in for any undeletable file (locked, no permission).
        let stubborn = game.join("mods/Stubborn/init.lua");
        fs::create_dir_all(&stubborn).unwrap();

        let mut manager = ModManager::with_database(
            game.join("mods.json"),
            vec![fixture("m2", vec![stubborn.display().to_string()])],
        );
        let RemoveOutcome { removed, failed, .. } = manager.remove_mod("m2").unwrap();

        assert!(removed.is_empty());
        assert_eq!(failed.len(), 1);
        let record = &manager.mods[0];
        assert!(
            !record.removed,
            "the record must not claim removal while the file is still there"
        );
        assert_eq!(
            record.files,
            vec![stubborn.display().to_string()],
            "the manifest narrows to what is left, so the user can retry"
        );
        cleanup(&game);
    }

    /// Two records holding one active file: `radioManager.lua` shipped by both
    /// RadioExt and the patch that replaces it.
    fn two_holders(tag: &str, owner_enabled: bool) -> (ModManager, PathBuf, PathBuf, PathBuf) {
        let game = game_dir(tag);
        let shared = game.join("bin/x64/plugins/cyber_engine_tweaks/mods/radioExt/radioManager.lua");
        let own = game.join("archive/pc/mod/patch.archive");
        write(&shared, "-- patched");
        write(&own, "patch");
        let mut owner = fixture("owner", vec![shared.display().to_string()]);
        owner.enabled = owner_enabled;
        let mut patch = fixture("patch", vec![shared.display().to_string(), own.display().to_string()]);
        patch.enabled = true;
        let manager = ModManager::with_database(game.join("mods.json"), vec![owner, patch]);
        (manager, game, shared, own)
    }

    fn ghost(path: &Path) -> PathBuf {
        PathBuf::from(format!("{}.disabled", path.display()))
    }

    #[test]
    fn shared_files_name_live_holders_only() {
        let (mut manager, game, shared, _) = two_holders("shared_scan", true);
        let found = manager.shared_files("patch");
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].path, shared.display().to_string());
        assert_eq!(found[0].holders.iter().map(|h| h.mod_id.as_str()).collect::<Vec<_>>(), ["owner"]);

        // a path spelled in another case is the same file on APFS
        manager.mods[0].files = vec![shared.display().to_string().to_uppercase()];
        assert_eq!(manager.shared_files("patch").len(), 1);

        manager.mods[0].removed = true;
        assert!(manager.shared_files("patch").is_empty(), "a flatlined record owns nothing");
        cleanup(&game);
    }

    #[test]
    fn ghosting_leaves_a_file_an_enabled_mod_holds() {
        let (mut manager, game, shared, own) = two_holders("ghost_guard", true);

        let outcome = manager.toggle_mod("patch").unwrap();

        assert!(!outcome.enabled);
        assert!(shared.exists(), "the owner still runs on it");
        assert!(!ghost(&shared).exists());
        assert_eq!(outcome.kept.len(), 1);
        assert!(ghost(&own).exists(), "the patch's own file is ghosted as usual");
        cleanup(&game);
    }

    #[test]
    fn ghosting_takes_a_file_only_disabled_mods_hold() {
        let (mut manager, game, shared, _) = two_holders("ghost_free", false);

        let outcome = manager.toggle_mod("patch").unwrap();

        assert!(outcome.kept.is_empty());
        assert!(ghost(&shared).exists());
        assert!(!shared.exists());
        cleanup(&game);
    }

    #[test]
    fn enabling_does_not_rename_over_a_file_already_active() {
        let (mut manager, game, shared, _) = two_holders("enable_over", true);
        manager.mods[1].enabled = false;
        write(&ghost(&shared), "-- stale copy");

        let outcome = manager.toggle_mod("patch").unwrap();

        assert!(outcome.enabled);
        assert_eq!(fs::read_to_string(&shared).unwrap(), "-- patched", "the active file is untouched");
        assert!(ghost(&shared).exists(), "the ghost is left, not renamed over it");
        cleanup(&game);
    }

    #[test]
    fn flatlining_keeps_a_file_another_enabled_mod_holds() {
        let (mut manager, game, shared, own) = two_holders("flatline_guard", true);

        let outcome = manager.remove_mod("patch").unwrap();

        assert!(shared.exists(), "the owner still runs on it");
        assert!(!own.exists());
        assert_eq!(outcome.kept.len(), 1);
        assert!(outcome.failed.is_empty());
        let record = manager.mods.iter().find(|m| m.id == "patch").unwrap();
        assert!(record.removed, "a kept shared file is not a failed removal");
        cleanup(&game);
    }

    #[test]
    fn flatlining_ghosts_a_file_only_disabled_mods_hold() {
        let (mut manager, game, shared, _) = two_holders("flatline_ghost", false);

        let outcome = manager.remove_mod("patch").unwrap();

        assert!(!shared.exists(), "nothing enabled wants it live");
        assert!(ghost(&shared).exists(), "the disabled owner still has it to switch back on");
        assert_eq!(outcome.kept.len(), 1);
        cleanup(&game);
    }

    #[test]
    fn a_new_nexus_name_keeps_the_old_one_findable() {
        let mut m = fixture("nova", vec![]);
        m.name = "Nova LUT 3.0 (AgX - HDR Support)".into();

        assert!(m.follow_nexus_name("Nova LUT 4.0 (AgX - New HDR)"));
        assert_eq!(m.name, "Nova LUT 4.0 (AgX - New HDR)");
        assert_eq!(m.former_names, ["Nova LUT 3.0 (AgX - HDR Support)"]);

        assert!(!m.follow_nexus_name("Nova LUT 4.0 (AgX - New HDR)"), "same name: nothing to do");
        assert!(!m.follow_nexus_name("  "), "an empty title from Nexus is ignored");

        // renamed back: the current name leaves the history, the one it replaces joins it
        assert!(m.follow_nexus_name("Nova LUT 3.0 (AgX - HDR Support)"));
        assert_eq!(m.former_names, ["Nova LUT 4.0 (AgX - New HDR)"]);
    }

    #[test]
    fn netrun_and_update_take_the_nexus_name() {
        let mut nexus = fixture("nexus", vec![]);
        nexus.mod_id = Some("11622".into());
        nexus.name = "Old".into();
        let mut local = fixture("local", vec![]);
        local.name = "Typed by user".into();
        let (mut manager, game) = manager_with("names", vec![nexus, local]);

        manager
            .update_mod_sync_data("nexus", None, None, false, None, None, None, None, None, Some("Synced"), None)
            .unwrap();
        assert_eq!(manager.mods[0].name, "Synced");

        manager
            .complete_reinstall("nexus", vec![], "2", Some("9"), None, None, None, false, Some("Updated"))
            .unwrap();
        assert_eq!(manager.mods[0].name, "Updated");
        assert_eq!(manager.mods[0].former_names, ["Old", "Synced"]);

        // no Nexus id: the name is the user's, not Nexus's
        manager
            .complete_reinstall("local", vec![], "2", None, None, None, None, false, Some("Whatever"))
            .unwrap();
        assert_eq!(manager.mods[1].name, "Typed by user");
        cleanup(&game);
    }

    #[test]
    fn a_ghosted_update_leaves_a_file_an_enabled_mod_holds() {
        // "patch" is switched off and was just updated; its new files are live
        let (mut manager, game, shared, own) = two_holders("reghost", true);
        manager.mods[1].enabled = false;
        let installed = vec![shared.display().to_string(), own.display().to_string()];

        let outcome = manager.reghost_after_update("patch", &installed);

        assert_eq!(outcome.renamed, 1);
        assert!(ghost(&own).exists(), "its own file goes back to .disabled");
        assert!(shared.exists(), "the enabled owner still runs on it");
        assert_eq!(outcome.kept.len(), 1);
        assert!(outcome.failed.is_empty());
        cleanup(&game);
    }

    fn needs(id: &str, name: &str, notes: Option<&str>) -> ModRequirement {
        ModRequirement { mod_id: Some(id.into()), name: name.into(), notes: notes.map(Into::into), external: false, url: None }
    }

    fn of_mod(id: &str, nexus_id: &str, enabled: bool, removed: bool) -> ModInfo {
        let mut m = fixture(id, vec![]);
        m.mod_id = Some(nexus_id.into());
        m.enabled = enabled;
        m.removed = removed;
        m
    }

    #[test]
    fn requirements_are_judged_by_what_the_game_loads() {
        let game = game_dir("reqs");
        write(&game.join("red4ext/RED4ext.dll"), "dll");
        fs::create_dir_all(game.join("archive/pc/ep1")).unwrap();

        let mut me = of_mod("me", "25873", true, false);
        me.requirements = Some(Requirements {
            mods: vec![
                needs("25552", "NCTO", Some("MANDATORY")),
                needs("1511", "redscript", Some("MANDATORY")),
                needs("4197", "TweakXL", None),
                needs("2380", "RED4ext", Some("MANDATORY")),
                needs("10135", "LEO", Some("Optional - MANDATORY for LEO addon")),
                needs("5036", "Claim Vehicles", None),
                needs("25873", "itself", None),
            ],
            dlc: vec![DlcRequirement { name: "Phantom Liberty".into(), notes: None }],
        });
        let mods = vec![
            me,
            of_mod("ncto-core", "25552", false, false),
            of_mod("ncto-addon", "25552", true, false), // one live part is enough
            of_mod("redscript", "1511", false, false),
            of_mod("tweakxl", "4197", false, true),
        ];
        let (manager, _) = manager_with("reqs_db", mods);

        let checks = manager.check_requirements("me", &game);
        let state = |name: &str| checks.iter().find(|c| c.name == name).map(|c| c.state);
        assert_eq!(state("NCTO"), Some(RequirementState::Installed));
        assert_eq!(state("redscript"), Some(RequirementState::Ghosted));
        assert_eq!(state("TweakXL"), Some(RequirementState::Flatlined));
        assert_eq!(state("RED4ext"), Some(RequirementState::OnDisk), "installed by hand");
        assert_eq!(state("Claim Vehicles"), Some(RequirementState::Missing));
        assert_eq!(state("itself"), None, "a mod doesn't require itself");
        assert_eq!(state("Phantom Liberty"), Some(RequirementState::Installed));

        let problems: Vec<&str> = checks.iter().filter(|c| c.is_problem()).map(|c| c.name.as_str()).collect();
        assert_eq!(problems, ["redscript", "TweakXL", "Claim Vehicles"], "LEO is optional per its note");
        cleanup(&game);
    }

    #[test]
    fn a_mod_never_synced_has_nothing_to_check() {
        let (manager, game) = manager_with("reqs_none", vec![of_mod("me", "1", true, false)]);
        assert!(manager.check_requirements("me", &game).is_empty());
        cleanup(&game);
    }
}
