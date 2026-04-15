use serde::{Deserialize, Serialize};

#[allow(dead_code)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionInfo {
    pub name: String,
    pub summary: String,
    pub author: String,
    pub total_downloads: u64,
    pub revision_number: u32,
    pub mod_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CollectionMod {
    pub mod_id: u64,
    pub file_id: u64,
    pub name: String,
    pub version: String,
    pub is_optional: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct CollectionDownloadResponse {
    pub mods: Vec<CollectionMod>,
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
#[allow(dead_code)]
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

#[allow(dead_code)]
pub struct ModDetails {
    pub name: String,
    pub version: String,
    pub author: String,
    pub summary: Option<String>,
    pub picture_url: Option<String>,
    pub nexus_updated_at: Option<String>,
}

/// Fetch full mod details (name, version, author, summary, picture) from Nexus API
pub async fn get_mod_details(
    game_domain: &str,
    mod_id: &str,
    api_key: &str,
) -> Result<ModDetails, String> {
    if api_key.is_empty() {
        return Err("API key not configured".to_string());
    }

    let url = format!(
        "https://api.nexusmods.com/v1/games/{}/mods/{}.json",
        game_domain, mod_id
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("apikey", api_key)
        .header("User-Agent", "CrossoverModManager/2.0")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if response.status() == reqwest::StatusCode::UNAUTHORIZED {
        return Err("Invalid API key".to_string());
    }
    if !response.status().is_success() {
        return Err(format!("Nexus API error: HTTP {}", response.status()));
    }

    #[derive(Deserialize)]
    struct NexusModResponse {
        name: String,
        version: String,
        author: String,
        summary: Option<String>,
        picture_url: Option<String>,
        updated_timestamp: Option<i64>,
    }

    let r: NexusModResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse mod details: {}", e))?;

    // Convert Unix timestamp to "DD Mon YYYY" string
    let nexus_updated_at = r.updated_timestamp.map(|ts| {
        use std::time::{Duration, UNIX_EPOCH};
        let d = UNIX_EPOCH + Duration::from_secs(ts as u64);
        let secs = d.duration_since(UNIX_EPOCH).unwrap_or_default().as_secs();
        // Simple date calculation (no external crate needed)
        let days = secs / 86400;
        let (year, month, day) = days_to_ymd(days);
        let months = ["Jan","Feb","Mar","Apr","May","Jun","Jul","Aug","Sep","Oct","Nov","Dec"];
        format!("{} {} {}", day, months[(month - 1) as usize], year)
    });

    Ok(ModDetails {
        name: r.name,
        version: r.version,
        author: r.author,
        summary: r.summary,
        picture_url: r.picture_url,
        nexus_updated_at,
    })
}

#[allow(dead_code)]
/// Fetch the latest file version for a mod (MAIN category) from Nexus API
pub async fn get_latest_file_version(
    game_domain: &str,
    mod_id: &str,
    api_key: &str,
) -> Result<Option<String>, String> {
    if api_key.is_empty() {
        return Ok(None);
    }

    let url = format!(
        "https://api.nexusmods.com/v1/games/{}/mods/{}/files.json",
        game_domain, mod_id
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("apikey", api_key)
        .header("User-Agent", "CrossoverModManager/2.0")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Ok(None);
    }

    #[derive(Deserialize)]
    struct NexusFile {
        version: Option<String>,
        category_name: Option<String>,
        file_id: u64,
        #[allow(dead_code)]
        name: Option<String>,
    }
    #[derive(Deserialize)]
    struct NexusFilesResponse {
        files: Vec<NexusFile>,
    }

    let r: NexusFilesResponse = response
        .json()
        .await
        .map_err(|_| "Failed to parse files response".to_string())?;

    // Find the latest MAIN file by highest semantic version, fallback to highest file_id
    let latest = r.files.iter()
        .filter(|f| f.category_name.as_deref() == Some("MAIN"))
        .max_by(|a, b| {
            let va = parse_version(a.version.as_deref().unwrap_or("0"));
            let vb = parse_version(b.version.as_deref().unwrap_or("0"));
            va.cmp(&vb).then(a.file_id.cmp(&b.file_id))
        });

    Ok(latest.and_then(|f| f.version.clone()))
}

/// Per-file info returned by get_file_names: (display_name, version, description)
pub type FileInfo = (String, Option<String>, Option<String>);

/// Fetch file info for all files of a mod. Returns map of file_id -> FileInfo.
pub async fn get_file_names(
    game_domain: &str,
    mod_id: &str,
    api_key: &str,
) -> Result<std::collections::HashMap<String, FileInfo>, String> {
    if api_key.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    let url = format!(
        "https://api.nexusmods.com/v1/games/{}/mods/{}/files.json",
        game_domain, mod_id
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("apikey", api_key)
        .header("User-Agent", "CrossoverModManager/2.0")
        .send()
        .await
        .map_err(|e| format!("Network error: {}", e))?;

    if !response.status().is_success() {
        return Ok(std::collections::HashMap::new());
    }

    #[derive(Deserialize)]
    struct NexusFile {
        file_id: u64,
        name: Option<String>,
        version: Option<String>,
        description: Option<String>,
    }
    #[derive(Deserialize)]
    struct NexusFilesResponse {
        files: Vec<NexusFile>,
    }

    let r: NexusFilesResponse = response
        .json()
        .await
        .map_err(|_| "Failed to parse files response".to_string())?;

    let mut map = std::collections::HashMap::new();
    for f in r.files {
        if let Some(name) = f.name {
            map.insert(f.file_id.to_string(), (name, f.version, f.description));
        }
    }
    Ok(map)
}

#[allow(dead_code)]
/// Parse version string into comparable numeric parts
fn parse_version(s: &str) -> Vec<u64> {
    s.split(|c: char| c == '.' || c == '-')
        .filter_map(|p| p.trim_start_matches('v').parse::<u64>().ok())
        .collect()
}

/// Convert days-since-epoch to (year, month, day)
fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    let mut year = 1970u64;
    loop {
        let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
        let days_in_year = if leap { 366 } else { 365 };
        if days < days_in_year { break; }
        days -= days_in_year;
        year += 1;
    }
    let leap = (year % 4 == 0 && year % 100 != 0) || year % 400 == 0;
    let days_in_month = [31u64, if leap { 29 } else { 28 }, 31,30,31,30,31,31,30,31,30,31];
    let mut month = 1u64;
    for &dim in &days_in_month {
        if days < dim { break; }
        days -= dim;
        month += 1;
    }
    (year, month, days + 1)
}

/// Get collection information from NexusMods API
///
/// # Arguments
/// * `game_domain` - Game identifier (e.g., "cyberpunk2077")
/// * `collection_id` - The collection ID on NexusMods
/// * `api_key` - User's NexusMods API key
///
/// # Returns
/// * `Ok(CollectionInfo)` - Collection information
/// * `Err(String)` - Error message if API call fails
pub async fn get_collection_info(
    game_domain: &str,
    collection_id: &str,
    api_key: &str,
) -> Result<CollectionInfo, String> {
    if api_key.is_empty() {
        return Err(
            "NexusMods API key is not configured. Please add your API key in Settings.".to_string(),
        );
    }

    let url = format!(
        "https://api.nexusmods.com/v1/games/{}/collections/{}.json",
        game_domain, collection_id
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("apikey", api_key)
        .header("User-Agent", "CrossoverModManager/1.1.0")
        .send()
        .await
        .map_err(|e| format!("Failed to get collection info: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to get collection info: HTTP {}",
            response.status()
        ));
    }

    let collection_info: CollectionInfo = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse collection info: {}", e))?;

    Ok(collection_info)
}

/// Get collection mods list from NexusMods API
///
/// # Arguments
/// * `game_domain` - Game identifier (e.g., "cyberpunk2077")
/// * `collection_id` - The collection ID on NexusMods
/// * `revision_number` - The collection revision number
/// * `api_key` - User's NexusMods API key
///
/// # Returns
/// * `Ok(Vec<CollectionMod>)` - List of mods in the collection
/// * `Err(String)` - Error message if API call fails
pub async fn get_collection_mods(
    game_domain: &str,
    collection_id: &str,
    revision_number: u32,
    api_key: &str,
) -> Result<Vec<CollectionMod>, String> {
    if api_key.is_empty() {
        return Err(
            "NexusMods API key is not configured. Please add your API key in Settings.".to_string(),
        );
    }

    let url = format!(
        "https://api.nexusmods.com/v1/games/{}/collections/{}/revisions/{}/download_links.json",
        game_domain, collection_id, revision_number
    );

    let client = reqwest::Client::new();
    let response = client
        .get(&url)
        .header("apikey", api_key)
        .header("User-Agent", "CrossoverModManager/1.1.0")
        .send()
        .await
        .map_err(|e| format!("Failed to get collection mods: {}", e))?;

    if !response.status().is_success() {
        return Err(format!(
            "Failed to get collection mods: HTTP {}",
            response.status()
        ));
    }

    let collection_response: CollectionDownloadResponse = response
        .json()
        .await
        .map_err(|e| format!("Failed to parse collection mods: {}", e))?;

    Ok(collection_response.mods)
}
