//! Fetches what NETRUN needs from NexusMods — latest version, summary,
//! picture, file names and the per-version changelog — in as few requests as
//! possible, and keeps the changelogs in a cache of their own.
//!
//! The GraphQL API (v2) answers for many mods at once: one `legacyModsByDomain`
//! request covers up to 80 mods' state, one aliased `modFiles` request covers a
//! batch of mods' files with their changelog text. A full NETRUN is about ten
//! requests instead of two per mod. If a GraphQL request fails for any reason
//! other than the rate limit, that batch falls back to the per-mod v1 REST API.
//!
//! A mod is only returned when every request for it succeeded, so callers can
//! write its version, files and changelog together or leave all of them alone.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::PathBuf;

const GRAPHQL_URL: &str = "https://api.nexusmods.com/v2/graphql";
const GAME_DOMAIN: &str = "cyberpunk2077";
/// Numeric game id GraphQL's `modFiles` wants (the one in staticdelivery URLs).
const GAME_ID: u32 = 3333;
const USER_AGENT: &str = "CrossoverModManager/2.0";
/// `legacyModsByDomain` returns at most 80 nodes per request; stay below it.
pub const BATCH_SIZE: usize = 50;
/// v1 fallback costs two requests per mod; stop before a mod would be left half-fetched.
const V1_REQUESTS_PER_MOD: i64 = 2;

pub struct ModState {
    pub version: String,
    pub summary: Option<String>,
    pub picture_url: Option<String>,
    /// "D Mon YYYY", the format the UI already shows.
    pub nexus_updated_at: Option<String>,
}

pub struct NexusFile {
    pub file_id: u64,
    pub name: Option<String>,
    pub version: Option<String>,
    pub description: Option<String>,
    pub category: Option<String>,
    pub uploaded: Option<i64>,
    pub changelog: Vec<String>,
}

pub struct ModSnapshot {
    pub state: ModState,
    pub files: Vec<NexusFile>,
}

impl ModSnapshot {
    /// file_id → (name, version, description), the shape `update_file_info` takes.
    pub fn file_info(&self) -> HashMap<String, crate::nexusmods_api::FileInfo> {
        self.files
            .iter()
            .filter_map(|f| {
                let name = f.name.clone()?;
                Some((f.file_id.to_string(), (name, f.version.clone(), f.description.clone())))
            })
            .collect()
    }

    pub fn changelog(&self) -> BTreeMap<String, VersionNotes> {
        build_changelog(&self.files)
    }
}

#[derive(Debug)]
pub enum FetchError {
    /// Nexus refused (429) or the v1 quota is too low to finish another mod.
    /// `reset` is the local time the hourly quota resets, when Nexus said so.
    RateLimited { reset: Option<String> },
    Other(String),
}

impl FetchError {
    pub fn message(&self) -> String {
        match self {
            FetchError::RateLimited { reset: Some(t) } => {
                format!("NexusMods request limit reached — resets at {}", t)
            }
            FetchError::RateLimited { reset: None } => "NexusMods request limit reached".to_string(),
            FetchError::Other(e) => e.clone(),
        }
    }
}

/// Outcome of fetching one batch: what arrived, which mods failed and why,
/// and whether the run has to stop here.
#[derive(Default)]
pub struct BatchResult {
    pub snapshots: HashMap<u64, ModSnapshot>,
    pub errors: HashMap<u64, String>,
    pub stop: Option<FetchError>,
    /// True when GraphQL failed and the batch went through v1 instead.
    pub used_fallback: Option<String>,
}

pub async fn fetch_batch(client: &reqwest::Client, api_key: &str, ids: &[u64]) -> BatchResult {
    match fetch_batch_graphql(client, api_key, ids).await {
        Ok(snapshots) => {
            let errors = ids
                .iter()
                .filter(|id| !snapshots.contains_key(id))
                .map(|id| (*id, "not found on NexusMods (hidden or removed?)".to_string()))
                .collect();
            BatchResult { snapshots, errors, ..Default::default() }
        }
        Err(e @ FetchError::RateLimited { .. }) => BatchResult {
            errors: ids.iter().map(|id| (*id, e.message())).collect(),
            stop: Some(e),
            ..Default::default()
        },
        Err(FetchError::Other(reason)) => {
            let mut result = fetch_batch_v1(client, api_key, ids).await;
            result.used_fallback = Some(reason);
            result
        }
    }
}

// ── GraphQL ─────────────────────────────────────────────────────────────

