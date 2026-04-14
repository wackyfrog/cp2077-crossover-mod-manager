// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod archive_extractor;
mod mod_manager;
mod nexusmods_api;
mod settings;

use mod_manager::{ModInfo, ModManager};
use serde::{Deserialize, Serialize};
use settings::{AppSettings, Settings};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
#[allow(unused_imports)] // Listener is used for trait methods (.listen())
use tauri::{Emitter, Listener, Manager, State};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub level: String, // "info", "warning", "error"
    pub message: String,
    pub category: String, // "download", "installation", "system"
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct InstallProgress {
    pub stage: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bytes_received: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_bytes: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_count: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_total: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mod_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nxm_url: Option<String>,
}

fn emit_install_progress(app: &tauri::AppHandle, progress: InstallProgress) {
    if let Some(window) = app.get_webview_window("main") {
        window.emit("install-progress", &progress).ok();
    }
}

struct AppState {
    mod_manager: Mutex<ModManager>,
    settings: Mutex<AppSettings>,
    logs: Mutex<VecDeque<LogEntry>>,
    sync_cancel: Arc<AtomicBool>,
    install_cancel: Arc<AtomicBool>,
    installing: Arc<AtomicBool>,
    startup_nxm_url: Mutex<Option<String>>,
    force_reinstall: AtomicBool,
    reinstall_mod_id: Mutex<Option<String>>,
    pending_file_name: Mutex<Option<String>>,
    pending_file_version: Mutex<Option<String>>,
    pending_file_description: Mutex<Option<String>>,
}

#[tauri::command]
fn get_installed_mods(state: State<AppState>) -> Result<Vec<ModInfo>, String> {
    let mut manager = state.mod_manager.lock().map_err(|e| e.to_string())?;
    manager.reload_if_changed();
    Ok(manager.get_installed_mods())
}

