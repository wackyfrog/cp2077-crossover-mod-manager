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

pub struct ModManager {
    database_path: PathBuf,
    mods: Vec<ModInfo>,
    last_modified: Option<std::time::SystemTime>,
}

impl ModManager {
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
            removed: false,
            removed_at: None,
            file_name: None,
            file_version: None,
            file_description: None,
            latest_file_id: None,
            reinstall_status: None,
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
    ) -> Result<(), String> {
        let mod_info = self
            .mods
            .iter_mut()
            .find(|m| m.id == mod_id)
            .ok_or("Mod not found")?;

        mod_info.summary = summary;
        mod_info.picture_url = picture_url;
        mod_info.update_available = Some(update_available);
        mod_info.latest_version = latest_version;
        mod_info.nexus_updated_at = nexus_updated_at;

        self.save_database()
    }

    pub fn toggle_mod(&mut self, mod_id: &str) -> Result<(bool, Vec<String>), String> {
        let mod_info = self
            .mods
            .iter_mut()
            .find(|m| m.id == mod_id)
            .ok_or("Mod not found")?;

        let enabling = !mod_info.enabled;
        let mut log_entries: Vec<String> = Vec::new();

        for file_path in &mod_info.files {
            let original = Path::new(file_path);
            let disabled = PathBuf::from(format!("{}.disabled", file_path));

            if enabling {
                if disabled.exists() {
                    fs::rename(&disabled, original).map_err(|e| {
                        format!("Failed to enable file {}: {}", file_path, e)
                    })?;
                    log_entries.push(format!("✓ Renamed: {}.disabled → {}", file_path, file_path));
                }
            } else {
                if original.exists() {
                    fs::rename(original, &disabled).map_err(|e| {
                        format!("Failed to disable file {}: {}", file_path, e)
                    })?;
                    log_entries.push(format!("✓ Renamed: {} → {}.disabled", file_path, file_path));
                }
            }
        }

        mod_info.enabled = enabling;
        self.save_database()?;
        Ok((enabling, log_entries))
    }

    pub fn remove_mod(
        &mut self,
        mod_id: &str,
    ) -> Result<(String, Vec<String>, Vec<String>), String> {
        let mod_index = self
            .mods
            .iter()
            .position(|m| m.id == mod_id)
            .ok_or("Mod not found")?;

        let mod_info = &mut self.mods[mod_index];
        let mod_name = mod_info.name.clone();

        let mut removed_files = Vec::new();
        let mut failed_files = Vec::new();

        // Remove all installed files from disk (with path safety check)
        for file_path in &mod_info.files {
            let path = Path::new(file_path);
            // Safety: only delete absolute paths that don't contain traversal
            if !path.is_absolute() || file_path.contains("..") {
                eprintln!("⛔ Skipping unsafe path: {}", file_path);
                failed_files.push(format!("{}: path safety check failed", file_path));
                continue;
            }
            // Safety: must be within a Cyberpunk 2077 game directory
            let path_lower = file_path.to_lowercase();
            if !path_lower.contains("cyberpunk 2077") {
                eprintln!("⛔ Skipping path outside game directory: {}", file_path);
                failed_files.push(format!("{}: outside game directory", file_path));
                continue;
            }
            match fs::remove_file(file_path) {
                Ok(_) => {
                    removed_files.push(file_path.clone());
                }
                Err(e) => {
                    eprintln!("Failed to remove file {}: {}", file_path, e);
                    failed_files.push(format!("{}: {}", file_path, e));
                }
            }
        }

        // Soft-delete: keep record but clear file list and mark as removed
        mod_info.files = Vec::new();
        mod_info.file_conflicts = HashMap::new();
        mod_info.removed = true;
        mod_info.removed_at = Some(chrono::Utc::now().to_rfc3339());
        mod_info.enabled = false;

        self.save_database()?;

        Ok((mod_name, removed_files, failed_files))
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
        let mut removed_files = Vec::new();
        let mut failed_files = Vec::new();

        for file_path in &mod_info.files {
            let path = Path::new(file_path);
            if !path.is_absolute() || file_path.contains("..") || !file_path.to_lowercase().contains("cyberpunk 2077") {
                eprintln!("⛔ Skipping unsafe path: {}", file_path);
                failed_files.push(format!("{}: path safety check failed", file_path));
                continue;
            }
            match fs::remove_file(file_path) {
                Ok(_) => removed_files.push(file_path.clone()),
                Err(e) => failed_files.push(format!("{}: {}", file_path, e)),
            }
        }

        mod_info.files = Vec::new();
        mod_info.file_conflicts = HashMap::new();
        self.save_database()?;

        Ok((mod_name, removed_files, failed_files))
    }

    /// Update mod record after successful reinstall (new files, version, file_id, etc)
    pub fn complete_reinstall(
        &mut self,
        mod_id: &str,
        new_files: Vec<String>,
        new_version: &str,
        new_file_id: Option<&str>,
        new_file_name: Option<String>,
        new_file_version: Option<String>,
        new_file_description: Option<String>,
    ) -> Result<(), String> {
        let mod_info = self.mods.iter_mut().find(|m| m.id == mod_id).ok_or("Mod not found")?;
        mod_info.files = new_files;
        mod_info.version = new_version.to_string();
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
        mod_info.enabled = true;
        mod_info.removed = false;
        mod_info.removed_at = None;
        mod_info.update_available = Some(false);
        mod_info.installed_at = Some(chrono::Utc::now().to_rfc3339());
        self.save_database()?;
        Ok(())
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
    pub fn check_file_conflicts(
        &self,
        files_to_install: &[String],
    ) -> HashMap<String, Vec<ConflictDetails>> {
        let mut conflicts: HashMap<String, Vec<ConflictDetails>> = HashMap::new();

        for file_path in files_to_install {
            // Check if this file is already installed by another mod
            for existing_mod in &self.mods {
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