async fn graphql(
    client: &reqwest::Client,
    api_key: &str,
    body: serde_json::Value,
) -> Result<serde_json::Value, FetchError> {
    let resp = client
        .post(GRAPHQL_URL)
        .header("apikey", api_key)
        .header("User-Agent", USER_AGENT)
        .json(&body)
        .send()
        .await
        .map_err(|e| FetchError::Other(format!("GraphQL network error: {}", e)))?;

    let status = resp.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(FetchError::RateLimited { reset: reset_time(resp.headers()) });
    }
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(FetchError::Other("Invalid API key".to_string()));
    }
    if !status.is_success() {
        return Err(FetchError::Other(format!("GraphQL HTTP {}", status)));
    }

    let mut json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| FetchError::Other(format!("GraphQL response unreadable: {}", e)))?;
    let data = json.get_mut("data").map(serde_json::Value::take).unwrap_or_default();
    if data.is_null() {
        let errors = json.get("errors").map(|e| e.to_string()).unwrap_or_default();
        return Err(FetchError::Other(format!("GraphQL error: {}", errors)));
    }
    Ok(data)
}

async fn fetch_batch_graphql(
    client: &reqwest::Client,
    api_key: &str,
    ids: &[u64],
) -> Result<HashMap<u64, ModSnapshot>, FetchError> {
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Node {
        mod_id: u64,
        version: Option<String>,
        summary: Option<String>,
        picture_url: Option<String>,
        updated_at: Option<String>,
    }
    #[derive(Deserialize)]
    struct Nodes {
        nodes: Vec<Node>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct GqlFile {
        file_id: u64,
        name: Option<String>,
        version: Option<String>,
        description: Option<String>,
        category: Option<String>,
        date: Option<i64>,
        changelog_text: Option<Vec<String>>,
    }

    let refs: Vec<_> = ids
        .iter()
        .map(|id| serde_json::json!({ "gameDomain": GAME_DOMAIN, "modId": id }))
        .collect();
    let states = graphql(
        client,
        api_key,
        serde_json::json!({
            "query": "query($ids: [CompositeDomainWithIdInput!]!, $count: Int) { \
                legacyModsByDomain(ids: $ids, count: $count) { \
                  nodes { modId version summary pictureUrl updatedAt } } }",
            "variables": { "ids": refs, "count": ids.len() },
        }),
    )
    .await?;
    let states: Nodes = serde_json::from_value(states["legacyModsByDomain"].clone())
        .map_err(|e| FetchError::Other(format!("GraphQL mod list unreadable: {}", e)))?;

    let fields = "fileId name version description category date changelogText";
    let query = ids
        .iter()
        .map(|id| format!("m{id}: modFiles(modId: {id}, gameId: {GAME_ID}) {{ {fields} }}"))
        .collect::<Vec<_>>()
        .join(" ");
    let mut files = graphql(client, api_key, serde_json::json!({ "query": format!("{{ {} }}", query) })).await?;

    let mut out = HashMap::new();
    for node in states.nodes {
        let Some(version) = node.version.filter(|v| !v.is_empty()) else { continue };
        // A mod whose files alias came back null had an error of its own: skip
        // it rather than record it with no files.
        let Some(raw) = files.get_mut(format!("m{}", node.mod_id)).map(serde_json::Value::take) else { continue };
        let Ok(gql_files) = serde_json::from_value::<Vec<GqlFile>>(raw) else { continue };
        out.insert(
            node.mod_id,
            ModSnapshot {
                state: ModState {
                    version,
                    summary: node.summary,
                    picture_url: node.picture_url,
                    nexus_updated_at: node.updated_at.as_deref().and_then(format_iso_date),
                },
                files: gql_files
                    .into_iter()
                    .map(|f| NexusFile {
                        file_id: f.file_id,
                        name: f.name,
                        version: f.version,
                        description: f.description,
                        category: f.category,
                        uploaded: f.date,
                        changelog: f.changelog_text.unwrap_or_default(),
                    })
                    .collect(),
            },
        );
    }
    Ok(out)
}

// ── v1 REST fallback ────────────────────────────────────────────────────

async fn fetch_batch_v1(client: &reqwest::Client, api_key: &str, ids: &[u64]) -> BatchResult {
    let mut result = BatchResult::default();
    for (i, id) in ids.iter().enumerate() {
        match fetch_one_v1(client, api_key, *id).await {
            Ok((snapshot, remaining)) => {
                result.snapshots.insert(*id, snapshot);
                if let Some((_, reset)) = remaining.filter(|(left, _)| *left < V1_REQUESTS_PER_MOD) {
                    let stop = FetchError::RateLimited { reset };
                    for rest in &ids[i + 1..] {
                        result.errors.insert(*rest, stop.message());
                    }
                    result.stop = Some(stop);
                    break;
                }
            }
            Err(e @ FetchError::RateLimited { .. }) => {
                for rest in &ids[i..] {
                    result.errors.insert(*rest, e.message());
                }
                result.stop = Some(e);
                break;
            }
            Err(FetchError::Other(e)) => {
                result.errors.insert(*id, e);
            }
        }
    }
    result
}

type Remaining = Option<(i64, Option<String>)>;

async fn v1_get(
    client: &reqwest::Client,
    api_key: &str,
    url: &str,
) -> Result<(serde_json::Value, Remaining), FetchError> {
    let resp = client
        .get(url)
        .header("apikey", api_key)
        .header("User-Agent", USER_AGENT)
        .send()
        .await
        .map_err(|e| FetchError::Other(format!("Network error: {}", e)))?;
    let status = resp.status();
    if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
        return Err(FetchError::RateLimited { reset: reset_time(resp.headers()) });
    }
    if status == reqwest::StatusCode::UNAUTHORIZED {
        return Err(FetchError::Other("Invalid API key".to_string()));
    }
    if !status.is_success() {
        return Err(FetchError::Other(format!("Nexus API error: HTTP {}", status)));
    }
    let remaining = resp
        .headers()
        .get("x-rl-hourly-remaining")
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.parse::<i64>().ok())
        .map(|left| (left, reset_time(resp.headers())));
    let json = resp
        .json()
        .await
        .map_err(|e| FetchError::Other(format!("Nexus response unreadable: {}", e)))?;
    Ok((json, remaining))
}