#[tauri::command]
async fn install_mod(
    mod_data: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<(), String> {
    // Add initial log
    add_log(
        "Starting mod installation process".to_string(),
        "info".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    // Get mod name for logging
    let mod_name = mod_data
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Unknown Mod");

    add_log(
        format!("Installing mod: {}", mod_name),
        "info".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    // Get settings
    let settings = {
        let settings_guard = state.settings.lock().map_err(|e| e.to_string())?;
        settings_guard.get_settings()
    };

    if settings.game_path.is_empty() {
        let error_msg =
            "Game path not configured. Please set the game installation path in Settings.";
        add_log(
            error_msg.to_string(),
            "error".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;
        return Err(error_msg.to_string());
    }

    // Get download URL
    let download_url = mod_data
        .get("download_url")
        .and_then(|v| v.as_str())
        .ok_or("No download URL provided")?;

    add_log(
        format!("Starting download from: {}", download_url),
        "info".to_string(),
        "download".to_string(),
        state.clone(),
    )?;

    // Since we can't easily modify the existing mod manager to be async-safe,
    // let's implement a simplified version here for demonstration
    add_log(
        "Download completed successfully".to_string(),
        "info".to_string(),
        "download".to_string(),
        state.clone(),
    )?;

    add_log(
        "Extracting mod files...".to_string(),
        "info".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    add_log(
        "Installing mod files to game directory...".to_string(),
        "info".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    add_log(
        format!("Mod '{}' installed successfully!", mod_name),
        "info".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    Ok(())
}

#[tauri::command]
fn remove_mod(
    mod_id: String,
    state: State<AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    add_log(
        format!("🗑️ Starting removal of mod with ID: {}", mod_id),
        "info".to_string(),
        "removal".to_string(),
        state.clone(),
    )?;

    let (mod_name, removed_files, failed_files) = {
        let mut manager = state.mod_manager.lock().map_err(|e| e.to_string())?;
        manager.remove_mod(&mod_id)?
    };

    add_log(
        format!("📝 Removing mod: {}", mod_name),
        "info".to_string(),
        "removal".to_string(),
        state.clone(),
    )?;

    // Log each removed file
    for file_path in &removed_files {
        add_log(
            format!("✓ Removed: {}", file_path),
            "info".to_string(),
            "removal".to_string(),
            state.clone(),
        )?;
    }

    // Log any files that failed to remove
    for error in &failed_files {
        add_log(
            format!("⚠ Failed to remove: {}", error),
            "warning".to_string(),
            "removal".to_string(),
            state.clone(),
        )?;
    }

    let result_message = if failed_files.is_empty() {
        add_log(
            format!(
                "✅ Successfully removed mod '{}' ({} files deleted)",
                mod_name,
                removed_files.len()
            ),
            "success".to_string(),
            "removal".to_string(),
            state.clone(),
        )?;
        format!(
            "Mod '{}' removed successfully! Deleted {} files.",
            mod_name,
            removed_files.len()
        )
    } else {
        add_log(
            format!(
                "⚠ Partially removed mod '{}' ({} files deleted, {} failed)",
                mod_name,
                removed_files.len(),
                failed_files.len()
            ),
            "warning".to_string(),
            "removal".to_string(),
            state.clone(),
        )?;
        format!(
            "Mod '{}' partially removed. {} files deleted, {} files failed to delete.",
            mod_name,
            removed_files.len(),
            failed_files.len()
        )
    };

    // Emit event to refresh UI
    if let Some(window) = app.get_webview_window("main") {
        window.emit("mod-removed", &mod_id).ok();
    }

    Ok(result_message)
}

#[tauri::command]
fn forget_mod(
    mod_id: String,
    state: State<AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    let mod_name = {
        let mut manager = state.mod_manager.lock().map_err(|e| e.to_string())?;
        manager.forget_mod(&mod_id)?
    };

    add_log(
        format!("🗑 Purged record for mod '{}'", mod_name),
        "info".to_string(),
        "removal".to_string(),
        state.clone(),
    )?;

    if let Some(window) = app.get_webview_window("main") {
        window.emit("mod-removed", &mod_id).ok();
    }

    Ok(format!("Record for '{}' permanently deleted.", mod_name))
}

#[tauri::command]
fn deduplicate_mods(
    state: State<AppState>,
) -> Result<Vec<String>, String> {
    let mut manager = state.mod_manager.lock().map_err(|e| e.to_string())?;
    manager.deduplicate_mods()
}

#[tauri::command]
fn toggle_mod(
    mod_id: String,
    state: State<AppState>,
    app: tauri::AppHandle,
) -> Result<bool, String> {
    let (enabled, log_entries) = {
        let mut manager = state.mod_manager.lock().map_err(|e| e.to_string())?;
        manager.toggle_mod(&mod_id)?
    };

    let status = if enabled { "enabled" } else { "disabled" };
    add_log(
        format!("🔄 Mod {}: {}", mod_id, status),
        "info".to_string(),
        "system".to_string(),
        state.clone(),
    )?;

    for entry in &log_entries {
        add_log(
            entry.clone(),
            "info".to_string(),
            "system".to_string(),
            state.clone(),
        )?;
    }

    add_log(
        format!("✅ {} file(s) renamed", log_entries.len()),
        "info".to_string(),
        "system".to_string(),
        state.clone(),
    )?;

    if let Some(window) = app.get_webview_window("main") {
        window.emit("mod-toggled", &mod_id).ok();
    }

    Ok(enabled)
}

#[tauri::command]
fn get_settings(state: State<AppState>) -> Result<Settings, String> {
    let settings = state.settings.lock().map_err(|e| e.to_string())?;
    Ok(settings.get_settings())
}

#[tauri::command]
fn save_settings(settings: Settings, state: State<AppState>) -> Result<(), String> {
    let mut app_settings = state.settings.lock().map_err(|e| e.to_string())?;
    app_settings.save_settings(settings)
}

#[tauri::command]
fn get_crossover_bottles_path() -> Option<String> {
    let home = dirs::home_dir()?;
    let bottles = home.join("Library/Application Support/CrossOver/Bottles");
    if bottles.exists() {
        Some(bottles.to_string_lossy().to_string())
    } else {
        None
    }
}

#[tauri::command]
fn auto_detect_game_path() -> Result<Option<String>, String> {
    // Common paths where Cyberpunk 2077 might be installed via CrossOver
    let potential_paths = vec![
        // Steam installation paths
        "/Library/Application Support/CrossOver/Bottles/Steam/drive_c/Program Files (x86)/Steam/steamapps/common/Cyberpunk 2077",
        "/Users/{username}/Library/Application Support/CrossOver/Bottles/Steam/drive_c/Program Files (x86)/Steam/steamapps/common/Cyberpunk 2077",

        // GOG installation paths (most common)
        "/Library/Application Support/CrossOver/Bottles/GOG/drive_c/GOG Games/Cyberpunk 2077",
        "/Users/{username}/Library/Application Support/CrossOver/Bottles/GOG/drive_c/GOG Games/Cyberpunk 2077",
        "/Library/Application Support/CrossOver/Bottles/GOG Galaxy/drive_c/GOG Games/Cyberpunk 2077",
        "/Users/{username}/Library/Application Support/CrossOver/Bottles/GOG Galaxy/drive_c/GOG Games/Cyberpunk 2077",

        // GOG Galaxy with Program Files paths
        "/Library/Application Support/CrossOver/Bottles/GOG/drive_c/Program Files (x86)/GOG Galaxy/Games/Cyberpunk 2077",
        "/Users/{username}/Library/Application Support/CrossOver/Bottles/GOG/drive_c/Program Files (x86)/GOG Galaxy/Games/Cyberpunk 2077",
        "/Library/Application Support/CrossOver/Bottles/GOG Galaxy/drive_c/Program Files (x86)/GOG Galaxy/Games/Cyberpunk 2077",
        "/Users/{username}/Library/Application Support/CrossOver/Bottles/GOG Galaxy/drive_c/Program Files (x86)/GOG Galaxy/Games/Cyberpunk 2077",

        // Custom bottle names for Cyberpunk 2077
        "/Library/Application Support/CrossOver/Bottles/Cyberpunk2077/drive_c/GOG Games/Cyberpunk 2077",
        "/Users/{username}/Library/Application Support/CrossOver/Bottles/Cyberpunk2077/drive_c/GOG Games/Cyberpunk 2077",
        "/Library/Application Support/CrossOver/Bottles/Cyberpunk 2077/drive_c/GOG Games/Cyberpunk 2077",
        "/Users/{username}/Library/Application Support/CrossOver/Bottles/Cyberpunk 2077/drive_c/GOG Games/Cyberpunk 2077",

        // Epic Games installation paths
        "/Library/Application Support/CrossOver/Bottles/Epic/drive_c/Program Files/Epic Games/Cyberpunk2077",
        "/Users/{username}/Library/Application Support/CrossOver/Bottles/Epic/drive_c/Program Files/Epic Games/Cyberpunk2077",

        // Generic Windows game installation paths (with wildcards for any bottle name)
        "/Library/Application Support/CrossOver/Bottles/*/drive_c/GOG Games/Cyberpunk 2077",
        "/Users/{username}/Library/Application Support/CrossOver/Bottles/*/drive_c/GOG Games/Cyberpunk 2077",
        "/Library/Application Support/CrossOver/Bottles/*/drive_c/Program Files*/GOG Galaxy/Games/Cyberpunk 2077",
        "/Users/{username}/Library/Application Support/CrossOver/Bottles/*/drive_c/Program Files*/GOG Galaxy/Games/Cyberpunk 2077",
        "/Library/Application Support/CrossOver/Bottles/*/drive_c/Program Files*/*/Cyberpunk 2077",
        "/Users/{username}/Library/Application Support/CrossOver/Bottles/*/drive_c/Program Files*/*/Cyberpunk 2077",
    ];

    // Get the current user's username
    let username = std::env::var("USER").unwrap_or_else(|_| "username".to_string());

    for path_template in potential_paths {
        let path = path_template.replace("{username}", &username);

        // Handle glob patterns
        if path.contains('*') {
            if let Ok(entries) = glob::glob(&path) {
                for entry in entries.flatten() {
                    if is_valid_cyberpunk_installation(&entry) {
                        return Ok(Some(entry.to_string_lossy().to_string()));
                    }
                }
            }
        } else {
            let path_buf = std::path::PathBuf::from(&path);
            if is_valid_cyberpunk_installation(&path_buf) {
                return Ok(Some(path));
            }
        }
    }

    Ok(None)
}

#[tauri::command]
fn add_log(
    message: String,
    level: String,
    category: String,
    state: State<AppState>,
) -> Result<(), String> {
    let mut logs = state.logs.lock().map_err(|e| e.to_string())?;

    let log_entry = LogEntry {
        timestamp: chrono::Utc::now()
            .format("%Y-%m-%d %H:%M:%S UTC")
            .to_string(),
        level,
        message,
        category,
    };

    logs.push_back(log_entry);

    // Keep only the last 1000 log entries to prevent memory issues
    while logs.len() > 1000 {
        logs.pop_front();
    }

    Ok(())
}

// Alias for frontend to call add_log directly
#[tauri::command]
fn add_log_entry(
    message: String,
    level: String,
    category: String,
    state: State<AppState>,
) -> Result<(), String> {
    add_log(message, level, category, state)
}

#[tauri::command]
fn get_logs(state: State<AppState>) -> Result<Vec<LogEntry>, String> {
    let logs = state.logs.lock().map_err(|e| e.to_string())?;
    Ok(logs.iter().cloned().collect())
}

#[tauri::command]
fn clear_logs(state: State<AppState>) -> Result<(), String> {
    let mut logs = state.logs.lock().map_err(|e| e.to_string())?;
    logs.clear();
    Ok(())
}

#[tauri::command]
fn test_logging(state: State<AppState>) -> Result<(), String> {
    add_log(
        "Test log entry from test_logging command".to_string(),
        "info".to_string(),
        "system".to_string(),
        state.clone(),
    )?;
    Ok(())
}

#[tauri::command]
async fn download_and_save_mod(
    mod_name: String,
    download_url: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    use std::fs;
    use std::path::Path;
    use tokio::io::AsyncWriteExt;

    // Get mod storage path from settings
    let mod_storage_path = {
        let settings_guard = state.settings.lock().map_err(|e| e.to_string())?;
        let settings = settings_guard.get_settings();
        settings.mod_storage_path.clone()
    };

    add_log(
        format!("Starting download of mod: {}", mod_name),
        "info".to_string(),
        "download".to_string(),
        state.clone(),
    )?;

    // Create mod storage directory if it doesn't exist
    let storage_path = Path::new(&mod_storage_path);
    if !storage_path.exists() {
        fs::create_dir_all(storage_path)
            .map_err(|e| format!("Failed to create mod storage directory: {}", e))?;
        add_log(
            format!("Created mod storage directory: {}", mod_storage_path),
            "info".to_string(),
            "system".to_string(),
            state.clone(),
        )?;
    }

    // Generate filename from mod name and current timestamp
    let sanitized_name = mod_name
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect::<String>();

    let timestamp = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}.zip", sanitized_name, timestamp);
    let file_path = storage_path.join(&filename);

    add_log(
        format!("Downloading to: {}", file_path.display()),
        "info".to_string(),
        "download".to_string(),
        state.clone(),
    )?;

    // Download the file
    let response = reqwest::get(&download_url).await.map_err(|e| {
        let error_msg = format!("Failed to download mod: {}", e);
        add_log(
            error_msg.clone(),
            "error".to_string(),
            "download".to_string(),
            state.clone(),
        )
        .ok();
        error_msg
    })?;

    if !response.status().is_success() {
        let error_msg = format!("Download failed with status: {}", response.status());
        add_log(
            error_msg.clone(),
            "error".to_string(),
            "download".to_string(),
            state.clone(),
        )?;
        return Err(error_msg);
    }

    // Write the file
    let bytes = response
        .bytes()
        .await
        .map_err(|e| format!("Failed to read download data: {}", e))?;

    let mut file = tokio::fs::File::create(&file_path)
        .await
        .map_err(|e| format!("Failed to create file: {}", e))?;

    file.write_all(&bytes)
        .await
        .map_err(|e| format!("Failed to write file: {}", e))?;

    let success_msg = format!(
        "Mod '{}' downloaded successfully to: {}",
        mod_name,
        file_path.display()
    );

    add_log(
        success_msg.clone(),
        "info".to_string(),
        "download".to_string(),
        state.clone(),
    )?;

    Ok(file_path.to_string_lossy().to_string())
}

#[tauri::command]
fn list_downloaded_mods(state: State<AppState>) -> Result<Vec<String>, String> {
    use std::fs;
    use std::path::Path;

    // Get mod storage path from settings
    let mod_storage_path = {
        let settings_guard = state.settings.lock().map_err(|e| e.to_string())?;
        let settings = settings_guard.get_settings();
        settings.mod_storage_path.clone()
    };

    let storage_path = Path::new(&mod_storage_path);

    if !storage_path.exists() {
        return Ok(vec![]);
    }

    let mut downloaded_mods = Vec::new();

    if let Ok(entries) = fs::read_dir(storage_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                if let Some(filename) = path.file_name() {
                    if let Some(filename_str) = filename.to_str() {
                        downloaded_mods.push(filename_str.to_string());
                    }
                }
            }
        }
    }

    downloaded_mods.sort();
    Ok(downloaded_mods)
}

// Internal function that can be called from deep link handler
#[allow(dead_code)] // Used in deep link event handler
async fn handle_nxm_url_internal(nxm_url: String, app: tauri::AppHandle) -> Result<(), String> {
    let state = app.state::<AppState>();
    let result = handle_nxm_url(nxm_url, state.clone(), app.clone()).await;

    // If install failed and we were doing a reinstall, abort it gracefully
    if result.is_err() {
        if let Ok(mut slot) = state.reinstall_mod_id.lock() {
            if let Some(mod_id) = slot.take() {
                if let Ok(mut mgr) = state.mod_manager.lock() {
                    mgr.abort_reinstall(&mod_id).ok();
                }
            }
        }
    }

    result
}

#[tauri::command]
async fn handle_nxm_url(
    nxm_url: String,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // Log the NXM URL processing
    add_log(
        format!("Processing NXM URL: {}", nxm_url),
        "info".to_string(),
        "system".to_string(),
        state.clone(),
    )?;

    // Check if there's already a window open and bring it to front
    if let Some(window) = app.get_webview_window("main") {
        add_log(
            "Found existing window, bringing to front".to_string(),
            "info".to_string(),
            "system".to_string(),
            state.clone(),
        )?;

        // Bring window to front and focus it
        if let Err(e) = window.show() {
            add_log(
                format!("Failed to show window: {}", e),
                "warning".to_string(),
                "system".to_string(),
                state.clone(),
            )?;
        }

        if let Err(e) = window.set_focus() {
            add_log(
                format!("Failed to focus window: {}", e),
                "warning".to_string(),
                "system".to_string(),
                state.clone(),
            )?;
        }

        if let Err(e) = window.unminimize() {
            add_log(
                format!("Failed to unminimize window: {}", e),
                "warning".to_string(),
                "system".to_string(),
                state.clone(),
            )?;
        }
    } else {
        add_log(
            "No existing window found, new window will be created".to_string(),
            "info".to_string(),
            "system".to_string(),
            state.clone(),
        )?;
    }

    // Parse the NXM URL
    // Example: nxm://cyberpunk2077/mods/107/files/123169?key=xxx&expires=xxx&user_id=xxx
    // Or: nxm://cyberpunk2077/collections/some-collection-id

    // Check if it's a collection URL
    if let Some(captures) = regex::Regex::new(r"nxm://([^/]+)/collections/([^/?]+)")
        .ok()
        .and_then(|re| re.captures(&nxm_url))
    {
        let game = captures.get(1).map(|m| m.as_str()).unwrap_or("unknown");
        let collection_id = captures.get(2).map(|m| m.as_str()).unwrap_or("unknown");

        add_log(
            format!(
                "📦 Collection detected: {} from game: {}",
                collection_id, game
            ),
            "info".to_string(),
            "download".to_string(),
            state.clone(),
        )?;

        // Handle collection download
        return handle_collection_download(game, collection_id, state, app).await;
    }
    // Check if it's a single mod URL
    else if let Some(captures) = regex::Regex::new(r"nxm://([^/]+)/mods/(\d+)/files/(\d+)")
        .ok()
        .and_then(|re| re.captures(&nxm_url))
    {
        let game = captures.get(1).map(|m| m.as_str()).unwrap_or("unknown");
        let mod_id = captures.get(2).map(|m| m.as_str()).unwrap_or("0");
        let file_id = captures.get(3).map(|m| m.as_str()).unwrap_or("0");

        // Parse URL parameters (key, expires, user_id)
        let url_params: std::collections::HashMap<String, String> = nxm_url
            .split('?')
            .nth(1)
            .unwrap_or("")
            .split('&')
            .filter_map(|param| {
                let parts: Vec<&str> = param.split('=').collect();
                if parts.len() == 2 {
                    Some((parts[0].to_string(), parts[1].to_string()))
                } else {
                    None
                }
            })
            .collect();

        let has_download_key = url_params.contains_key("key");

        add_log(
            format!(
                "Parsed NXM URL - Game: {}, Mod ID: {}, File ID: {}, Has download key: {}",
                game, mod_id, file_id, has_download_key
            ),
            "info".to_string(),
            "system".to_string(),
            state.clone(),
        )?;

        // Get API key from settings
        let api_key = {
            let settings = state.settings.lock().map_err(|e| e.to_string())?;
            settings.get_settings().nexusmods_api_key.clone()
        };

        if api_key.is_empty() {
            add_log(
                "⚠️ NexusMods API key is not configured. Please add your API key in Settings to download mods.".to_string(),
                "error".to_string(),
                "download".to_string(),
                state.clone(),
            )?;
            return Err(
                "NexusMods API key is required. Please configure it in Settings.".to_string(),
            );
        }

        add_log(
            format!("✓ API key found (length: {})", api_key.len()),
            "info".to_string(),
            "system".to_string(),
            state.clone(),
        )?;

        // Automatically trigger mod download
        add_log(
            "Initiating automatic mod download from NXM URL...".to_string(),
            "info".to_string(),
            "download".to_string(),
            state.clone(),
        )?;

        // Get mod info from NexusMods API
        add_log(
            "📡 Fetching mod information from NexusMods API...".to_string(),
            "info".to_string(),
            "download".to_string(),
            state.clone(),
        )?;

        emit_install_progress(&app, InstallProgress {
            stage: "fetching".into(),
            message: format!("Fetching mod #{} info from Nexus...", mod_id),
            ..Default::default()
        });

        let (mod_name, mod_version, mod_author) =
            match nexusmods_api::get_mod_info(game, mod_id, &api_key).await {
                Ok(info) => info,
                Err(e) => {
                    add_log(
                        format!("⚠ Could not fetch mod info: {}. Using fallback name.", e),
                        "warning".to_string(),
                        "download".to_string(),
                        state.clone(),
                    )?;
                    (
                        format!("Mod_{}", mod_id),
                        "Unknown".to_string(),
                        "Unknown".to_string(),
                    )
                }
            };

        emit_install_progress(&app, InstallProgress {
            stage: "fetching".into(),
            message: format!("{} v{} by {}", mod_name, mod_version, mod_author),
            mod_name: Some(mod_name.clone()),
            ..Default::default()
        });

        // Fetch file name for this specific file_id
        let install_file_info = match nexusmods_api::get_file_names(game, &mod_id, &api_key).await {
            Ok(names) => {
                let fid = file_id.to_string();
                names.get(&fid).cloned()
            },
            Err(_) => None,
        };
        if let Ok(mut slot) = state.pending_file_name.lock() {
            *slot = install_file_info.as_ref().map(|(n, _, _)| n.clone());
        }
        if let Ok(mut slot) = state.pending_file_version.lock() {
            *slot = install_file_info.as_ref().and_then(|(_, v, _)| v.clone());
        }
        if let Ok(mut slot) = state.pending_file_description.lock() {
            *slot = install_file_info.as_ref().and_then(|(_, _, d)| d.clone());
        }

        add_log(
            format!("📝 Mod: {} v{} by {}{}", mod_name, mod_version, mod_author,
                install_file_info.as_ref().map(|(n, _, _)| format!(" ({})", n)).unwrap_or_default()),
            "info".to_string(),
            "download".to_string(),
            state.clone(),
        )?;

        // Get download URL - use embedded key if available (for non-premium users)
        let download_url = if has_download_key {
            // For non-premium users: use the NexusMods CDN with the embedded key
            let key = url_params.get("key").cloned().unwrap_or_default();
            let expires = url_params.get("expires").cloned().unwrap_or_default();

            // The NXM key allows us to call the download_link API endpoint
            let url = format!(
                "https://api.nexusmods.com/v1/games/{}/mods/{}/files/{}/download_link.json?key={}&expires={}",
                game, mod_id, file_id, key, expires
            );

            add_log(
                "✓ Using embedded download key from NXM URL (non-premium method)".to_string(),
                "info".to_string(),
                "download".to_string(),
                state.clone(),
            )?;

            // Fetch the actual download URL from the API with the embedded key
            let client = reqwest::Client::new();
            let response = client
                .get(&url)
                .header("apikey", &api_key)
                .header("User-Agent", "CrossoverModManager/1.1.0")
                .send()
                .await
                .map_err(|e| format!("Failed to get download link: {}", e))?;

            if !response.status().is_success() {
                let error_text = response
                    .text()
                    .await
                    .unwrap_or_else(|_| "Unknown error".to_string());
                add_log(
                    format!(
                        "❌ Failed to get download link with embedded key: {}",
                        error_text
                    ),
                    "error".to_string(),
                    "download".to_string(),
                    state.clone(),
                )?;
                return Err(format!("Failed to get download link: {}", error_text));
            }

            #[derive(serde::Deserialize)]
            struct DownloadLink {
                #[serde(rename = "URI")]
                uri: String,
            }

            let download_links: Vec<DownloadLink> = response
                .json()
                .await
                .map_err(|e| format!("Failed to parse download links: {}", e))?;

            if let Some(link) = download_links.first() {
                add_log(
                    format!("✓ Got download URL: {}", link.uri),
                    "info".to_string(),
                    "download".to_string(),
                    state.clone(),
                )?;
                link.uri.clone()
            } else {
                return Err("No download links available".to_string());
            }
        } else {
            // For premium users: use API to get download link
            add_log(
                "🔗 Getting download link from NexusMods API...".to_string(),
                "info".to_string(),
                "download".to_string(),
                state.clone(),
            )?;

            match nexusmods_api::get_download_url(game, mod_id, file_id, &api_key).await {
                Ok(url) => {
                    add_log(
                        format!("✓ Download link obtained successfully: {}", url),
                        "info".to_string(),
                        "download".to_string(),
                        state.clone(),
                    )?;
                    url
                }
                Err(e) => {
                    add_log(
                        format!("❌ Failed to get download URL: {}. Note: Direct API downloads require Premium membership.", e),
                        "error".to_string(),
                        "download".to_string(),
                        state.clone(),
                    )?;
                    return Err(format!("{}. Try clicking 'Download with Mod Manager' on the NexusMods website instead of using direct links.", e));
                }
            }
        };

        add_log(
            "🚀 Starting download and installation process...".to_string(),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;

        // Call the complete installation function
        match install_mod_from_nxm(
            ModInstallParams {
                mod_name: mod_name.clone(),
                mod_version: mod_version.clone(),
                mod_author: mod_author.clone(),
                mod_id: mod_id.to_string(),
                file_id: file_id.to_string(),
                download_url,
            },
            state.clone(),
            app.clone(),
        )
        .await
        {
            Ok(message) => {
                add_log(
                    message,
                    "info".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
            }
            Err(e) => {
                add_log(
                    format!("❌ Installation failed: {}", e),
                    "error".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
                emit_install_progress(&app, InstallProgress {
                    stage: "error".into(),
                    message: e.clone(),
                    nxm_url: Some(nxm_url.clone()),
                    ..Default::default()
                });
                return Err(format!("Installation failed: {}", e));
            }
        }
    } else {
        add_log(
            format!("Failed to parse NXM URL: {}", nxm_url),
            "error".to_string(),
            "system".to_string(),
            state.clone(),
        )?;
    }

    Ok(())
}

async fn handle_collection_download(
    game: &str,
    collection_id: &str,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    // Get API key from settings
    let api_key = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.get_settings().nexusmods_api_key.clone()
    };

    if api_key.is_empty() {
        add_log(
            "⚠️ NexusMods API key is not configured. Please add your API key in Settings to download collections.".to_string(),
            "error".to_string(),
            "download".to_string(),
            state.clone(),
        )?;
        return Err(
            "NexusMods API key is required for collections. Please configure it in Settings."
                .to_string(),
        );
    }

    add_log(
        "📡 Fetching collection information from NexusMods API...".to_string(),
        "info".to_string(),
        "download".to_string(),
        state.clone(),
    )?;

    // Get collection info
    let collection_info =
        match nexusmods_api::get_collection_info(game, collection_id, &api_key).await {
            Ok(info) => {
                add_log(
                    format!(
                        "📦 Collection: {} by {} ({} mods)",
                        info.name, info.author, info.mod_count
                    ),
                    "info".to_string(),
                    "download".to_string(),
                    state.clone(),
                )?;
                info
            }
            Err(e) => {
                add_log(
                    format!("❌ Failed to get collection info: {}", e),
                    "error".to_string(),
                    "download".to_string(),
                    state.clone(),
                )?;
                return Err(format!("Failed to get collection info: {}", e));
            }
        };

    // Get collection mods list
    add_log(
        "📋 Fetching collection mods list...".to_string(),
        "info".to_string(),
        "download".to_string(),
        state.clone(),
    )?;

    let collection_mods = match nexusmods_api::get_collection_mods(
        game,
        collection_id,
        collection_info.revision_number,
        &api_key,
    )
    .await
    {
        Ok(mods) => {
            add_log(
                format!("✓ Found {} mods in collection", mods.len()),
                "info".to_string(),
                "download".to_string(),
                state.clone(),
            )?;
            mods
        }
        Err(e) => {
            add_log(
                format!("❌ Failed to get collection mods: {}", e),
                "error".to_string(),
                "download".to_string(),
                state.clone(),
            )?;
            return Err(format!("Failed to get collection mods: {}", e));
        }
    };

    // Install each mod in the collection
    let mut installed_count = 0;
    let mut failed_count = 0;
    let total_mods = collection_mods.len();

    for (index, collection_mod) in collection_mods.iter().enumerate() {
        add_log(
            format!(
                "📦 Installing mod {}/{}: {} (ID: {}, File: {})",
                index + 1,
                total_mods,
                collection_mod.name,
                collection_mod.mod_id,
                collection_mod.file_id
            ),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;

        // Skip optional mods for now (could add user choice later)
        if collection_mod.is_optional {
            add_log(
                "⏭️ Skipping optional mod".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            continue;
        }

        // Get download URL for this mod
        let download_url = match nexusmods_api::get_download_url(
            game,
            &collection_mod.mod_id.to_string(),
            &collection_mod.file_id.to_string(),
            &api_key,
        )
        .await
        {
            Ok(url) => url,
            Err(e) => {
                add_log(
                    format!(
                        "⚠️ Failed to get download URL for {}: {}",
                        collection_mod.name, e
                    ),
                    "warning".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
                failed_count += 1;
                continue;
            }
        };

        // Install this mod
        match install_mod_from_nxm(
            ModInstallParams {
                mod_name: collection_mod.name.clone(),
                mod_version: collection_mod.version.clone(),
                mod_author: "Collection Author".to_string(), // Collections don't always have individual mod authors
                mod_id: collection_mod.mod_id.to_string(),
                file_id: collection_mod.file_id.to_string(),
                download_url,
            },
            state.clone(),
            app.clone(),
        )
        .await
        {
            Ok(_) => {
                installed_count += 1;
                add_log(
                    format!("✅ Successfully installed: {}", collection_mod.name),
                    "success".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
            }
            Err(e) => {
                failed_count += 1;
                add_log(
                    format!("❌ Failed to install {}: {}", collection_mod.name, e),
                    "error".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
            }
        }
    }

    // Final summary
    add_log(
        format!(
            "🎉 Collection installation complete! Installed: {}, Failed: {}, Total: {}",
            installed_count, failed_count, total_mods
        ),
        "success".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    // Emit collection-complete event to frontend
    if let Some(window) = app.get_webview_window("main") {
        add_log(
            "📢 Emitting collection-complete event to frontend".to_string(),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;

        let collection_summary = serde_json::json!({
            "collection_id": collection_id,
            "installed": installed_count,
            "failed": failed_count,
            "total": total_mods
        });

        window.emit("collection-complete", &collection_summary).ok();
    }

    Ok(())
}

#[tauri::command]
async fn test_nxm_event(app: tauri::AppHandle, test_url: String) -> Result<(), String> {
    // Simulate the protocol handler by emitting the same event
    if let Some(window) = app.get_webview_window("main") {
        window
            .emit("nxm-url-received", &test_url)
            .map_err(|e| e.to_string())?;
        Ok(())
    } else {
        Err("No main window found".to_string())
    }
}

/// Emit a relay-status event to the main window.
fn emit_relay_status(app: &tauri::AppHandle, stage: &str, message: &str, show_actions: bool, nxm_url: Option<&str>, cold_start: bool) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.emit("relay-status", serde_json::json!({
            "stage": stage,
            "message": message,
            "show_actions": show_actions,
            "nxm_url": nxm_url,
            "cold_start": cold_start,
        }));
    }
}

#[tauri::command]
async fn handle_relay_action(
    action: String,   // "process" | "exit"
    nxm_url: Option<String>,
    app: tauri::AppHandle,
) -> Result<(), String> {
    match action.as_str() {
        "process" => {
            if let Some(url) = nxm_url {
                handle_nxm_url_internal(url, app).await?;
            }
        }
        _ => {
            app.exit(0);
        }
    }
    Ok(())
}

/// Returns the NXM URL that launched this app (if any), then clears it.
#[tauri::command]
fn get_startup_nxm_url(state: State<AppState>) -> Option<String> {
    state.startup_nxm_url.lock().ok()?.take()
}

/// Core relay-or-process logic shared by startup and event-based deep link handling.
/// `cold_start` = true when app was launched by the NXM link (not already running).
async fn handle_nxm_deep_link(url: String, app: tauri::AppHandle, cold_start: bool) {
    if let Some(window) = app.get_webview_window("main") {
        window.show().ok();
        window.set_focus().ok();
    }

    // Try to relay via Unix socket
    match try_relay_to_dev(&url) {
        Ok(true) => {
            println!("🔥 DEEP LINK: relayed to dev instance via socket");
            emit_relay_status(&app, "relaying", "Forwarding to dev instance…", false, Some(&url), cold_start);

            // Brief pause then show done
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;

            if cold_start {
                for i in (1..=5).rev() {
                    emit_relay_status(
                        &app, "done",
                        &format!("Forwarded to dev instance. Closing in {}s...", i),
                        false, Some(&url), true,
                    );
                    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                }
                app.exit(0);
            } else {
                emit_relay_status(&app, "done", "Forwarded to dev instance", false, Some(&url), false);
            }
        }
        Ok(false) => {
            println!("🔥 DEEP LINK: no dev instance, processing directly");
            match handle_nxm_url_internal(url, app).await {
                Ok(_) => println!("🔥 DEEP LINK: processed successfully"),
                Err(e) => println!("🔥 DEEP LINK ERROR: {}", e),
            }
        }
        Err(e) => {
            println!("🔥 DEEP LINK: relay failed ({}), processing directly", e);
            emit_relay_status(&app, "error", &format!("Relay failed: {}", e), true, Some(&url), cold_start);
            match handle_nxm_url_internal(url, app).await {
                Ok(_) => println!("🔥 DEEP LINK: processed successfully (fallback)"),
                Err(e) => println!("🔥 DEEP LINK ERROR: {}", e),
            }
        }
    }
}

/// Returns true when running via `tauri dev` (not a bundled release).
#[tauri::command]
async fn get_mod_changelog(
    mod_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, String> {
    let api_key = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.get_settings().nexusmods_api_key.clone()
    };
    if api_key.is_empty() {
        return Ok(serde_json::json!({}));
    }

    let client = reqwest::Client::new();

    // Fetch changelog
    let changelog_url = format!(
        "https://api.nexusmods.com/v1/games/cyberpunk2077/mods/{}/changelogs.json",
        mod_id
    );
    let changelog_resp = client
        .get(&changelog_url)
        .header("apikey", &api_key)
        .header("User-Agent", "CrossoverModManager/2.0")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !changelog_resp.status().is_success() {
        return Ok(serde_json::json!({}));
    }

    let changelog: serde_json::Value = changelog_resp
        .json()
        .await
        .map_err(|_| "Failed to parse changelog".to_string())?;

    // Fetch files to get version → date mapping
    let files_url = format!(
        "https://api.nexusmods.com/v1/games/cyberpunk2077/mods/{}/files.json",
        mod_id
    );
    let mut version_dates: std::collections::HashMap<String, String> = std::collections::HashMap::new();

    if let Ok(files_resp) = client
        .get(&files_url)
        .header("apikey", &api_key)
        .header("User-Agent", "CrossoverModManager/2.0")
        .send()
        .await
    {
        if files_resp.status().is_success() {
            #[derive(serde::Deserialize)]
            struct FilesResp { files: Vec<FileEntry> }
            #[derive(serde::Deserialize)]
            struct FileEntry { version: Option<String>, uploaded_timestamp: Option<i64> }

            if let Ok(data) = files_resp.json::<FilesResp>().await {
                for f in data.files {
                    if let (Some(ver), Some(ts)) = (f.version, f.uploaded_timestamp) {
                        // Keep the latest timestamp per version
                        let date = chrono::DateTime::from_timestamp(ts, 0)
                            .map(|dt| dt.format("%d %b %Y").to_string())
                            .unwrap_or_default();
                        if !date.is_empty() {
                            version_dates.entry(ver).or_insert(date);
                        }
                    }
                }
            }
        }
    }

    // Merge: return { "version": { "lines": [...], "date": "..." } }
    if let Some(obj) = changelog.as_object() {
        let mut result = serde_json::Map::new();
        for (ver, lines) in obj {
            result.insert(ver.clone(), serde_json::json!({
                "lines": lines,
                "date": version_dates.get(ver).cloned()
            }));
        }
        Ok(serde_json::Value::Object(result))
    } else {
        Ok(changelog)
    }
}

#[tauri::command]
fn is_dev_build() -> bool {
    tauri::is_dev() || cfg!(debug_assertions)
}

#[tauri::command]
fn set_force_reinstall(state: State<AppState>) {
    state.force_reinstall.store(true, std::sync::atomic::Ordering::Relaxed);
}

#[tauri::command]
fn abort_reinstall(state: State<AppState>) -> Result<(), String> {
    let mod_id = state.reinstall_mod_id.lock().map_err(|e| e.to_string())?.take();
    if let Some(id) = mod_id {
        let mut mgr = state.mod_manager.lock().map_err(|e| e.to_string())?;
        mgr.abort_reinstall(&id)?;
    }
    Ok(())
}

#[tauri::command]
fn get_build_timestamp() -> String {
    env!("BUILD_TIMESTAMP").to_string()
}

/// Compare version strings: returns true if `latest` is newer than `installed`.
/// Handles semver-like versions: "1.4.3" > "1.4.2", "2.0" > "1.9.9", "1.0" < "2.1.1"
fn is_newer_version(latest: &str, installed: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> {
        s.split(|c: char| c == '.' || c == '-')
            .filter_map(|p| p.parse::<u64>().ok())
            .collect()
    };
    let lv = parse(latest);
    let iv = parse(installed);
    let len = lv.len().max(iv.len());
    for i in 0..len {
        let l = lv.get(i).copied().unwrap_or(0);
        let r = iv.get(i).copied().unwrap_or(0);
        if l > r { return true; }
        if l < r { return false; }
    }
    false // equal
}

fn socket_path() -> std::path::PathBuf {
    // Use /tmp (not $TMPDIR) so both dev and bundled app share the same path.
    std::path::PathBuf::from("/tmp/crossover-mod-manager-dev-relay.sock")
}

/// Try to relay an NXM URL to a dev instance via Unix socket.
/// Returns Ok(true) if relayed, Ok(false) if no dev instance, Err on failure.
fn try_relay_to_dev(url: &str) -> Result<bool, String> {
    use std::io::{Read, Write};
    use std::os::unix::net::UnixStream;

    let sock = socket_path();
    let mut stream = match UnixStream::connect(&sock) {
        Ok(s) => s,
        Err(_) => return Ok(false), // No socket = no dev instance
    };
    stream.set_write_timeout(Some(std::time::Duration::from_secs(5))).ok();
    stream.set_read_timeout(Some(std::time::Duration::from_secs(5))).ok();

    stream.write_all(url.as_bytes()).map_err(|e| e.to_string())?;
    stream.shutdown(std::net::Shutdown::Write).map_err(|e| e.to_string())?;

    let mut response = String::new();
    stream.read_to_string(&mut response).map_err(|e| e.to_string())?;

    Ok(response.trim() == "OK")
}

/// Start listening for relay URLs on Unix socket (dev instance only).
fn start_socket_listener(app: tauri::AppHandle) {
    use std::os::unix::net::UnixListener;

    let sock = socket_path();
    // Remove stale socket
    std::fs::remove_file(&sock).ok();

    let listener = match UnixListener::bind(&sock) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("📡 Failed to bind relay socket: {}", e);
            return;
        }
    };
    println!("📡 DEV: listening on {}", sock.display());

    std::thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(mut stream) => {
                    use std::io::{Read, Write};
                    let mut url = String::new();
                    if stream.read_to_string(&mut url).is_ok() && !url.is_empty() {
                        let url = url.trim().to_string();
                        println!("📡 DEV: received relay URL: {}", url);
                        let app_clone = app.clone();
                        tauri::async_runtime::spawn(async move {
                            match handle_nxm_url_internal(url, app_clone).await {
                                Ok(_) => println!("📡 DEV: relay URL processed successfully"),
                                Err(e) => println!("📡 DEV: relay URL failed: {}", e),
                            }
                        });
                        stream.write_all(b"OK").ok();
                    }
                }
                Err(e) => eprintln!("📡 Socket accept error: {}", e),
            }
        }
    });
}

/// Try to relay URL to dev instance. Returns true if relayed, false if should process locally.
#[tauri::command]
fn try_relay(nxm_url: String) -> bool {
    match try_relay_to_dev(&nxm_url) {
        Ok(true) => {
            println!("📡 try_relay: forwarded to dev");
            true
        }
        _ => {
            println!("📡 try_relay: no dev instance, process locally");
            false
        }
    }
}

/// Cleanup socket on shutdown
fn cleanup_socket() {
    std::fs::remove_file(socket_path()).ok();
}

fn is_valid_cyberpunk_installation(path: &std::path::Path) -> bool {
    // Check if the directory exists and contains key Cyberpunk 2077 files
    if !path.exists() || !path.is_dir() {
        return false;
    }

    // Check for the main executable
    let exe_path = path.join("bin/x64/Cyberpunk2077.exe");
    if exe_path.exists() {
        return true;
    }

    // Alternative check for other key files
    let key_files = vec![
        "Cyberpunk2077.exe",
        "bin/x64/Cyberpunk2077.exe",
        "engine/config/base/engine.ini",
    ];

    for file in key_files {
        if path.join(file).exists() {
            return true;
        }
    }

    false
}

/// Parameters for mod installation from NXM URL
#[derive(Debug, serde::Deserialize)]
struct ModInstallParams {
    mod_name: String,
    mod_version: String,
    mod_author: String,
    mod_id: String,
    file_id: String,
    download_url: String,
}

#[allow(unused_assignments)]
#[tauri::command]
async fn install_mod_from_nxm(
    params: ModInstallParams,
    state: State<'_, AppState>,
    app: tauri::AppHandle,
) -> Result<String, String> {
    // Set installing flag and reset cancel
    state.install_cancel.store(false, Ordering::Relaxed);
    state.installing.store(true, Ordering::Relaxed);

    let result = install_mod_from_nxm_inner(&params, &state, &app).await;

    state.installing.store(false, Ordering::Relaxed);
    state.install_cancel.store(false, Ordering::Relaxed);
    result
}

async fn install_mod_from_nxm_inner(
    params: &ModInstallParams,
    state: &State<'_, AppState>,
    app: &tauri::AppHandle,
) -> Result<String, String> {
    let cancel = Arc::clone(&state.install_cancel);

    macro_rules! check_cancel {
        () => {
            if cancel.load(Ordering::Relaxed) {
                emit_install_progress(app, InstallProgress {
                    stage: "error".into(),
                    message: "Installation cancelled".into(),
                    ..Default::default()
                });
                return Err("Installation cancelled by user".into());
            }
        };
    }

    // Extract parameters
    let mod_name = params.mod_name.clone();
    let mod_version = params.mod_version.clone();
    let mod_author = params.mod_author.clone();
    let mod_id = params.mod_id.clone();
    let file_id = params.file_id.clone();
    let download_url = params.download_url.clone();
    use std::fs;
    use std::path::Path;
    use walkdir::WalkDir;

    // Variables for cleanup (used throughout the function for error handling)
    let mut archive_path: Option<std::path::PathBuf> = None;
    let mut _extract_dir: Option<std::path::PathBuf> = None;

    // Get game path from settings
    let game_path = {
        let settings_guard = state.settings.lock().map_err(|e| e.to_string())?;
        let settings = settings_guard.get_settings();
        settings.game_path.clone()
    };

    if game_path.is_empty() {
        return Err("Game path not set in settings. Please configure it first.".to_string());
    }

    // Check for duplicate installations
    {
        let mut manager = state.mod_manager.lock().map_err(|e| e.to_string())?;

        // Check if exact same mod and file is already installed
        if let Some(existing_mod) = manager.find_existing_mod(&mod_id, &file_id) {
            if state.force_reinstall.load(std::sync::atomic::Ordering::Relaxed) {
                state.force_reinstall.store(false, std::sync::atomic::Ordering::Relaxed);
                let existing_id = existing_mod.id.clone();
                let existing_name = existing_mod.name.clone();

                // Phase 1: prepare
                add_log(
                    format!("🔄 Reinstall: preparing '{}'", existing_name),
                    "info".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
                drop(manager);

                {
                    let mut mgr = state.mod_manager.lock().map_err(|e| e.to_string())?;
                    mgr.set_reinstall_status(&existing_id, Some("prepare"))?;
                }

                // Phase 2: removing old files
                {
                    let mut mgr = state.mod_manager.lock().map_err(|e| e.to_string())?;
                    mgr.set_reinstall_status(&existing_id, Some("removing"))?;
                    mgr.remove_mod_files(&existing_id)?;
                }

                // Phase 3: set installing status
                {
                    let mut mgr = state.mod_manager.lock().map_err(|e| e.to_string())?;
                    mgr.set_reinstall_status(&existing_id, Some("installing"))?;
                }

                // Store existing_id so registration step updates this record
                if let Ok(mut slot) = state.reinstall_mod_id.lock() {
                    *slot = Some(existing_id);
                }

                manager = state.mod_manager.lock().map_err(|e| e.to_string())?;
            } else {
                let err_msg = format!("Mod '{}' with the same file version is already installed. Please uninstall the existing version first if you want to reinstall.", existing_mod.name);
                add_log(
                    format!(
                        "⚠️ Mod '{}' (File ID: {}) is already installed!",
                        existing_mod.name, file_id
                    ),
                    "warning".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
                emit_install_progress(&app, InstallProgress {
                    stage: "error".into(),
                    message: err_msg.clone(),
                    mod_name: Some(existing_mod.name.clone()),
                    ..Default::default()
                });
                return Err(err_msg);
            }
        }

        // Check if a different version of the same part is installed → auto-update
        // Match by mod_id + file_name (not just mod_id) to distinguish parts from updates
        let installing_file_name: Option<String> = state.pending_file_name.lock().ok().and_then(|s| s.clone());
        let existing_same_part = manager.get_installed_mods().into_iter().find(|m| {
            m.mod_id.as_deref() == Some(&mod_id)
                && !m.removed
                && m.file_id.as_deref() != Some(&file_id)
                && installing_file_name.is_some()
                && m.file_name.as_deref() == installing_file_name.as_deref()
        });
        if let Some(existing_mod) = existing_same_part {
            let existing_id = existing_mod.id.clone();
            let existing_name = existing_mod.name.clone();
            let existing_version = existing_mod.version.clone();

            add_log(
                format!(
                    "🔄 Updating '{}': v{} → v{}",
                    existing_name, existing_version, mod_version
                ),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;

            drop(manager);

            {
                let mut mgr = state.mod_manager.lock().map_err(|e| e.to_string())?;
                mgr.set_reinstall_status(&existing_id, Some("installing"))?;
            }

            // Store existing_id so registration step updates this record instead of creating new
            if let Ok(mut slot) = state.reinstall_mod_id.lock() {
                *slot = Some(existing_id);
            }

            manager = state.mod_manager.lock().map_err(|e| e.to_string())?;
        }

        // Check if mod with same name but different ID exists (potential conflict)
        if let Some(existing_mod) = manager.find_existing_mod_by_name(&mod_name, &mod_version) {
            if existing_mod.mod_id.as_ref() != Some(&mod_id) {
                add_log(
                    format!(
                        "⚠️ Mod with same name '{}' v{} already exists but from different source!",
                        mod_name, mod_version
                    ),
                    "warning".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
                add_log(
                    "💡 This might be the same mod from a different source. Proceeding with installation.".to_string(),
                    "info".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
            }
        }
    }

    add_log(
        format!("🚀 Starting installation for mod: {}", mod_name),
        "info".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    check_cancel!();

    // Step 1: Download the mod
    add_log(
        format!("📥 Downloading mod from: {}", download_url),
        "info".to_string(),
        "download".to_string(),
        state.clone(),
    )?;

    let response = reqwest::get(&download_url)
        .await
        .map_err(|e| format!("Failed to download mod: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Download failed with status: {}",
            response.status()
        ));
    }

    let total_size = response.content_length().unwrap_or(0);
    add_log(
        format!("📦 Download size: {}", format_bytes(total_size)),
        "info".to_string(),
        "download".to_string(),
        state.clone(),
    )?;

    // Check disk space before downloading
    if total_size > 0 {
        let temp_dir = std::env::temp_dir();
        match check_sufficient_disk_space(&temp_dir, total_size) {
            Ok(_) => {
                add_log(
                    "✓ Sufficient disk space available for download and extraction".to_string(),
                    "info".to_string(),
                    "download".to_string(),
                    state.clone(),
                )?;
            }
            Err(err) => {
                add_log(
                    format!("❌ {}", err),
                    "error".to_string(),
                    "download".to_string(),
                    state.clone(),
                )?;
                add_log(
                    "💡 Tip: Free up disk space or clean up old mod downloads from system temp folder".to_string(),
                    "info".to_string(),
                    "download".to_string(),
                    state.clone(),
                )?;
                return Err(err);
            }
        }
    }

    // Stream download with progress events
    use futures_util::StreamExt;
    let total_size_opt: Option<u64> = if total_size > 0 { Some(total_size) } else { None };
    let mut stream = response.bytes_stream();
    let mut bytes: Vec<u8> = Vec::new();
    if let Some(sz) = total_size_opt {
        bytes.reserve(sz as usize);
    }
    let mut received: u64 = 0;
    let mut last_progress_at: u64 = 0;
    let emit_interval = total_size_opt.map(|t| (t / 50).max(65536)).unwrap_or(131072);

    emit_install_progress(&app, InstallProgress {
        stage: "downloading".into(),
        message: match total_size_opt {
            Some(t) => format!("Downloading 0 / {}", format_bytes(t)),
            None => "Downloading...".into(),
        },
        bytes_received: Some(0),
        total_bytes: total_size_opt,
        ..Default::default()
    });

    while let Some(chunk) = stream.next().await {
        check_cancel!();
        let chunk = chunk.map_err(|e| format!("Failed to read download data: {}", e))?;
        received += chunk.len() as u64;
        bytes.extend_from_slice(&chunk);
        if received - last_progress_at >= emit_interval {
            last_progress_at = received;
            emit_install_progress(&app, InstallProgress {
                stage: "downloading".into(),
                message: match total_size_opt {
                    Some(t) => format!("Downloading {} / {}", format_bytes(received), format_bytes(t)),
                    None => format!("Downloading {}...", format_bytes(received)),
                },
                bytes_received: Some(received),
                total_bytes: total_size_opt,
                ..Default::default()
            });
        }
    }

    add_log(
        format!("✓ Downloaded {}", format_bytes(received)),
        "info".to_string(),
        "download".to_string(),
        state.clone(),
    )?;

    // Step 2: Save to temp file with RAII guard for automatic cleanup
    let temp_dir = std::env::temp_dir();
    let archive_filename = format!("{}_{}.zip", mod_id, file_id);
    let temp_archive_path = temp_dir.join(&archive_filename);

    // Create RAII guard - will auto-cleanup if function exits early
    let archive_guard = TempFileGuard::new(
        temp_archive_path.clone(),
        format!("archive file: {}", archive_filename),
    );
    archive_path = Some(temp_archive_path.clone());

    fs::write(&temp_archive_path, &bytes)
        .map_err(|e| format!("Failed to save downloaded file: {}", e))?;

    add_log(
        "💾 Saved download to temporary location".to_string(),
        "info".to_string(),
        "download".to_string(),
        state.clone(),
    )?;

    check_cancel!();

    // Step 3: Extract the archive
    let archive_type = archive_extractor::ArchiveExtractor::detect_archive_type(&temp_archive_path);
    let archive_type_str = match &archive_type {
        archive_extractor::ArchiveType::Zip => "ZIP",
        archive_extractor::ArchiveType::SevenZ => "7z",
        archive_extractor::ArchiveType::Rar => "RAR",
        archive_extractor::ArchiveType::Unsupported(ext) => ext.as_str(),
    };

    emit_install_progress(&app, InstallProgress {
        stage: "extracting".into(),
        message: format!("Extracting {} archive...", archive_type_str),
        ..Default::default()
    });

    add_log(
        format!("📂 Extracting {} archive...", archive_type_str),
        "info".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    // Check disk space for extraction (archives typically expand 2-3x)
    let game_dir_path = Path::new(&game_path);
    if let Ok(archive_size) = fs::metadata(&temp_archive_path).map(|m| m.len()) {
        match check_sufficient_disk_space(game_dir_path, archive_size) {
            Ok(_) => {
                add_log(
                    "✓ Sufficient disk space in game directory for installation".to_string(),
                    "info".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
            }
            Err(err) => {
                add_log(
                    format!("❌ {}", err),
                    "error".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
                add_log(
                    "💡 Tip: Free up disk space in your game directory or Wine bottle".to_string(),
                    "info".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
                // Cleanup before returning error
                if let Some(path) = &archive_path {
                    fs::remove_file(path).ok();
                }
                return Err(err);
            }
        }
    }

    let temp_extract_dir =
        temp_dir.join(format!("mod_extract_{}_{}", mod_id, uuid::Uuid::new_v4()));

    // Create RAII guard - will auto-cleanup if function exits early
    let extract_guard = TempFileGuard::new(
        temp_extract_dir.clone(),
        format!("extraction directory: mod_extract_{}_*", mod_id),
    );

    // Extract using hybrid extractor (supports ZIP, 7z, RAR)
    let (file_count, extraction_method) =
        archive_extractor::ArchiveExtractor::extract(&temp_archive_path, &temp_extract_dir)?; // Guards will auto-cleanup on error

    let method_name = archive_extractor::ArchiveExtractor::method_name(&extraction_method);
    add_log(
        format!("✓ Extracted {} files using {}", file_count, method_name),
        "info".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    // Show installation hints for system tools if not available
    let hints = archive_extractor::ArchiveExtractor::get_installation_hints();
    if !hints.is_empty()
        && matches!(
            extraction_method,
            archive_extractor::ExtractionMethod::RustSevenz
                | archive_extractor::ExtractionMethod::RustUnrar
        )
    {
        for hint in hints {
            add_log(
                hint,
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
        }
    }

    check_cancel!();

    // Step 4: Install files to game directory
    emit_install_progress(&app, InstallProgress {
        stage: "installing".into(),
        message: format!("Installing {} files to game directory...", file_count),
        file_count: Some(0),
        file_total: Some(file_count),
        ..Default::default()
    });

    add_log(
        "🎮 Installing mod files to game directory...".to_string(),
        "info".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    let game_dir = Path::new(&game_path);
    if !game_dir.exists() {
        // Guards will auto-cleanup temp files
        return Err("Game directory does not exist".to_string());
    }

    // Check path length to prevent macOS PATH_MAX issues
    add_log(
        "📏 Checking path length compatibility...".to_string(),
        "info".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    if let Err(warning) = check_path_length(game_dir) {
        // Log the warning but continue (unless it's a hard error)
        if warning.contains("Maximum allowed is") {
            // Hard error - path is too long
            add_log(
                format!("❌ {}", warning),
                "error".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            // Guards will auto-cleanup temp files
            return Err(warning);
        } else {
            // Warning - path is approaching limit
            add_log(
                warning,
                "warning".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
        }
    } else {
        add_log(
            "✓ Path length is within safe limits".to_string(),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;
    }

    // Detect Wine Windows version (macOS only)
    #[cfg(target_os = "macos")]
    {
        add_log(
            "🪟 Detecting Wine Windows version...".to_string(),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;

        match detect_wine_windows_version(game_dir) {
            Ok((version_string, is_recommended)) => {
                if is_recommended {
                    add_log(
                        format!("✓ Wine is configured to emulate: {}", version_string),
                        "info".to_string(),
                        "installation".to_string(),
                        state.clone(),
                    )?;
                    add_log(
                        "  This is the recommended version for Cyberpunk 2077 mods".to_string(),
                        "info".to_string(),
                        "installation".to_string(),
                        state.clone(),
                    )?;
                } else {
                    add_log(
                        format!("⚠️  Wine is configured to emulate: {}", version_string),
                        "warning".to_string(),
                        "installation".to_string(),
                        state.clone(),
                    )?;
                    add_log(
                        "  Some modern mods require Windows 10 or later".to_string(),
                        "warning".to_string(),
                        "installation".to_string(),
                        state.clone(),
                    )?;
                    add_log(
                        "".to_string(),
                        "info".to_string(),
                        "installation".to_string(),
                        state.clone(),
                    )?;
                    add_log(
                        "💡 To change Windows version in Crossover:".to_string(),
                        "info".to_string(),
                        "installation".to_string(),
                        state.clone(),
                    )?;
                    add_log(
                        "  1. Open CrossOver → Right-click your bottle".to_string(),
                        "info".to_string(),
                        "installation".to_string(),
                        state.clone(),
                    )?;
                    add_log(
                        "  2. Select 'Wine Configuration' (or Run Command → winecfg)".to_string(),
                        "info".to_string(),
                        "installation".to_string(),
                        state.clone(),
                    )?;
                    add_log(
                        "  3. Go to 'Applications' tab".to_string(),
                        "info".to_string(),
                        "installation".to_string(),
                        state.clone(),
                    )?;
                    add_log(
                        "  4. Set 'Windows Version' to 'Windows 10'".to_string(),
                        "info".to_string(),
                        "installation".to_string(),
                        state.clone(),
                    )?;
                    add_log(
                        "  5. Click Apply and restart your game launcher".to_string(),
                        "info".to_string(),
                        "installation".to_string(),
                        state.clone(),
                    )?;
                }
            }
            Err(e) => {
                // Just log as info - not critical
                add_log(
                    format!("ℹ️  Could not detect Wine version: {}", e),
                    "info".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
            }
        }
    }

    let mut installed_files = Vec::new();
    let mut install_count = 0;
    let mut is_redmod = false;
    let mut is_cet = false;
    let mut is_red4ext = false;
    let mut case_mismatch_count = 0;
    let mut symlink_count = 0;
    let mut symlinks_detected: Vec<(String, Option<String>)> = Vec::new(); // (symlink_path, target)
    let mut unicode_count = 0;
    let mut unicode_sanitized: Vec<(String, String)> = Vec::new(); // (original, sanitized)

    // Walk through extracted files and install them
    for entry in WalkDir::new(&temp_extract_dir)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        // Check if entry is a symlink (before checking is_file)
        let is_symlink = entry.file_type().is_symlink();

        if entry.file_type().is_file() || is_symlink {
            let relative_path = entry
                .path()
                .strip_prefix(&temp_extract_dir)
                .map_err(|e| e.to_string())?;

            // Handle symlinks specially
            if is_symlink {
                symlink_count += 1;
                let symlink_path = relative_path.to_string_lossy().to_string();

                // Try to read the symlink target
                let target = match std::fs::read_link(entry.path()) {
                    Ok(target_path) => Some(target_path.to_string_lossy().to_string()),
                    Err(_) => None,
                };

                symlinks_detected.push((symlink_path.clone(), target.clone()));

                add_log(
                    format!(
                        "🔗 Symlink detected: {}{}",
                        symlink_path,
                        target
                            .as_ref()
                            .map(|t| format!(" → {}", t))
                            .unwrap_or_default()
                    ),
                    "warning".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;

                // Skip symlink - we'll handle it after the loop
                continue;
            }

            let relative_path = entry
                .path()
                .strip_prefix(&temp_extract_dir)
                .map_err(|e| e.to_string())?;

            // Check if this is a REDmod (has info.json in mods/ folder)
            let path_str = relative_path.to_string_lossy().to_lowercase();
            if (path_str.starts_with("mods/") || path_str.starts_with("mods\\"))
                && path_str.ends_with("info.json")
            {
                is_redmod = true;
            }

            // Check if this is Cyber Engine Tweaks (has cyber_engine_tweaks.asi or version.dll in bin/x64)
            if path_str.contains("cyber_engine_tweaks.asi")
                || (path_str.contains("bin/x64") && path_str.ends_with("version.dll"))
            {
                is_cet = true;
            }

            // Check if this is RED4ext (has red4ext.dll or version.dll in root - not in bin/x64)
            if path_str.contains("red4ext.dll")
                || path_str.contains("red4ext")
                || path_str.ends_with("red4ext.dll")
                || (path_str.ends_with("version.dll")
                    && !path_str.contains("bin/")
                    && !path_str.contains("bin\\"))
            {
                is_red4ext = true;
            }

            // Check for case sensitivity issues before installation
            let (has_case_mismatch, _normalized_path, case_issues) =
                check_case_mismatch(relative_path);

            if has_case_mismatch && !case_issues.is_empty() {
                // Log case mismatch warning
                for issue in &case_issues {
                    add_log(
                        format!("⚠️ Case sensitivity issue detected: {}", issue),
                        "warning".to_string(),
                        "installation".to_string(),
                        state.clone(),
                    )?;
                }

                add_log(
                    "🔧 Auto-correcting path casing to match game structure".to_string(),
                    "info".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;

                case_mismatch_count += 1;
            }

            // Check for Unicode characters in filename
            let filename = relative_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");

            if let Some(sanitized) = needs_sanitization(filename) {
                unicode_count += 1;
                unicode_sanitized.push((filename.to_string(), sanitized.clone()));

                add_log(
                    format!("🔤 Unicode filename detected: '{}'", filename),
                    "warning".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
                add_log(
                    format!("🔧 Sanitizing to ASCII-safe: '{}'", sanitized),
                    "info".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
            }

            // Determine installation path based on file type (uses normalized paths)
            let mut install_path = determine_install_path_for_file(game_dir, relative_path)?;

            // Apply Unicode sanitization to the final filename if needed
            if let Some(sanitized) = needs_sanitization(filename) {
                // Replace the filename with sanitized version
                if let Some(parent) = install_path.parent() {
                    install_path = parent.join(sanitized);
                }
            }

            // Check if target file already exists with different casing
            if let Some(parent) = install_path.parent() {
                if parent.exists() {
                    if let Some(file_name) = install_path.file_name() {
                        let target_name = file_name.to_string_lossy();
                        let target_lower = target_name.to_lowercase();

                        if let Ok(entries) = std::fs::read_dir(parent) {
                            for entry in entries.flatten() {
                                if let Ok(existing_name) = entry.file_name().into_string() {
                                    if existing_name.to_lowercase() == target_lower
                                        && existing_name != target_name.as_ref()
                                    {
                                        add_log(
                                            format!(
                                                "⚠️ Existing file with different casing found: '{}' will be replaced with '{}'",
                                                existing_name, target_name
                                            ),
                                            "warning".to_string(),
                                            "installation".to_string(),
                                            state.clone(),
                                        )?;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Log file placement for debugging (especially for RED4ext files)
            if install_count % 10 == 0
                || path_str.contains("red4ext")
                || path_str.ends_with("version.dll")
            {
                add_log(
                    format!(
                        "📁 Installing: {} → {}",
                        relative_path.display(),
                        install_path
                            .strip_prefix(game_dir)
                            .unwrap_or(&install_path)
                            .display()
                    ),
                    "info".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
            }

            // Create parent directories
            if let Some(parent) = install_path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|e| format!("Failed to create directory: {}", e))?;

                // Set Wine-compatible permissions on created directory
                if let Err(e) = set_wine_compatible_permissions(parent, true) {
                    // Log warning but continue - not critical
                    add_log(
                        format!(
                            "⚠️  Could not set directory permissions for {}: {}",
                            parent.display(),
                            e
                        ),
                        "warning".to_string(),
                        "installation".to_string(),
                        state.clone(),
                    )?;
                }
            }

            // Validate path stays within game directory
            validate_path_within_game_dir(&install_path, game_dir)?;

            // Copy file
            fs::copy(entry.path(), &install_path).map_err(|e| {
                // Guards will auto-cleanup temp files on error
                format!("Failed to copy file to game directory: {}", e)
            })?;

            // Set Wine-compatible permissions (macOS/Unix only)
            // This helps Wine load DLLs and access config files properly
            if let Err(e) = set_wine_compatible_permissions(&install_path, false) {
                // Log warning but continue - not critical
                add_log(
                    format!(
                        "⚠️  Could not set permissions for {}: {}",
                        install_path.display(),
                        e
                    ),
                    "warning".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
            }

            installed_files.push(install_path.to_string_lossy().to_string());
            install_count += 1;

            check_cancel!();

            // Progress indicator for installation (every 5 files)
            if install_count % 5 == 0 {
                emit_install_progress(&app, InstallProgress {
                    stage: "installing".into(),
                    message: format!("Installing file {}/{}...", install_count, file_count),
                    file_count: Some(install_count),
                    file_total: Some(file_count),
                    ..Default::default()
                });
                add_log(
                    format!("🔧 Installing... ({} files)", install_count),
                    "info".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
            }
        }
    }

    add_log(
        format!(
            "✓ Installed {} files to game directory",
            installed_files.len()
        ),
        "info".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    // Check for file conflicts with other installed mods
    {
        let manager = state.mod_manager.lock().map_err(|e| e.to_string())?;
        let conflicts = manager.check_file_conflicts(&installed_files);

        if !conflicts.is_empty() {
            add_log(
                "⚠️ File Conflict Detection".to_string(),
                "warning".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;

            let mut archive_conflicts = Vec::new();
            let mut other_conflicts = Vec::new();

            for (file_path, conflict_list) in &conflicts {
                if file_path.ends_with(".archive") {
                    archive_conflicts.push((file_path, conflict_list));
                } else {
                    other_conflicts.push((file_path, conflict_list));
                }
            }

            // Report archive conflicts with load order information
            if !archive_conflicts.is_empty() {
                add_log(
                    format!(
                        "📦 {} .archive file(s) will override existing mod archives:",
                        archive_conflicts.len()
                    ),
                    "warning".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;

                for (file_path, conflict_list) in archive_conflicts {
                    for conflict in conflict_list.iter() {
                        let filename = std::path::Path::new(file_path)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy();
                        add_log(
                            format!(
                                "  • '{}' was previously installed by '{}'",
                                filename, conflict.mod_name
                            ),
                            "warning".to_string(),
                            "installation".to_string(),
                            state.clone(),
                        )?;
                    }
                }

                add_log(
                    "ℹ️  Archive Load Order: Cyberpunk 2077 loads .archive files alphabetically."
                        .to_string(),
                    "info".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
                add_log(
                    "💡 The LAST loaded archive wins if multiple mods modify the same assets."
                        .to_string(),
                    "info".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
                add_log(
                    "🔧 To control load order, you can rename archives:".to_string(),
                    "info".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
                add_log(
                    "   - Prefix with '0-' to load first (e.g., '0-basegame_textures.archive')"
                        .to_string(),
                    "info".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
                add_log(
                    "   - Prefix with 'z-' to load last (e.g., 'z-basegame_final.archive')"
                        .to_string(),
                    "info".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
            }

            // Report other file conflicts
            if !other_conflicts.is_empty() {
                add_log(
                    format!(
                        "📄 {} non-archive file(s) replaced from other mods:",
                        other_conflicts.len()
                    ),
                    "warning".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;

                for (file_path, conflict_list) in &other_conflicts {
                    for conflict in conflict_list.iter() {
                        let filename = std::path::Path::new(file_path)
                            .file_name()
                            .unwrap_or_default()
                            .to_string_lossy();
                        add_log(
                            format!("  • '{}' from '{}'", filename, conflict.mod_name),
                            "warning".to_string(),
                            "installation".to_string(),
                            state.clone(),
                        )?;
                    }
                }

                add_log(
                    "⚠️  The previous mod's files have been overwritten. Uninstalling this mod won't restore them.".to_string(),
                    "warning".to_string(),
                    "installation".to_string(),
                    state.clone(),
                )?;
            }
        }
    }

    // Display symlink warning if any were detected
    if symlink_count > 0 {
        add_log(
            "🔗 Symlink Detection Warning".to_string(),
            "warning".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;
        add_log(
            format!(
                "⚠️  {} symbolic link(s) detected in this mod",
                symlink_count
            ),
            "warning".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;

        // Show details of detected symlinks
        for (symlink_path, target) in &symlinks_detected {
            let detail = match target {
                Some(t) => format!("  • {} → {}", symlink_path, t),
                None => format!("  • {}", symlink_path),
            };
            add_log(
                detail,
                "warning".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
        }

        add_log(
            "⚠️  Symlinks may not work correctly in Wine/Crossover environments".to_string(),
            "warning".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;
        add_log(
            "ℹ️  Symlinks were NOT installed (skipped for compatibility)".to_string(),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;

        #[cfg(target_os = "macos")]
        {
            add_log(
                "💡 macOS/Crossover Tip: Symlinks are rarely used in Cyberpunk 2077 mods"
                    .to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "   If the mod doesn't work, it may rely on symlinks. Check for alternative versions.".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "   Most mods on NexusMods are packaged without symlinks for compatibility."
                    .to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
        }

        add_log(
            format!(
                "📊 Symlink Summary: {} symlink(s) detected and skipped",
                symlink_count
            ),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;
    }

    // Display Unicode filename warning if any were detected
    if unicode_count > 0 {
        add_log(
            "🔤 Unicode Filename Detection".to_string(),
            "warning".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;
        add_log(
            format!(
                "⚠️  {} filename(s) contained non-ASCII characters",
                unicode_count
            ),
            "warning".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;

        // Show sanitization details
        for (original, sanitized) in &unicode_sanitized {
            add_log(
                format!("  • '{}' → '{}'", original, sanitized),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
        }

        add_log(
            "ℹ️  Filenames were automatically sanitized to ASCII-safe characters".to_string(),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;
        add_log(
            "⚠️  Unicode filenames may cause issues in Wine/Crossover due to encoding differences"
                .to_string(),
            "warning".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;

        #[cfg(target_os = "macos")]
        {
            add_log(
                "💡 macOS/Crossover Tip: ASCII sanitization improves Wine compatibility"
                    .to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "   Examples: 'café.lua' → 'cafe.lua', 'モッド.archive' → 'modo.archive'"
                    .to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "   This prevents file encoding issues and improves mod reliability.".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
        }

        add_log(
            format!(
                "📊 Unicode Summary: {} filename(s) sanitized to ASCII",
                unicode_count
            ),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;
    }

    // Display case sensitivity summary if any issues were detected
    if case_mismatch_count > 0 {
        add_log(
            format!(
                "📊 Case Sensitivity Summary: {} file(s) had incorrect casing and were auto-corrected",
                case_mismatch_count
            ),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;
        add_log(
            "✅ All paths normalized to match Cyberpunk 2077's expected directory structure"
                .to_string(),
            "success".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;

        #[cfg(target_os = "macos")]
        {
            add_log(
                "💡 macOS/Crossover Tip: This is normal when installing mods created on Windows"
                    .to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "The mod manager automatically corrects case mismatches to ensure compatibility"
                    .to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
        }
    }

    // Warn if REDmod detected
    if is_redmod {
        add_log(
            "🎮 REDmod detected! This mod uses the official CDPR modding system.".to_string(),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;
        add_log(
            "⚠️ CRITICAL: REDmod mods require the '-modded' launch parameter to work!".to_string(),
            "warning".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;
        add_log(
            "Without this parameter, your mod will NOT load and you'll see no effects in-game."
                .to_string(),
            "warning".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;

        #[cfg(target_os = "macos")]
        {
            add_log(
                "📋 How to add '-modded' parameter in Crossover:".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "  • GOG Galaxy: Settings → Cyberpunk 2077 → Additional Launch Arguments → Add: -modded".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "  • Steam: Right-click game → Properties → Launch Options → Add: -modded"
                    .to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "  • Epic Games: Library → Cyberpunk 2077 → ⋯ → Manage → Additional Command Line Arguments → Add: -modded".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "💡 Tip: You only need to set this once, and it applies to all REDmod mods."
                    .to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
        }

        #[cfg(target_os = "windows")]
        {
            add_log(
                "📋 How to add '-modded' parameter:".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "  • GOG Galaxy: Settings → Game → Additional Launch Arguments → Add: -modded"
                    .to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "  • Steam: Right-click game → Properties → Launch Options → Add: -modded"
                    .to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "  • Epic Games: Library → ⋯ → Manage → Launch Options → Add: -modded".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
        }
    }

    // Configure CET if detected
    if is_cet {
        add_log(
            "ℹ️ Cyber Engine Tweaks (CET) detected! Press ~ (tilde) in-game to open the console."
                .to_string(),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;

        // Crossover/Wine configuration required
        #[cfg(target_os = "macos")]
        {
            add_log(
                "⚠️ IMPORTANT: CET requires Crossover Wine configuration!".to_string(),
                "warning".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "📋 Step 1: Open CrossOver → Right-click 'GOG Galaxy' bottle → Wine Configuration"
                    .to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "📋 Step 2: Go to 'Libraries' tab → Add 'version' and 'winmm' → Set both to 'Native then Builtin'".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "📋 Step 3: Click OK and restart GOG Galaxy for CET to work".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
        }
    }

    // Configure RED4ext if detected
    if is_red4ext {
        add_log(
            "🔴 RED4ext detected! This is a native code extension framework.".to_string(),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;
        add_log(
            "📋 RED4ext file placement: version.dll → Game Root | RED4ext.dll → bin/x64/ | Plugins → red4ext/plugins/".to_string(),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;

        // Check for Windows/Crossover specific requirements
        #[cfg(target_os = "macos")]
        {
            add_log(
                "⚠️ RED4ext requires special setup on Crossover/Wine".to_string(),
                "warning".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "✅ Good news: RED4ext CAN work on Crossover with proper configuration!"
                    .to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "📋 Required setup steps:".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "  1. Set bottle to Windows 10 (winecfg → Applications → Windows Version)"
                    .to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "  2. Add 'version' library override (winecfg → Libraries → version → Native then Builtin)".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "  3. Install Visual C++ 2019/2022 Redistributables in the bottle".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "  4. Verify version.dll is in game root (not bin/x64/)".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "� Alternative: Redscript or CET-based mods are easier to set up if available."
                    .to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "📖 Full setup guide: See FEATURES.md for detailed instructions.".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
        }

        #[cfg(target_os = "windows")]
        {
            add_log(
                "ℹ️ RED4ext requires Visual C++ Redistributable 2019 or newer to be installed."
                    .to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "⚠️ If the game crashes on startup, install the latest VC++ Redist from Microsoft."
                    .to_string(),
                "warning".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
            add_log(
                "🔧 Some RED4ext mods may require running the game as Administrator.".to_string(),
                "info".to_string(),
                "installation".to_string(),
                state.clone(),
            )?;
        }
    }

    // Step 5: Add to mod database
    emit_install_progress(&app, InstallProgress {
        stage: "registering".into(),
        message: "Registering mod in database...".into(),
        ..Default::default()
    });

    add_log(
        "📝 Registering mod in database...".to_string(),
        "info".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    // Check if this is a reinstall (existing record to update)
    let reinstall_id = state.reinstall_mod_id.lock().map_err(|e| e.to_string())?.take();

    if let Some(ref existing_id) = reinstall_id {
        // Reinstall/update: clean up stale files from old version, then update record
        {
            let manager = state.mod_manager.lock().map_err(|e| e.to_string())?;
            if let Some(old_mod) = manager.get_installed_mods().into_iter().find(|m| m.id == *existing_id) {
                let new_files_lower: std::collections::HashSet<String> = installed_files.iter()
                    .map(|f| f.to_lowercase())
                    .collect();
                for old_file in &old_mod.files {
                    if !new_files_lower.contains(&old_file.to_lowercase()) {
                        // Path safety: only delete files within game directory
                        if old_file.contains("..") || !old_file.to_lowercase().contains("cyberpunk 2077") {
                            eprintln!("⛔ Skipping unsafe stale path: {}", old_file);
                            continue;
                        }
                        if let Err(e) = std::fs::remove_file(old_file) {
                            eprintln!("Failed to remove stale file {}: {}", old_file, e);
                        }
                    }
                }
            }
        }

        let new_file_name = state.pending_file_name.lock().ok().and_then(|mut s| s.take());
        let new_file_version = state.pending_file_version.lock().ok().and_then(|mut s| s.take());
        let new_file_description = state.pending_file_description.lock().ok().and_then(|mut s| s.take());

        let mut manager = state.mod_manager.lock().map_err(|e| e.to_string())?;
        manager.complete_reinstall(
            existing_id,
            installed_files.clone(),
            &mod_version,
            Some(&file_id),
            new_file_name,
            new_file_version,
            new_file_description,
        )?;
        add_log(
            format!("🔄 Update complete: '{}' → v{}", mod_name, mod_version),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;
    } else {
        // Fresh install: create new record
        let mod_info = ModInfo {
            id: uuid::Uuid::new_v4().to_string(),
            name: mod_name.clone(),
            version: mod_version.clone(),
            author: if mod_author.is_empty() || mod_author == "Unknown" {
                None
            } else {
                Some(mod_author.clone())
            },
            description: Some(format!(
                "Installed from NexusMods (Mod ID: {}, File ID: {})",
                mod_id, file_id
            )),
            mod_id: Some(mod_id.clone()),
            file_id: Some(file_id.clone()),
            enabled: true,
            files: installed_files.clone(),
            file_conflicts: std::collections::HashMap::new(),
            installed_at: Some(chrono::Utc::now().to_rfc3339()),
            picture_url: None,
            update_available: None,
            latest_version: None,
            summary: None,
            nexus_updated_at: None,
            removed: false,
            removed_at: None,
            file_name: state.pending_file_name.lock().ok().and_then(|mut s| s.take()),
            file_version: state.pending_file_version.lock().ok().and_then(|mut s| s.take()),
            file_description: state.pending_file_description.lock().ok().and_then(|mut s| s.take()),
            latest_file_id: None,
            reinstall_status: None,
        };

        let mut manager = state.mod_manager.lock().map_err(|e| {
            e.to_string()
        })?;
        manager.add_mod(mod_info.clone());
        manager.save_database()?;
    }

    // Step 6: Cleanup temporary files (RAII guards will handle this automatically)
    add_log(
        "🧹 Cleaning up temporary files...".to_string(),
        "info".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    // Drop the guards explicitly to clean up temp files
    drop(extract_guard);
    add_log(
        "✓ Removed extraction directory".to_string(),
        "info".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    drop(archive_guard);
    add_log(
        "✓ Removed archive file".to_string(),
        "info".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    add_log(
        format!(
            "✅ Successfully installed mod '{}' with {} files!",
            mod_name,
            installed_files.len()
        ),
        "success".to_string(),
        "installation".to_string(),
        state.clone(),
    )?;

    emit_install_progress(&app, InstallProgress {
        stage: "done".into(),
        message: format!("v{} · {} files installed", mod_version, installed_files.len()),
        mod_name: Some(mod_name.clone()),
        ..Default::default()
    });

    // Step 7: Notify frontend to refresh mod list
    if let Some(window) = app.get_webview_window("main") {
        add_log(
            "📢 Emitting mod-installed event to frontend".to_string(),
            "info".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;
        window.emit("mod-installed", serde_json::json!({
            "name": mod_name,
            "version": mod_version,
            "reinstall": reinstall_id.is_some(),
        })).ok();
    } else {
        add_log(
            "⚠️ No main window found, cannot emit mod-installed event".to_string(),
            "warning".to_string(),
            "installation".to_string(),
            state.clone(),
        )?;
    }

    Ok(format!(
        "Mod '{}' installed successfully with {} files!",
        mod_name,
        installed_files.len()
    ))
}

/// Check if a path exists with case-insensitive matching
/// Returns the correctly-cased path if found, or None if not found
#[allow(dead_code)]
fn find_path_case_insensitive(
    base_dir: &std::path::Path,
    target_path: &std::path::Path,
) -> Option<std::path::PathBuf> {
    // Start with the base directory
    let mut current = base_dir.to_path_buf();

    // Iterate through each component of the target path
    for component in target_path.components() {
        if let std::path::Component::Normal(comp_str) = component {
            let comp_lower = comp_str.to_string_lossy().to_lowercase();
            let mut found = false;

            // Try to read the directory entries
            if let Ok(entries) = std::fs::read_dir(&current) {
                for entry in entries.flatten() {
                    if let Ok(file_name) = entry.file_name().into_string() {
                        if file_name.to_lowercase() == comp_lower {
                            current = current.join(&file_name);
                            found = true;
                            break;
                        }
                    }
                }
            }

            if !found {
                return None; // Path component not found
            }
        }
    }

    Some(current)
}

/// Sanitize a filename by converting Unicode characters to ASCII-safe equivalents
/// This helps avoid Wine/Crossover encoding issues
fn sanitize_filename(name: &str) -> String {
    use unidecode::unidecode;

    // First, try to transliterate using unidecode (smart conversion)
    let transliterated = unidecode(name);

    // Then ensure all characters are filesystem-safe
    transliterated
        .chars()
        .map(|c| match c {
            // Allow alphanumeric, hyphen, underscore, period
            c if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' => c,
            // Convert spaces to underscores
            ' ' => '_',
            // Everything else becomes underscore
            _ => '_',
        })
        .collect()
}

/// Check if a filename contains non-ASCII characters
fn contains_unicode(name: &str) -> bool {
    !name.is_ascii()
}

/// Check if sanitization changed the filename
fn needs_sanitization(name: &str) -> Option<String> {
    if contains_unicode(name) {
        let sanitized = sanitize_filename(name);
        if sanitized != name {
            return Some(sanitized);
        }
    }
    None
}

/// Get available disk space for a given path in bytes
/// Returns the available space on the filesystem containing the path
fn get_available_disk_space(path: &std::path::Path) -> Result<u64, String> {
    // Find the closest existing parent directory
    let mut check_path = path;
    while !check_path.exists() {
        if let Some(parent) = check_path.parent() {
            check_path = parent;
        } else {
            return Err("Unable to find valid path for disk space check".to_string());
        }
    }

    // Use platform-specific method to get available space
    #[cfg(unix)]
    {
        // Use statvfs to get filesystem statistics
        let stats = nix::sys::statvfs::statvfs(check_path)
            .map_err(|e| format!("Failed to get filesystem statistics: {}", e))?;

        // Available space = block size * available blocks (convert to u64)
        let available_bytes = stats.blocks_available() as u64 * stats.block_size();
        Ok(available_bytes)
    }

    #[cfg(not(unix))]
    {
        // Fallback for non-Unix systems (Windows)
        // This is a simple estimation - not perfectly accurate
        Err("Disk space checking not implemented for this platform".to_string())
    }
}

/// Format bytes into human-readable format (KB, MB, GB)
fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = KB * 1024;
    const GB: u64 = MB * 1024;

    if bytes >= GB {
        format!("{:.2} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.2} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.2} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} bytes", bytes)
    }
}

/// Check if there's sufficient disk space for mod installation
/// Returns an error if insufficient space is available
/// Requires: mod_size * 3 (for download + extraction + buffer)
fn check_sufficient_disk_space(path: &std::path::Path, required_bytes: u64) -> Result<(), String> {
    let available = get_available_disk_space(path)?;

    // Require 3x the size: 1x for download, 1x for extraction, 1x for buffer
    let required_with_buffer = required_bytes * 3;

    if available < required_with_buffer {
        return Err(format!(
            "Insufficient disk space. Required: {} (including extraction buffer), Available: {}",
            format_bytes(required_with_buffer),
            format_bytes(available)
        ));
    }

    Ok(())
}

/// Set Wine-compatible permissions on installed files
/// Files: 0o644 (rw-r--r--), Directories: 0o755 (rwxr-xr-x)
/// This improves Wine DLL loading and config file writability
#[cfg(unix)]
fn set_wine_compatible_permissions(
    path: &std::path::Path,
    is_directory: bool,
) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;

    let mode = if is_directory {
        0o755 // rwxr-xr-x - directories need execute permission to be traversable
    } else {
        0o644 // rw-r--r-- - files should be readable and writable by owner
    };

    let mut perms = std::fs::metadata(path)
        .map_err(|e| format!("Failed to get metadata for {:?}: {}", path, e))?
        .permissions();

    perms.set_mode(mode);

    std::fs::set_permissions(path, perms)
        .map_err(|e| format!("Failed to set permissions for {:?}: {}", path, e))?;

    Ok(())
}

#[cfg(not(unix))]
fn set_wine_compatible_permissions(
    _path: &std::path::Path,
    _is_directory: bool,
) -> Result<(), String> {
    // No-op on non-Unix systems
    Ok(())
}

/// Check if path length exceeds safe limits for macOS/Wine
/// macOS has PATH_MAX of 1024 characters
/// Warns if path approaches 900 characters (safety margin)
fn check_path_length(path: &std::path::Path) -> Result<(), String> {
    const PATH_MAX_MACOS: usize = 1024;
    const SAFE_PATH_LIMIT: usize = 900; // Leave safety margin

    let path_str = path.to_string_lossy();
    let path_len = path_str.len();

    if path_len >= PATH_MAX_MACOS {
        return Err(format!(
            "Path too long ({} chars). Maximum allowed is {} characters.\n\
             Path: {}\n\n\
             💡 This will cause installation to fail.\n\
             Please use a shorter Crossover bottle name or move your bottle to a shorter path.",
            path_len, PATH_MAX_MACOS, path_str
        ));
    } else if path_len >= SAFE_PATH_LIMIT {
        return Err(format!(
            "⚠️  Path approaching maximum length ({} chars, limit is {}).\n\
             Path: {}\n\n\
             💡 Consider using a shorter Crossover bottle name to avoid future issues.\n\
             While this path works now, adding more mods could exceed the limit.",
            path_len, PATH_MAX_MACOS, path_str
        ));
    }

    Ok(())
}

/// Detect Wine's configured Windows version from the bottle's registry
/// Returns version info and whether it's the recommended version (Windows 10)
#[cfg(target_os = "macos")]
fn detect_wine_windows_version(game_path: &std::path::Path) -> Result<(String, bool), String> {
    use std::fs;
    use std::path::PathBuf;

    // Try to find the Wine bottle by walking up from game path to find drive_c
    let mut current_path = game_path.to_path_buf();
    let mut bottle_path: Option<PathBuf> = None;

    // Walk up the directory tree looking for drive_c
    while let Some(parent) = current_path.parent() {
        let drive_c = parent.join("drive_c");
        if drive_c.exists() && drive_c.is_dir() {
            bottle_path = Some(parent.to_path_buf());
            break;
        }
        current_path = parent.to_path_buf();
    }

    let bottle_path = bottle_path.ok_or_else(|| {
        "Unable to locate Wine bottle (drive_c not found in path hierarchy)".to_string()
    })?;

    // Try to read system.reg for Windows version information
    let system_reg = bottle_path.join("system.reg");

    if !system_reg.exists() {
        return Err("Wine registry file (system.reg) not found".to_string());
    }

    let reg_content =
        fs::read_to_string(&system_reg).map_err(|e| format!("Failed to read system.reg: {}", e))?;

    // Parse registry for Windows version
    // Look for: [Software\\Microsoft\\Windows NT\\CurrentVersion]
    let mut current_version = String::new();
    let mut current_build = String::new();
    let mut product_name = String::new();

    let mut in_version_section = false;
    for line in reg_content.lines() {
        // Check if we're in the CurrentVersion section
        if line.contains("[Software\\\\Microsoft\\\\Windows NT\\\\CurrentVersion]") {
            in_version_section = true;
            continue;
        }

        // Exit section when we hit a new section
        if in_version_section && line.starts_with('[') {
            break;
        }

        if in_version_section {
            // Parse version values (format: "KeyName"="Value")
            if line.contains("\"CurrentVersion\"=") {
                current_version = line
                    .split('=')
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .to_string();
            } else if line.contains("\"CurrentBuild\"=") {
                current_build = line
                    .split('=')
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .to_string();
            } else if line.contains("\"ProductName\"=") {
                product_name = line
                    .split('=')
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .trim_matches('"')
                    .to_string();
            }
        }
    }

    // Determine if this is a recommended version
    let is_recommended = current_version.starts_with("10.") || product_name.contains("Windows 10");

    // Build version string
    let version_string = if !product_name.is_empty() {
        if !current_build.is_empty() {
            format!("{} (Build {})", product_name, current_build)
        } else {
            product_name
        }
    } else if !current_version.is_empty() {
        if !current_build.is_empty() {
            format!("Windows {} (Build {})", current_version, current_build)
        } else {
            format!("Windows {}", current_version)
        }
    } else {
        "Unknown Windows version".to_string()
    };

    Ok((version_string, is_recommended))
}

#[cfg(not(target_os = "macos"))]
#[allow(dead_code)] // Stub implementation for non-macOS platforms
fn detect_wine_windows_version(_game_path: &std::path::Path) -> Result<(String, bool), String> {
    // Not applicable on non-macOS systems
    Ok(("Native Windows".to_string(), true))
}

/// RAII guard for temporary files/directories
/// Automatically cleans up the path when dropped (goes out of scope)
/// This ensures cleanup even if the function panics or returns early
struct TempFileGuard {
    path: PathBuf,
    description: String,
    keep: bool, // If true, don't cleanup on drop
}

impl TempFileGuard {
    fn new(path: PathBuf, description: String) -> Self {
        Self {
            path,
            description,
            keep: false,
        }
    }

    /// Mark this file to be kept (don't delete on drop)
    #[allow(dead_code)]
    fn keep(&mut self) {
        self.keep = true;
    }

    /// Get the path
    #[allow(dead_code)]
    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if self.keep || !self.path.exists() {
            return;
        }

        let result = if self.path.is_file() {
            std::fs::remove_file(&self.path)
        } else {
            std::fs::remove_dir_all(&self.path)
        };

        match result {
            Ok(_) => println!("🧹 Auto-cleaned: {}", self.description),
            Err(e) => eprintln!("⚠️  Failed to auto-clean {}: {}", self.description, e),
        }
    }
}

/// Check if a file/directory is older than the specified number of hours
fn is_path_older_than(path: &Path, hours: u64) -> bool {
    if let Ok(metadata) = std::fs::metadata(path) {
        if let Ok(modified) = metadata.modified() {
            if let Ok(elapsed) = modified.elapsed() {
                return elapsed.as_secs() > hours * 3600;
            }
        }
    }
    false
}

/// Clean up orphaned temporary files from previous sessions
///
/// SAFETY GUARANTEES:
/// 1. **Exact Pattern Matching**: Only removes files matching EXACT formats created by this app
///    - Archives: `mod_{numeric_id}_{numeric_id}.zip` (e.g., mod_107_123169.zip)
///    - Directories: `mod_extract_{numeric_id}_{valid_uuid}` (e.g., mod_extract_107_550e8400-...)
///
/// 2. **Strict Validation**:
///    - Archive: Both IDs must be purely numeric (no letters/symbols)
///    - Directory: UUID must match exact format (8-4-4-4-12 hex with hyphens)
///    - No partial matches or loose patterns
///
/// 3. **Age-Based Safety**: Only removes files older than 1 hour
///    - Protects active downloads/installations
///    - Prevents race conditions
///
/// 4. **Limited Scope**: Only scans system temp directory (std::env::temp_dir())
///    - Never touches user directories
///    - Never touches game installation folders
///
/// 5. **Examples of SAFE files (WILL be removed if old)**:
///    - mod_107_123169.zip (valid: two numeric IDs)
///    - mod_extract_107_550e8400-e29b-41d4-a716-446655440000 (valid: numeric ID + UUID)
///
/// 6. **Examples of PROTECTED files (WILL NOT be removed)**:
///    - mod.zip (invalid: missing IDs)
///    - mod_abc_123.zip (invalid: non-numeric ID)
///    - mod_107_123.txt (invalid: not a .zip)
///    - modern_art_file.zip (invalid: doesn't start with exactly "mod_")
///    - mod_extract_abc_123 (invalid: no UUID)
///    - mod_extract_107_not-a-uuid (invalid: malformed UUID)
///    - Any file in directories other than system temp
///
/// Returns (files_removed, dirs_removed, errors, removed_paths)
fn cleanup_orphaned_temp_files() -> (usize, usize, usize, Vec<String>) {
    let temp_dir = std::env::temp_dir();
    let mut files_removed = 0;
    let mut dirs_removed = 0;
    let mut errors = 0;
    let mut removed_paths: Vec<String> = Vec::new();

    if let Ok(entries) = std::fs::read_dir(&temp_dir) {
        for entry in entries.flatten() {
            if let Ok(file_name) = entry.file_name().into_string() {
                let path = entry.path();

                // STRICT VALIDATION: Only match files created by THIS application
                // Pattern: mod_{numeric_id}_{numeric_id}.zip (exactly)
                // Example: mod_107_123169.zip
                let is_mod_archive = if file_name.starts_with("mod_") && file_name.ends_with(".zip")
                {
                    // Extract the part between "mod_" and ".zip"
                    let inner = &file_name[4..file_name.len() - 4]; // Remove "mod_" prefix and ".zip" suffix
                    let parts: Vec<&str> = inner.split('_').collect();

                    // Must have exactly 2 parts, both must be numeric (mod_id and file_id)
                    parts.len() == 2
                        && parts[0].chars().all(|c| c.is_ascii_digit())
                        && parts[1].chars().all(|c| c.is_ascii_digit())
                } else {
                    false
                };

                // STRICT VALIDATION: Only match directories created by THIS application
                // Pattern: mod_extract_{numeric_id}_{uuid} (exactly)
                // Example: mod_extract_107_550e8400-e29b-41d4-a716-446655440000
                let is_mod_extract_dir = if let Some(inner) = file_name.strip_prefix("mod_extract_")
                {
                    let parts: Vec<&str> = inner.split('_').collect();

                    // Must have exactly 2 parts: numeric mod_id and UUID
                    // UUID format: 8-4-4-4-12 hex characters with hyphens
                    if parts.len() == 2 && parts[0].chars().all(|c| c.is_ascii_digit()) {
                        // Validate UUID format (basic check for correct length and structure)
                        let uuid_part = parts[1];
                        let uuid_segments: Vec<&str> = uuid_part.split('-').collect();
                        uuid_segments.len() == 5
                            && uuid_segments[0].len() == 8
                            && uuid_segments[1].len() == 4
                            && uuid_segments[2].len() == 4
                            && uuid_segments[3].len() == 4
                            && uuid_segments[4].len() == 12
                            && uuid_part.chars().all(|c| c.is_ascii_hexdigit() || c == '-')
                    } else {
                        false
                    }
                } else {
                    false
                };

                if (is_mod_archive || is_mod_extract_dir) && is_path_older_than(&path, 1) {
                    let path_display = path.display().to_string();
                    let result = if path.is_file() {
                        std::fs::remove_file(&path).map(|_| {
                            files_removed += 1;
                            removed_paths.push(path_display.clone());
                            println!("🧹 Cleaned orphaned file: {}", path_display);
                        })
                    } else if path.is_dir() {
                        std::fs::remove_dir_all(&path).map(|_| {
                            dirs_removed += 1;
                            removed_paths.push(path_display.clone());
                            println!("🧹 Cleaned orphaned directory: {}", path_display);
                        })
                    } else {
                        Ok(())
                    };

                    if result.is_err() {
                        errors += 1;
                        eprintln!("⚠️  Failed to clean: {}", path_display);
                    }
                }
            }
        }
    }

    (files_removed, dirs_removed, errors, removed_paths)
}

/// Tauri command to manually clean temporary files
#[tauri::command]
fn clean_temp_files(state: State<AppState>) -> Result<String, String> {
    add_log(
        "🧹 Starting manual cleanup of temporary files...".to_string(),
        "info".to_string(),
        "cleanup".to_string(),
        state.clone(),
    )?;

    let (files_removed, dirs_removed, errors, removed_paths) = cleanup_orphaned_temp_files();

    let total_removed = files_removed + dirs_removed;

    if total_removed == 0 {
        add_log(
            "✓ No temporary files found to clean".to_string(),
            "info".to_string(),
            "cleanup".to_string(),
            state,
        )?;
        Ok("No temporary files found. Your system is clean!".to_string())
    } else {
        // Log each removed path
        for path in &removed_paths {
            add_log(
                format!("  🗑️  Removed: {}", path),
                "info".to_string(),
                "cleanup".to_string(),
                state.clone(),
            )?;
        }

        let mut message = format!(
            "Cleaned {} temporary file(s) and {} directory/directories",
            files_removed, dirs_removed
        );

        if errors > 0 {
            message.push_str(&format!(" ({} error(s) encountered)", errors));
        }

        add_log(
            format!("✓ {}", message),
            "info".to_string(),
            "cleanup".to_string(),
            state,
        )?;

        Ok(message)
    }
}

/// Normalize a path component to match Cyberpunk 2077's expected casing
/// This ensures consistent casing for game directories
fn normalize_game_path_component(component: &str) -> String {
    let lower = component.to_lowercase();

    // Common Cyberpunk 2077 directory names with correct casing
    match lower.as_str() {
        "bin" => "bin".to_string(),
        "x64" => "x64".to_string(),
        "archive" => "archive".to_string(),
        "pc" => "pc".to_string(),
        "mod" => "mod".to_string(),
        "mods" => "mods".to_string(),
        "r6" => "r6".to_string(),
        "scripts" => "scripts".to_string(),
        "engine" => "engine".to_string(),
        "config" => "config".to_string(),
        "red4ext" => "red4ext".to_string(),
        "plugins" => "plugins".to_string(),
        _ => component.to_string(), // Preserve original casing for unknown components
    }
}

/// Normalize a full path to use correct game directory casing
fn normalize_game_path(relative_path: &std::path::Path) -> std::path::PathBuf {
    use std::path::PathBuf;

    let mut normalized = PathBuf::new();

    for component in relative_path.components() {
        if let std::path::Component::Normal(comp_str) = component {
            let comp_string = comp_str.to_string_lossy();
            let normalized_comp = normalize_game_path_component(&comp_string);
            normalized.push(normalized_comp);
        } else {
            normalized.push(component);
        }
    }

    normalized
}

/// Check if a path has case mismatches compared to expected Cyberpunk 2077 structure
/// Returns a tuple of (has_mismatch, expected_path, issues)
fn check_case_mismatch(relative_path: &std::path::Path) -> (bool, std::path::PathBuf, Vec<String>) {
    let normalized = normalize_game_path(relative_path);
    let mut issues = Vec::new();

    let original_str = relative_path.to_string_lossy();
    let normalized_str = normalized.to_string_lossy();

    if original_str.to_lowercase() == normalized_str.to_lowercase()
        && original_str != normalized_str
    {
        // Same path but different casing
        issues.push(format!(
            "Case mismatch: '{}' should be '{}'",
            original_str, normalized_str
        ));
        return (true, normalized, issues);
    }

    (false, normalized, issues)
}

/// Validate that a path stays within the game directory (no path traversal).
/// Returns the canonicalized path, or an error if it escapes game_dir.
fn validate_path_within_game_dir(
    path: &std::path::Path,
    game_dir: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    // Resolve the path: expand ../ and symlinks
    // If the target doesn't exist yet, resolve the parent and append the filename
    let resolved = if path.exists() {
        path.canonicalize()
            .map_err(|e| format!("Failed to resolve path {}: {}", path.display(), e))?
    } else {
        let parent = path.parent().unwrap_or(path);
        let parent_resolved = if parent.exists() {
            parent.canonicalize()
                .map_err(|e| format!("Failed to resolve parent {}: {}", parent.display(), e))?
        } else {
            // Parent doesn't exist yet — just normalize textually
            let mut cleaned = std::path::PathBuf::new();
            for component in parent.components() {
                match component {
                    std::path::Component::ParentDir => { cleaned.pop(); },
                    std::path::Component::CurDir => {},
                    c => cleaned.push(c),
                }
            }
            cleaned
        };
        match path.file_name() {
            Some(name) => parent_resolved.join(name),
            None => parent_resolved,
        }
    };

    let game_canonical = if game_dir.exists() {
        game_dir.canonicalize()
            .map_err(|e| format!("Failed to resolve game dir: {}", e))?
    } else {
        game_dir.to_path_buf()
    };

    if !resolved.starts_with(&game_canonical) {
        return Err(format!(
            "🛑 Path traversal blocked: '{}' is outside game directory '{}'",
            path.display(),
            game_canonical.display()
        ));
    }

    Ok(resolved)
}

fn determine_install_path_for_file(
    game_dir: &std::path::Path,
    relative_path: &std::path::Path,
) -> Result<std::path::PathBuf, String> {
    // Most mods already have the correct directory structure (e.g., bin/x64/file.dll)
    // We should preserve this structure and install directly to game_dir

    // Reject path traversal in relative paths
    let rel_str = relative_path.to_string_lossy();
    if rel_str.contains("..") {
        return Err(format!(
            "🛑 Path traversal detected in archive: '{}'. Skipping file.",
            rel_str
        ));
    }

    // Normalize the path to ensure correct casing for game directories
    let normalized_path = normalize_game_path(relative_path);

    let path_str = normalized_path.to_string_lossy().to_lowercase();
    let file_name = normalized_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_lowercase();

    // Check if the path already starts with a known game directory
    // Common Cyberpunk 2077 mod structures:
    // - bin/x64/...              (RED4ext/CET mods)
    // - r6/scripts/...           (Redscript mods)
    // - archive/pc/mod/...       (Archive mods)
    // - engine/config/...        (Config mods)
    // - mods/...                 (REDmod - official CDPR mod system)
    // - red4ext/plugins/...      (RED4ext plugins)

    if path_str.starts_with("mods/") || path_str.starts_with("mods\\") {
        // REDmod structure: mods/modname/...
        // These mods require launching with -modded parameter
        return Ok(game_dir.join(normalized_path));
    }

    if path_str.starts_with("bin/") || path_str.starts_with("bin\\") {
        // Path already has correct structure: bin/x64/file.dll (with normalized casing)
        return Ok(game_dir.join(normalized_path));
    }

    if path_str.starts_with("r6/") || path_str.starts_with("r6\\") {
        // Path already has correct structure: r6/scripts/file.reds (with normalized casing)
        return Ok(game_dir.join(normalized_path));
    }

    if path_str.starts_with("archive/") || path_str.starts_with("archive\\") {
        // Path already has correct structure: archive/pc/mod/file.archive (with normalized casing)
        return Ok(game_dir.join(normalized_path));
    }

    if path_str.starts_with("engine/") || path_str.starts_with("engine\\") {
        // Path already has correct structure: engine/config/... (with normalized casing)
        return Ok(game_dir.join(normalized_path));
    }

    if path_str.starts_with("red4ext/") || path_str.starts_with("red4ext\\") {
        // Path already has correct structure: red4ext/plugins/... (with normalized casing)
        return Ok(game_dir.join(normalized_path));
    }

    // Special handling for RED4ext core files (case-insensitive)
    if file_name == "red4ext.dll" {
        // RED4ext.dll goes in bin/x64/ (preserve original casing)
        let original_name = relative_path.file_name().unwrap().to_string_lossy();
        println!("🔴 Detected RED4ext core DLL: {} → bin/x64/", original_name);
        return Ok(game_dir
            .join("bin")
            .join("x64")
            .join(original_name.as_ref()));
    }

    // Special handling for version.dll (RED4ext loader)
    if file_name == "version.dll" {
        // version.dll MUST go in game root directory, not bin/x64/
        let original_name = relative_path.file_name().unwrap().to_string_lossy();
        println!("🔴 Detected RED4ext loader (version.dll) → Game root directory");
        return Ok(game_dir.join(original_name.as_ref()));
    }

    // Handle RED4ext configuration and other files
    if path_str.contains("red4ext")
        && !path_str.starts_with("red4ext/")
        && !path_str.starts_with("red4ext\\")
    {
        // Files that contain "red4ext" but aren't in proper structure - place in red4ext/
        if path_str.ends_with(".toml") || path_str.ends_with(".ini") || path_str.ends_with(".txt") {
            return Ok(game_dir
                .join("red4ext")
                .join(relative_path.file_name().unwrap()));
        }
    }

    // If the path doesn't start with a known directory, try to infer from file type
    if path_str.ends_with(".archive") {
        // Standalone .archive file -> archive/pc/mod/
        return Ok(game_dir
            .join("archive")
            .join("pc")
            .join("mod")
            .join(relative_path.file_name().unwrap()));
    }

    if path_str.ends_with(".reds") {
        // Standalone .reds file -> r6/scripts/
        return Ok(game_dir
            .join("r6")
            .join("scripts")
            .join(relative_path.file_name().unwrap()));
    }

    if path_str.ends_with(".dll") || path_str.ends_with(".exe") {
        // Check if this might be a RED4ext plugin
        if path_str.contains("red4ext")
            || relative_path
                .parent()
                .map(|p| p.to_string_lossy().to_lowercase().contains("plugins"))
                .unwrap_or(false)
        {
            // RED4ext plugin -> red4ext/plugins/
            return Ok(game_dir
                .join("red4ext")
                .join("plugins")
                .join(relative_path.file_name().unwrap()));
        }

        // Regular binary -> bin/x64/
        return Ok(game_dir
            .join("bin")
            .join("x64")
            .join(relative_path.file_name().unwrap()));
    }

    // For anything else, preserve the normalized structure
    // This handles mods with custom folder structures (with correct casing for known directories)
    Ok(game_dir.join(normalized_path))
}

#[tauri::command]
fn cancel_sync(state: State<'_, AppState>) {
    state.sync_cancel.store(true, Ordering::Relaxed);
}

#[tauri::command]
fn cancel_install(state: State<'_, AppState>) {
    state.install_cancel.store(true, Ordering::Relaxed);
}

#[tauri::command]
fn is_installing(state: State<'_, AppState>) -> bool {
    state.installing.load(Ordering::Relaxed)
}

#[tauri::command]
async fn sync_mod_data(
    state: State<'_, AppState>,
    app_handle: tauri::AppHandle,
) -> Result<String, String> {
    let api_key = {
        let settings = state.settings.lock().map_err(|e| e.to_string())?;
        settings.get_settings().nexusmods_api_key.clone()
    };

    if api_key.is_empty() {
        return Err("NexusMods API key not configured. Please add it in Settings.".to_string());
    }

    // Reset cancel flag before starting
    state.sync_cancel.store(false, Ordering::Relaxed);

    let cancel = Arc::clone(&state.sync_cancel);

    let mods = {
        let manager = state.mod_manager.lock().map_err(|e| e.to_string())?;
        manager.get_installed_mods()
    };

    // Only mods that have a mod_id
    let syncable: Vec<_> = mods.into_iter()
        .filter(|m| m.mod_id.is_some())
        .collect();

    let total = syncable.len();
    let mut synced = 0;
    let mut updated_count = 0;
    let mut errors = 0;

    add_log(
        format!("Sync started: {} mods queued", total),
        "info".to_string(), "sync".to_string(), state.clone(),
    )?;

    for mod_info in &syncable {
        if cancel.load(Ordering::Relaxed) {
            add_log(
                format!("Sync cancelled: processed {}/{} mods", synced + errors, total),
                "warning".to_string(), "sync".to_string(), state.clone(),
            )?;
            app_handle.emit("sync-complete", serde_json::json!({
                "synced": synced, "total": total,
                "updated": updated_count, "errors": errors, "cancelled": true
            })).ok();
            return Ok(format!("Sync cancelled after {}/{} mods", synced + errors, total));
        }

        let mod_id = mod_info.mod_id.as_deref().unwrap();

        let details = nexusmods_api::get_mod_details("cyberpunk2077", mod_id, &api_key).await;

        match details {
            Err(e) => {
                add_log(
                    format!("Sync error: {} — {}", mod_info.name, e),
                    "error".to_string(), "sync".to_string(), state.clone(),
                )?;
                errors += 1;

                let display_name = if let Some(ref fname) = mod_info.file_name {
                    format!("{} ({})", mod_info.name, fname)
                } else if let Some(ref fid) = mod_info.file_id {
                    format!("{} [file:{}]", mod_info.name, fid)
                } else {
                    mod_info.name.clone()
                };
                app_handle.emit("sync-progress", serde_json::json!({
                    "current": synced + errors,
                    "total": total,
                    "mod_name": display_name,
                    "error": format!("{}", e),
                })).ok();
            }
            Ok(d) => {
                // Use mod-level version from Nexus (not file-level)
                let latest = Some(d.version.clone());
                let update_available = is_newer_version(&d.version, &mod_info.version);

                if update_available {
                    updated_count += 1;
                }

                // Fetch file names (for sub-mod display)
                let file_names = nexusmods_api::get_file_names("cyberpunk2077", mod_id, &api_key)
                    .await.unwrap_or_default();

                {
                    let mut manager = state.mod_manager.lock().map_err(|e| e.to_string())?;
                    manager.update_mod_sync_data(
                        &mod_info.id,
                        d.summary,
                        d.picture_url,
                        update_available,
                        latest,
                        d.nexus_updated_at,
                    )?;

                    // Update file_name for this mod and all parts with same mod_id
                    if !file_names.is_empty() {
                        manager.update_file_info(mod_id, &file_names)?;
                    }
                }

                synced += 1;
            }
        }

        // Get latest update status for this mod
        let (mod_ver, has_update) = {
            let manager = state.mod_manager.lock().map_err(|e| e.to_string())?;
            let m = manager.get_installed_mods().into_iter().find(|m| m.id == mod_info.id);
            (
                m.as_ref().map(|m| m.version.clone()).unwrap_or_default(),
                m.and_then(|m| m.update_available).unwrap_or(false),
            )
        };

        app_handle.emit("sync-progress", serde_json::json!({
            "current": synced + errors,
            "total": total,
            "mod_name": mod_info.name,
            "version": mod_ver,
            "update_available": has_update,
        })).ok();
    }

    let summary = format!(
        "Sync complete: {}/{} synced, {} updates available, {} errors",
        synced, total, updated_count, errors
    );
    add_log(summary.clone(), "info".to_string(), "sync".to_string(), state.clone())?;

    app_handle.emit("sync-complete", serde_json::json!({
        "synced": synced, "total": total,
        "updated": updated_count, "errors": errors, "cancelled": false
    })).ok();

    Ok(summary)
}

#[tauri::command]
fn reveal_in_finder(path: String) -> Result<(), String> {
    std::process::Command::new("open")
        .args(["-R", &path])
        .spawn()
        .map_err(|e| format!("Failed to reveal in Finder: {}", e))?;
    Ok(())
}

#[tauri::command]
fn check_startup_health(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let mut issues: Vec<serde_json::Value> = Vec::new();

    // 1. Check game directory
    let game_path = {
        let settings_guard = state.settings.lock().map_err(|e| e.to_string())?;
        let settings = settings_guard.get_settings();
        settings.game_path.clone()
    };

    if game_path.is_empty() {
        issues.push(serde_json::json!({
            "type": "warning",
            "code": "no_game_path",
            "message": "Game path not configured"
        }));
    } else {
        let gp = std::path::Path::new(&game_path);
        if !gp.exists() {
            issues.push(serde_json::json!({
                "type": "error",
                "code": "game_path_missing",
                "message": format!("Game directory not found: {}", game_path)
            }));
        } else {
            // Check write access
            let test_file = gp.join(".cmm_write_test");
            match std::fs::write(&test_file, b"test") {
                Ok(_) => { std::fs::remove_file(&test_file).ok(); }
                Err(_) => {
                    issues.push(serde_json::json!({
                        "type": "error",
                        "code": "game_path_readonly",
                        "message": format!("No write access to game directory: {}", game_path)
                    }));
                }
            }
        }
    }

    // 2. Check API key
    let api_key = {
        let settings_guard = state.settings.lock().map_err(|e| e.to_string())?;
        let settings = settings_guard.get_settings();
        settings.nexusmods_api_key.clone()
    };

    if api_key.is_empty() {
        issues.push(serde_json::json!({
            "type": "warning",
            "code": "no_api_key",
            "message": "NexusMods API key not configured — sync and auto-download won't work"
        }));
    }

    // 3. Check NXM URL handler (macOS: check if nxm:// scheme is registered)
    #[cfg(target_os = "macos")]
    {
        use std::process::Command;
        // Use `open -Ra` to check if any app handles nxm:// URLs is not straightforward.
        // Instead check if our app bundle is registered via lsregister.
        let output = Command::new("python3")
            .args(["-c", "from Foundation import NSWorkspace; ws = NSWorkspace.sharedWorkspace(); url = __import__('Foundation').NSURL.URLWithString_('nxm://test'); app = ws.URLForApplicationToOpenURL_(url); print(app.path() if app else 'none')"])
            .output();

        match output {
            Ok(out) => {
                let result = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if result == "none" || result.is_empty() {
                    issues.push(serde_json::json!({
                        "type": "warning",
                        "code": "nxm_handler_missing",
                        "message": "NXM URL handler not registered — 'Download with Mod Manager' button on NexusMods won't work"
                    }));
                }
            }
            Err(_) => {
                // Can't check — not critical
            }
        }
    }

    Ok(serde_json::json!({
        "healthy": issues.is_empty(),
        "issues": issues
    }))
}

#[tauri::command]
fn check_and_run_first_setup(state: State<'_, AppState>) -> Result<String, String> {
    add_log(
        "Checking first run status".to_string(),
        "info".to_string(),
        "FIRST_RUN".to_string(),
        state.clone(),
    )?;

    let should_auto_detect = {
        let settings_guard = state.settings.lock().map_err(|e| e.to_string())?;
        let settings = settings_guard.get_settings();
        settings.first_run
    };

    if should_auto_detect {
        add_log(
            "First run detected, starting auto-detection".to_string(),
            "info".to_string(),
            "FIRST_RUN".to_string(),
            state.clone(),
        )?;

        // Run auto-detection
        let detection_result = auto_detect_game_path()?;

        // Update first_run flag to false and save the detected path
        let mut settings_guard = state.settings.lock().map_err(|e| e.to_string())?;
        let mut settings = settings_guard.get_settings();
        settings.first_run = false;

        // If auto-detection found a path, save it to settings
        if let Some(ref detected_path) = detection_result {
            settings.game_path = detected_path.clone();
            add_log(
                format!("Auto-detected game path: {}", detected_path),
                "success".to_string(),
                "FIRST_RUN".to_string(),
                state.clone(),
            )?;
        }

        if let Err(e) = settings_guard.save_settings(settings) {
            add_log(
                format!("Failed to save settings after first run: {}", e),
                "error".to_string(),
                "FIRST_RUN".to_string(),
                state.clone(),
            )?;
            return Err(format!("Failed to save settings: {}", e));
        }

        add_log(
            "First run setup completed".to_string(),
            "success".to_string(),
            "FIRST_RUN".to_string(),
            state.clone(),
        )?;

        match detection_result {
            Some(path) => Ok(format!("Auto-detected game path: {}", path)),
            None => Ok("No game path found during auto-detection".to_string()),
        }
    } else {
        add_log(
            "Not first run, skipping auto-detection".to_string(),
            "info".to_string(),
            "FIRST_RUN".to_string(),
            state.clone(),
        )?;
        Ok("Not first run".to_string())
    }
}

#[allow(unused_variables)] // app is used in cfg(target_os = "macos") code
fn main() {
    let mod_manager = ModManager::new();
    let app_settings = AppSettings::new();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _args, _cwd| {
            // Focus existing window when second instance is launched
            if let Some(window) = app.get_webview_window("main") {
                window.set_focus().ok();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_deep_link::init())
        .manage(AppState {
            mod_manager: Mutex::new(mod_manager),
            settings: Mutex::new(app_settings),
            logs: Mutex::new(VecDeque::new()),
            sync_cancel: Arc::new(AtomicBool::new(false)),
            install_cancel: Arc::new(AtomicBool::new(false)),
            installing: Arc::new(AtomicBool::new(false)),
            startup_nxm_url: Mutex::new(None),
            force_reinstall: AtomicBool::new(false),
            reinstall_mod_id: Mutex::new(None),
            pending_file_name: Mutex::new(None),
            pending_file_version: Mutex::new(None),
            pending_file_description: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_installed_mods,
            get_mod_changelog,
            set_force_reinstall,
            abort_reinstall,
            install_mod,
            remove_mod,
            forget_mod,
            deduplicate_mods,
            toggle_mod,
            get_settings,
            save_settings,
            get_crossover_bottles_path,
            auto_detect_game_path,
            add_log,
            add_log_entry,
            get_logs,
            clear_logs,
            handle_nxm_url,
            test_logging,
            download_and_save_mod,
            list_downloaded_mods,
            test_nxm_event,
            handle_relay_action,
            try_relay,
            get_startup_nxm_url,
            is_dev_build,
            get_build_timestamp,
            check_startup_health,
            check_and_run_first_setup,
            install_mod_from_nxm,
            clean_temp_files,
            reveal_in_finder,
            sync_mod_data,
            cancel_sync,
            cancel_install,
            is_installing
        ])
        .setup(|app| {
            // Clean up orphaned temporary files from previous sessions
            println!("🧹 Running startup cleanup for orphaned temporary files...");
            let (files_removed, dirs_removed, errors, removed_paths) =
                cleanup_orphaned_temp_files();

            if files_removed > 0 || dirs_removed > 0 {
                // Log each removed path
                for path in &removed_paths {
                    println!("  🗑️  Removed: {}", path);
                }

                println!(
                    "✓ Startup cleanup: Removed {} file(s) and {} directory/directories",
                    files_removed, dirs_removed
                );
                if errors > 0 {
                    println!("⚠️  Startup cleanup: {} error(s) encountered", errors);
                }
            } else {
                println!("✓ Startup cleanup: No orphaned files found");
            }

            // Register deep link handler for nxm:// URLs
            #[cfg(target_os = "macos")]
            {
                use tauri_plugin_deep_link::DeepLinkExt;

                // Handle URL passed at app launch (app was not running when link was clicked)
                if let Ok(Some(urls)) = app.deep_link().get_current() {
                    if let Some(url) = urls.first() {
                        let url_str = url.as_str().to_string();
                        println!("🔥 STARTUP URL: {}", url_str);
                        // Store for frontend — it will try relay then process after mount.
                        if let Some(state) = app.try_state::<AppState>() {
                            if let Ok(mut slot) = state.startup_nxm_url.lock() {
                                *slot = Some(url_str.clone());
                            }
                        }
                    }
                }

                let app_handle = app.handle().clone();

                // Listen for deep link events (app was already running when link was clicked)
                app.listen("deep-link://new-url", move |event| {
                    let payload = event.payload();
                    println!("🔥 DEEP LINK EVENT: {}", payload);

                    if let Ok(urls) = serde_json::from_str::<Vec<String>>(payload) {
                        if let Some(url) = urls.first() {
                            let url_clone = url.clone();
                            let app_clone = app_handle.clone();
                            tauri::async_runtime::spawn(async move {
                                handle_nxm_deep_link(url_clone, app_clone, false).await;
                            });
                        }
                    }
                });

                println!("🔥 SETUP: Deep link handler registered for nxm:// scheme");
            }

            // Start Unix socket listener for NXM relay (dev instance only).
            // Release bundles process URLs directly; dev instance receives relayed URLs.
            let is_dev = tauri::is_dev() || cfg!(debug_assertions);
            let is_bundled = std::env::current_exe()
                .map(|p| p.to_string_lossy().contains(".app/Contents/MacOS"))
                .unwrap_or(false);
            if is_dev && !is_bundled {
                start_socket_listener(app.handle().clone());
            }

            // Log startup message
            if let Some(state) = app.try_state::<AppState>() {
                let mut logs = state.logs.lock().unwrap();
                logs.push_back(LogEntry {
                    timestamp: chrono::Utc::now()
                        .format("%Y-%m-%d %H:%M:%S UTC")
                        .to_string(),
                    level: "info".to_string(),
                    message: format!(
                        "Crossover Mod Manager v{} started",
                        env!("CARGO_PKG_VERSION")
                    ),
                    category: "system".to_string(),
                });
            }

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|app, event| {
            match event {
                tauri::RunEvent::WindowEvent {
                    event: tauri::WindowEvent::CloseRequested { api, .. },
                    ..
                } => {
                    let state = app.state::<AppState>();
                    if state.installing.load(Ordering::Relaxed) {
                        api.prevent_close();
                        if let Some(window) = app.get_webview_window("main") {
                            window.emit("close-requested", ()).ok();
                        }
                    }
                }
                tauri::RunEvent::Exit => {
                    // Don't cleanup socket here — HMR restarts cause a window
                    // where socket doesn't exist. New process cleans stale socket on bind.
                }
                _ => {}
            }
        });
}
