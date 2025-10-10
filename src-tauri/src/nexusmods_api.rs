use reqwest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct NexusModsFile {
    #[serde(rename = "URI")]
    uri: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize)]
struct DownloadLink {
    #[serde(rename = "URI")]
    uri: String,
    name: String,
}

/// Get download URL for a mod file from NexusMods API
///
/// # Arguments
/// * `game_domain` - Game identifier (e.g., "cyberpunk2077")
/// * `mod_id` - The mod ID on NexusMods
/// * `file_id` - The specific file ID to download
/// * `api_key` - User's NexusMods API key
///
/// # Returns
/// * `Ok(String)` - The download URL
/// * `Err(String)` - Error message if API call fails
pub async fn get_download_url(
    game_domain: &str,
    mod_id: &str,
    file_id: &str,
    api_key: &str,
) -> Result<String, String> {
    if api_key.is_empty() {
        return Err(
            "NexusMods API key is not configured. Please add your API key in Settings.".to_string(),
        );
    }

    // NexusMods API endpoint for getting download links
    let url = format!(
        "https://api.nexusmods.com/v1/games/{}/mods/{}/files/{}/download_link.json",
        game_domain, mod_id, file_id
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("apikey", api_key)
        .header("User-Agent", "CrossoverModManager/1.1.0")
        .send()
        .await
        .map_err(|e| format!("Failed to connect to NexusMods API: {}", e))?;

    let status = response.status();

    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(
            "Invalid API key. Please check your NexusMods API key in Settings.".to_string(),
        );
    }

    if status == reqwest::StatusCode::NOT_FOUND {
        return Err(format!("Mod file not found (Mod ID: {}, File ID: {}). The file may have been removed or the IDs are incorrect.", mod_id, file_id));
    }

    if !status.is_success() {
        let error_text = response
            .text()
            .await
            .unwrap_or_else(|_| "Unknown error".to_string());
        return Err(format!("NexusMods API error ({}): {}", status, error_text));
    }

    // Parse response - it returns an array of download links
    let download_links: Vec<DownloadLink> = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse NexusMods API response: {}", e))?;

    // Get the first download link (usually CDN link)
    if let Some(link) = download_links.first() {
        Ok(link.uri.clone())
    } else {
        Err(
            "No download links available. You may need a Premium account for this file."
                .to_string(),
        )
    }
}

/// Validate NexusMods API key by making a test request
///
/// # Arguments
/// * `api_key` - User's NexusMods API key
///
/// # Returns
/// * `Ok(bool)` - true if API key is valid
/// * `Err(String)` - Error message if validation fails
pub async fn validate_api_key(api_key: &str) -> Result<bool, String> {
    if api_key.is_empty() {
        return Err("API key is empty".to_string());
    }

    // Use the validate endpoint
    let url = "https://api.nexusmods.com/v1/users/validate.json";

    let client = reqwest::Client::new();
    let response = client
        .get(url)
        .header("apikey", api_key)
        .header("User-Agent", "CrossoverModManager/1.1.0")
        .send()
        .await
        .map_err(|e| format!("Failed to connect to NexusMods API: {}", e))?;

    Ok(response.status().is_success())
}

/// Get mod info from NexusMods API
///
/// # Arguments
/// * `game_domain` - Game identifier (e.g., "cyberpunk2077")
/// * `mod_id` - The mod ID on NexusMods
/// * `api_key` - User's NexusMods API key
///
/// # Returns
/// * `Ok((String, String))` - Tuple of (mod_name, mod_version)
/// * `Err(String)` - Error message if API call fails
pub async fn get_mod_info(
    game_domain: &str,
    mod_id: &str,
    api_key: &str,
) -> Result<(String, String, String), String> {
    if api_key.is_empty() {
        return Ok((
            format!("Mod_{}", mod_id),
            "Unknown".to_string(),
            "Unknown".to_string(),
        ));
    }

    let url = format!(
        "https://api.nexusmods.com/v1/games/{}/mods/{}.json",
        game_domain, mod_id
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("apikey", api_key)
        .header("User-Agent", "CrossoverModManager/1.1.0")
        .send()
        .await
        .map_err(|e| format!("Failed to get mod info: {}", e))?;

    if !response.status().is_success() {
        return Ok((
            format!("Mod_{}", mod_id),
            "Unknown".to_string(),
            "Unknown".to_string(),
        ));
    }

    #[derive(Deserialize)]
    struct ModInfo {
        name: String,
        version: String,
        author: String,
    }

    let mod_info: ModInfo = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse mod info: {}", e))?;

    Ok((mod_info.name, mod_info.version, mod_info.author))
}