async fn fetch_one_v1(
    client: &reqwest::Client,
    api_key: &str,
    id: u64,
) -> Result<(ModSnapshot, Remaining), FetchError> {
    #[derive(Deserialize)]
    struct V1Mod {
        version: Option<String>,
        summary: Option<String>,
        picture_url: Option<String>,
        updated_timestamp: Option<i64>,
    }
    #[derive(Deserialize)]
    struct V1File {
        file_id: u64,
        name: Option<String>,
        version: Option<String>,
        description: Option<String>,
        category_name: Option<String>,
        uploaded_timestamp: Option<i64>,
        changelog_html: Option<String>,
    }
    #[derive(Deserialize)]
    struct V1Files {
        files: Vec<V1File>,
    }

    let base = format!("https://api.nexusmods.com/v1/games/{}/mods/{}", GAME_DOMAIN, id);
    let (m, _) = v1_get(client, api_key, &format!("{}.json", base)).await?;
    let (f, remaining) = v1_get(client, api_key, &format!("{}/files.json", base)).await?;
    let m: V1Mod = serde_json::from_value(m).map_err(|e| FetchError::Other(format!("Mod details unreadable: {}", e)))?;
    let f: V1Files = serde_json::from_value(f).map_err(|e| FetchError::Other(format!("File list unreadable: {}", e)))?;
    let version = m.version.filter(|v| !v.is_empty()).ok_or_else(|| FetchError::Other("Nexus returned no version".to_string()))?;

    Ok((
        ModSnapshot {
            state: ModState {
                version,
                summary: m.summary,
                picture_url: m.picture_url,
                nexus_updated_at: m.updated_timestamp.and_then(format_timestamp_day),
            },
            files: f
                .files
                .into_iter()
                .map(|f| NexusFile {
                    file_id: f.file_id,
                    name: f.name,
                    version: f.version,
                    description: f.description,
                    category: f.category_name,
                    uploaded: f.uploaded_timestamp,
                    changelog: f.changelog_html.filter(|c| !c.trim().is_empty()).into_iter().collect(),
                })
                .collect(),
        },
        remaining,
    ))
}

// ── Changelog ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VersionNotes {
    pub lines: Vec<String>,
    pub date: Option<String>,
}

/// One entry per version, from the files that carry it. Archived and deleted
/// files are hidden on Nexus; they only count when their author wrote notes.
fn build_changelog(files: &[NexusFile]) -> BTreeMap<String, VersionNotes> {
    let mut by_version: BTreeMap<String, (Option<i64>, Vec<String>)> = BTreeMap::new();
    for f in files {
        let Some(ver) = f.version.as_deref().map(str::trim).filter(|v| !v.is_empty()) else { continue };
        let hidden = matches!(f.category.as_deref(), Some("ARCHIVED" | "DELETED"));
        if hidden && f.changelog.is_empty() {
            continue;
        }
        let entry = by_version.entry(ver.to_string()).or_insert((None, Vec::new()));
        // Several files can share a version (multi-part mods): first upload
        // dates it, the first one with notes speaks for it.
        entry.0 = match (entry.0, f.uploaded) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        };
        if entry.1.is_empty() {
            entry.1 = f.changelog.clone();
        }
    }
    by_version
        .into_iter()
        .map(|(ver, (ts, lines))| {
            let date = ts.and_then(|t| chrono::DateTime::from_timestamp(t, 0)).map(|d| d.format("%d %b %Y").to_string());
            (ver, VersionNotes { lines, date })
        })
        .collect()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct CachedChangelog {
    /// RFC 3339; when NETRUN (or a single-mod refresh) last fetched it.
    pub fetched_at: String,
    pub versions: BTreeMap<String, VersionNotes>,
}

