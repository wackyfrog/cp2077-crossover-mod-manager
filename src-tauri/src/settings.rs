use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub game_path: String,
    pub mod_storage_path: String,
    #[serde(default = "default_first_run")]
    pub first_run: bool,
    #[serde(default)]
    pub nexusmods_api_key: String,
    #[serde(default = "default_true")]
    pub show_splash: bool,
}

fn default_true() -> bool {
    true
}

fn default_first_run() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        // Default mod storage to a "Mods" folder in the user's Downloads directory
        let default_mod_path = dirs::download_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("CrossoverModManager")
            .join("Mods")
            .to_string_lossy()
            .to_string();

        Self {
            game_path: String::new(),
            mod_storage_path: default_mod_path,
            first_run: true,
            nexusmods_api_key: String::new(),
            show_splash: true,
        }
    }
}

pub struct AppSettings {
    settings_path: PathBuf,
    settings: Settings,
}

impl AppSettings {
    pub fn new() -> Self {
        let settings_path = Self::get_settings_path();
        let settings = Self::load_settings(&settings_path);

        Self {
            settings_path,
            settings,
        }
    }

    fn get_settings_path() -> PathBuf {
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
        let app_dir = home.join(".crossover-mod-manager");

        if !app_dir.exists() {
            fs::create_dir_all(&app_dir).ok();
        }

        app_dir.join("settings.json")
    }

    fn load_settings(path: &PathBuf) -> Settings {
        if path.exists() {
            if let Ok(content) = fs::read_to_string(path) {
                if let Ok(settings) = serde_json::from_str::<Settings>(&content) {
                    return settings;
                }
            }
        }
        Settings::default()
    }

    pub fn get_settings(&self) -> Settings {
        self.settings.clone()
    }

    pub fn save_settings(&mut self, settings: Settings) -> Result<(), String> {
        let json = serde_json::to_string_pretty(&settings)
            .map_err(|e| format!("Failed to serialize settings: {}", e))?;

        fs::write(&self.settings_path, json)
            .map_err(|e| format!("Failed to write settings: {}", e))?;

        self.settings = settings;
        Ok(())
    }
}