/// Changelogs keyed by Nexus mod id, in `~/.crossover-mod-manager/changelogs.json`
/// — apart from `mods.json`, since the parts of one mod share a changelog and
/// the text would otherwise be stored once per part.
pub struct ChangelogCache {
    path: PathBuf,
    entries: HashMap<String, CachedChangelog>,
}

impl ChangelogCache {
    pub fn load() -> Self {
        let dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".crossover-mod-manager");
        fs::create_dir_all(&dir).ok();
        let path = dir.join("changelogs.json");
        let entries = fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        Self { path, entries }
    }

    pub fn get(&self, mod_id: &str) -> Option<&CachedChangelog> {
        self.entries.get(mod_id)
    }

    pub fn insert(&mut self, mod_id: &str, versions: BTreeMap<String, VersionNotes>) {
        self.entries.insert(
            mod_id.to_string(),
            CachedChangelog { fetched_at: chrono::Utc::now().to_rfc3339(), versions },
        );
    }

    /// Written to a temp file and renamed, so a crash mid-write can't leave
    /// half a cache behind.
    pub fn save(&self) -> Result<(), String> {
        let json = serde_json::to_string(&self.entries).map_err(|e| e.to_string())?;
        let tmp = self.path.with_extension("json.tmp");
        fs::write(&tmp, json).map_err(|e| format!("Failed to write changelog cache: {}", e))?;
        fs::rename(&tmp, &self.path).map_err(|e| format!("Failed to save changelog cache: {}", e))
    }
}

// ── Helpers ─────────────────────────────────────────────────────────────

/// `x-rl-hourly-reset` ("2026-09-23 19:00:00 +0000") as local "HH:MM".
fn reset_time(headers: &reqwest::header::HeaderMap) -> Option<String> {
    let raw = headers.get("x-rl-hourly-reset")?.to_str().ok()?;
    let t = chrono::DateTime::parse_from_str(raw, "%Y-%m-%d %H:%M:%S %z").ok()?;
    Some(t.with_timezone(&chrono::Local).format("%H:%M").to_string())
}

fn format_timestamp_day(ts: i64) -> Option<String> {
    chrono::DateTime::from_timestamp(ts, 0).map(|d| d.format("%-d %b %Y").to_string())
}

fn format_iso_date(iso: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(iso).ok().map(|d| d.format("%-d %b %Y").to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(version: &str, category: &str, uploaded: i64, lines: &[&str]) -> NexusFile {
        NexusFile {
            file_id: uploaded as u64,
            name: Some("main".into()),
            version: Some(version.into()),
            description: None,
            category: Some(category.into()),
            uploaded: Some(uploaded),
            changelog: lines.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn changelog_skips_silent_archived_files_but_keeps_listed_versions() {
        let log = build_changelog(&[
            file("2.1.0-alpha.1", "ARCHIVED", 1759587754, &[]),
            file("3.1.3", "MAIN", 1788207911, &[]),
            file("3.1.2", "OLD_VERSION", 1788110007, &["Crash fix on enemy spawns."]),
        ]);
        assert!(!log.contains_key("2.1.0-alpha.1"));
        assert_eq!(log["3.1.3"].lines, Vec::<String>::new());
        assert_eq!(log["3.1.2"].lines, vec!["Crash fix on enemy spawns.".to_string()]);
        assert_eq!(log["3.1.2"].date.as_deref(), Some("30 Aug 2026"));
    }

    #[test]
    fn changelog_merges_parts_sharing_a_version() {
        let log = build_changelog(&[
            file("1.0", "OPTIONAL", 200, &[]),
            file("1.0", "MAIN", 100, &["notes"]),
        ]);
        assert_eq!(log.len(), 1);
        assert_eq!(log["1.0"].lines, vec!["notes".to_string()]);
        assert_eq!(log["1.0"].date.as_deref(), Some("01 Jan 1970"));
    }

    #[test]
    fn dates_match_the_ui_format() {
        assert_eq!(format_iso_date("2026-08-31T20:25:11Z").as_deref(), Some("31 Aug 2026"));
        assert_eq!(format_timestamp_day(1788207911).as_deref(), Some("31 Aug 2026"));
        assert_eq!(format_iso_date("2026-09-03T00:00:00Z").as_deref(), Some("3 Sep 2026"));
    }
}
