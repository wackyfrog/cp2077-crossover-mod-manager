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
    /// The mod's title on Nexus now; authors rename it with new versions.
    pub name: Option<String>,
    pub version: String,
    pub summary: Option<String>,
    pub picture_url: Option<String>,
    /// "D Mon YYYY", the format the UI already shows.
    pub nexus_updated_at: Option<String>,
    /// (member id, account name) of whoever uploaded the mod.
    pub uploader: Option<(u64, String)>,
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
    /// (old file id, new file id) pairs the author declared on Nexus. Only
    /// v1 has them: filled by the v1 fallback, or by `fetch_file_updates`
    /// for mods whose update target the files alone can't settle.
    pub file_updates: Vec<(u64, u64)>,
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

/// What NETRUN concludes about one installed record.
#[derive(Debug, PartialEq)]
pub struct InstalledState {
    pub update_available: bool,
    /// Set when the record's version must be replaced by the installed file's.
    pub corrected_version: Option<String>,
}

/// Whether a record is out of date, from the mod's declared version and from
/// the installed file itself.
///
/// When an author releases a new version they move the old file to
/// OLD_VERSION (or ARCHIVED). A record whose installed file sits there is out
/// of date whatever the version numbers say — authors often leave the mod's
/// version behind. It also means the file, not the record, says what's on
/// disk: earlier releases of this app recorded the mod's latest version after
/// an update that had reinstalled the old file, so that version is corrected.
/// Files still listed as MAIN or OPTIONAL are left alone — an optional part
/// the author never touched legitimately carries an older version than the mod.
pub fn installed_state(
    files: &[NexusFile],
    installed_file_id: Option<&str>,
    recorded_version: &str,
    latest_version: &str,
    is_newer: impl Fn(&str, &str) -> bool,
) -> InstalledState {
    let installed = installed_file_id
        .map(str::trim)
        .and_then(|fid| files.iter().find(|f| f.file_id.to_string() == fid));
    let retired = installed.is_some_and(|f| matches!(f.category.as_deref(), Some("OLD_VERSION" | "ARCHIVED")));
    let corrected_version = installed
        .filter(|_| retired)
        .and_then(|f| f.version.as_deref())
        .map(str::trim)
        .filter(|v| !v.is_empty() && *v != recorded_version.trim())
        .map(String::from);
    let version = corrected_version.as_deref().unwrap_or(recorded_version);
    InstalledState { update_available: retired || is_newer(latest_version, version), corrected_version }
}

/// The file Update should download for a record, if one is clear.
///
/// The author's word comes first: a live file down the replacement chain
/// Nexus keeps (`file_updates`, see `declared_successor`). Without one, the
/// newest live file with the installed file's name,
/// uploaded after it. Authors who rename the file with each release (the
/// version in the name, or a new name altogether — SPLAT went from "Splat
/// Physics" to "SPLAT Physics Realistic Ragdoll Overhaul") never have one,
/// and asking for the installed file just reinstalls it. Then, when exactly
/// one MAIN file was uploaded after the installed one, that is the successor
/// — but only for a mod installed as a single record (`sole_record`): with
/// several parts installed, a renamed part can't be told from the core
/// (LUT Switcher's "Nova LUT Pack" would get the core "LUTSwitcher" file).
/// Anything else is ambiguous (`None`), and Update opens the Files tab.
///
/// Files the author retired (OLD_VERSION, ARCHIVED, DELETED) are never a
/// target: installing one leaves the mod OUTDATED, and Update would point
/// at it again (Nova LUT uploaded its 4.0.0s switcher pack as OLD_VERSION).
pub fn update_target(
    files: &[NexusFile],
    file_updates: &[(u64, u64)],
    installed_file_id: Option<&str>,
    sole_record: bool,
) -> Option<String> {
    let installed = installed_file_id
        .map(str::trim)
        .and_then(|fid| files.iter().find(|f| f.file_id.to_string() == fid))?;
    if let Some(declared) = declared_successor(files, file_updates, installed.file_id) {
        return Some(declared.to_string());
    }
    let since = installed.uploaded?;
    let later: Vec<&NexusFile> = files
        .iter()
        .filter(|f| f.uploaded.is_some_and(|t| t > since))
        .filter(|f| !is_retired(f))
        .collect();
    if let Some(same) = later
        .iter()
        .filter(|f| f.name.is_some() && f.name == installed.name)
        .max_by_key(|f| f.uploaded)
    {
        return Some(same.file_id.to_string());
    }
    match later.iter().filter(|f| f.category.as_deref() == Some("MAIN")).collect::<Vec<_>>()[..] {
        [only] if sole_record => Some(only.file_id.to_string()),
        _ => None,
    }
}

fn is_retired(f: &NexusFile) -> bool {
    matches!(f.category.as_deref(), Some("OLD_VERSION" | "ARCHIVED" | "DELETED"))
}

/// The furthest live file down the author's replacement chain from
/// `installed`. The chain can pass through retired files (Nova LUT 2 → 3 → 4)
/// and can end in one (the Nova LUT switcher pack 3.0.0s → 4.0.0s, itself
/// OLD_VERSION after the pack moved to LUT Switcher) — then there's nothing
/// to update to. Unlike the rename rule it names one file for one file, so it
/// holds for mods installed as several records.
fn declared_successor(files: &[NexusFile], file_updates: &[(u64, u64)], installed: u64) -> Option<u64> {
    let mut seen = std::collections::HashSet::from([installed]);
    let mut current = installed;
    let mut live = None;
    while let Some(&(_, next)) = file_updates.iter().find(|(old, _)| *old == current) {
        if !seen.insert(next) {
            break;
        }
        if files.iter().any(|f| f.file_id == next && !is_retired(f)) {
            live = Some(next);
        }
        current = next;
    }
    live
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
        name: Option<String>,
        version: Option<String>,
        summary: Option<String>,
        picture_url: Option<String>,
        updated_at: Option<String>,
        uploader: Option<Uploader>,
    }
    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Uploader {
        member_id: u64,
        name: String,
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
                  nodes { modId name version summary pictureUrl updatedAt uploader { memberId name } } } }",
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
                    name: node.name.filter(|n| !n.trim().is_empty()),
                    version,
                    summary: node.summary,
                    picture_url: node.picture_url,
                    nexus_updated_at: node.updated_at.as_deref().and_then(format_iso_date),
                    uploader: node.uploader.map(|u| (u.member_id, u.name)),
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
                file_updates: Vec::new(),
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

#[derive(Deserialize)]
struct V1FileUpdate {
    old_file_id: u64,
    new_file_id: u64,
}

/// The author's "this file replaces that one" links for one mod — one v1
/// request. GraphQL doesn't carry them (no such field on `ModFile`).
pub async fn fetch_file_updates(client: &reqwest::Client, api_key: &str, id: u64) -> Result<Vec<(u64, u64)>, FetchError> {
    #[derive(Deserialize)]
    struct V1Updates {
        #[serde(default)]
        file_updates: Vec<V1FileUpdate>,
    }
    let url = format!("https://api.nexusmods.com/v1/games/{}/mods/{}/files.json", GAME_DOMAIN, id);
    let (json, _) = v1_get(client, api_key, &url).await?;
    let parsed: V1Updates = serde_json::from_value(json).map_err(|e| FetchError::Other(format!("File list unreadable: {}", e)))?;
    Ok(parsed.file_updates.into_iter().map(|u| (u.old_file_id, u.new_file_id)).collect())
}

async fn fetch_one_v1(
    client: &reqwest::Client,
    api_key: &str,
    id: u64,
) -> Result<(ModSnapshot, Remaining), FetchError> {
    #[derive(Deserialize)]
    struct V1Mod {
        name: Option<String>,
        version: Option<String>,
        summary: Option<String>,
        picture_url: Option<String>,
        updated_timestamp: Option<i64>,
        user: Option<V1User>,
    }
    #[derive(Deserialize)]
    struct V1User {
        member_id: u64,
        name: String,
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
        #[serde(default)]
        file_updates: Vec<V1FileUpdate>,
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
                name: m.name.filter(|n| !n.trim().is_empty()),
                version,
                summary: m.summary,
                picture_url: m.picture_url,
                nexus_updated_at: m.updated_timestamp.and_then(format_timestamp_day),
                uploader: m.user.map(|u| (u.member_id, u.name)),
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
            file_updates: f.file_updates.iter().map(|u| (u.old_file_id, u.new_file_id)).collect(),
        },
        remaining,
    ))
}

// ── Changelog ───────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
pub struct VersionNotes {
    pub lines: Vec<String>,
    pub date: Option<String>,
    /// Unix seconds of the version's first upload. Version numbers on Nexus
    /// don't order reliably ("0.65" vs "0.7"), upload times do. Absent in
    /// caches written before it existed, until the next NETRUN.
    #[serde(default)]
    pub uploaded: Option<i64>,
    /// The file's description, only when there are no `lines` and it isn't a
    /// repeat of an older version's (see `build_changelog`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// One entry per version, from the files that carry it. Archived and deleted
/// files are hidden on Nexus; they only count when their author wrote notes.
///
/// Some authors never fill in the changelog and describe each upload in the
/// file's description instead, so a version without notes carries its file
/// description — shown apart, since about half of such descriptions are
/// install instructions or image links rather than notes. A description that
/// repeats one already seen on an older version is static text and is left
/// out.
fn build_changelog(files: &[NexusFile]) -> BTreeMap<String, VersionNotes> {
    #[derive(Default)]
    struct Acc {
        uploaded: Option<i64>,
        lines: Vec<String>,
        description: Option<String>,
    }
    let mut ordered: Vec<&NexusFile> = files.iter().collect();
    ordered.sort_by_key(|f| f.uploaded.unwrap_or(i64::MAX));

    let mut by_version: BTreeMap<String, Acc> = BTreeMap::new();
    for f in ordered {
        let Some(ver) = f.version.as_deref().map(str::trim).filter(|v| !v.is_empty()) else { continue };
        let hidden = matches!(f.category.as_deref(), Some("ARCHIVED" | "DELETED"));
        if hidden && f.changelog.is_empty() {
            continue;
        }
        let entry = by_version.entry(ver.to_string()).or_default();
        // Several files can share a version (multi-part mods): the first
        // upload dates it, the first one with notes speaks for it.
        entry.uploaded = entry.uploaded.or(f.uploaded);
        if entry.lines.is_empty() {
            entry.lines = f.changelog.clone();
        }
        if entry.description.is_none() {
            entry.description = f.description.as_deref().map(str::trim).filter(|d| !d.is_empty()).map(String::from);
        }
    }

    // Oldest first, so a repeated description stays on the version that
    // introduced it
    let mut chronological: Vec<(&String, &mut Acc)> = by_version.iter_mut().collect();
    chronological.sort_by_key(|(_, acc)| acc.uploaded.unwrap_or(i64::MAX));
    let mut seen = std::collections::HashSet::new();
    for (_, acc) in chronological {
        let Some(desc) = acc.description.take() else { continue };
        let key = desc.split_whitespace().collect::<Vec<_>>().join(" ");
        if seen.insert(key) && acc.lines.is_empty() {
            acc.description = Some(desc);
        }
    }

    by_version
        .into_iter()
        .map(|(ver, acc)| {
            let date = acc
                .uploaded
                .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
                .map(|d| d.format("%d %b %Y").to_string());
            (ver, VersionNotes { lines: acc.lines, date, uploaded: acc.uploaded, description: acc.description })
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

    fn described(version: &str, uploaded: i64, lines: &[&str], description: &str) -> NexusFile {
        NexusFile { description: Some(description.into()), ..file(version, "MAIN", uploaded, lines) }
    }

    #[test]
    fn description_stands_in_only_where_notes_are_missing_and_not_repeated() {
        let log = build_changelog(&[
            described("1.0", 100, &[], "Extract to your Cyberpunk folder."),
            described("1.1", 200, &[], "- Fixed a bug"),
            described("1.2", 300, &[], "Extract to your  Cyberpunk folder."),
            described("1.3", 400, &["Real notes"], "- Something else"),
        ]);
        assert_eq!(log["1.0"].description.as_deref(), Some("Extract to your Cyberpunk folder."));
        assert_eq!(log["1.1"].description.as_deref(), Some("- Fixed a bug"));
        // same text as 1.0 once whitespace is folded: static, dropped
        assert_eq!(log["1.2"].description, None);
        // has notes of its own
        assert_eq!(log["1.3"].description, None);
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
        assert_eq!(log["3.1.2"].uploaded, Some(1788110007));
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

    fn state(category: &str, file_version: &str, recorded: &str, latest: &str) -> InstalledState {
        let mut f = file(file_version, category, 1, &[]);
        f.file_id = 7;
        // stand-in for main.rs's is_newer_version: plain numeric-dot compare
        let newer = |a: &str, b: &str| {
            let p = |s: &str| s.split('.').map(|x| x.parse::<u64>().unwrap_or(0)).collect::<Vec<_>>();
            p(a) > p(b)
        };
        installed_state(&[f], Some("7"), recorded, latest, newer)
    }

    #[test]
    fn a_retired_file_is_outdated_and_corrects_a_version_recorded_by_mistake() {
        // Native Interactions Framework: 1.0.5a reinstalled, recorded as 1.1.3
        let s = state("OLD_VERSION", "1.0.5a", "1.1.3", "1.1.3");
        assert!(s.update_available);
        assert_eq!(s.corrected_version.as_deref(), Some("1.0.5a"));
    }

    #[test]
    fn a_retired_file_is_outdated_even_when_versions_agree() {
        let s = state("ARCHIVED", "1.0.1", "1.0.1", "1.0.1");
        assert_eq!(s, InstalledState { update_available: true, corrected_version: None });
    }

    #[test]
    fn a_live_optional_part_with_an_older_version_is_left_alone() {
        // Dynamic Appearances: part file 0.4.0 under mod version 0.7
        let s = state("OPTIONAL", "0.4.0", "0.7", "0.7");
        assert_eq!(s, InstalledState { update_available: false, corrected_version: None });
    }

    #[test]
    fn a_live_file_follows_the_declared_version() {
        assert!(state("MAIN", "2.0", "2.0", "2.1").update_available);
        assert!(!state("MAIN", "2.1", "2.1", "2.1").update_available);
    }

    fn named(id: u64, name: &str, category: &str, uploaded: i64) -> NexusFile {
        NexusFile { file_id: id, name: Some(name.into()), ..file("1", category, uploaded, &[]) }
    }

    #[test]
    fn update_target_prefers_a_newer_file_with_the_same_name() {
        let files = [
            named(1, "Core", "OLD_VERSION", 100),
            named(2, "Core", "MAIN", 200),
            named(3, "Addon", "MAIN", 300),
        ];
        assert_eq!(update_target(&files, &[], Some("1"), true).as_deref(), Some("2"));
    }

    #[test]
    fn update_target_follows_a_rename_when_one_main_file_came_after() {
        // SPLAT: installed "Splat Physics", renamed later; one MAIN left
        let files = [
            named(152859, "Splat Physics", "OLD_VERSION", 100),
            named(158554, "SPLAT Physics Realistic Ragdoll Overhaul", "ARCHIVED", 200),
            named(158573, "SPLAT Physics Realistic Ragdoll Overhaul", "MAIN", 300),
        ];
        assert_eq!(update_target(&files, &[], Some("152859"), true).as_deref(), Some("158573"));
    }

    #[test]
    fn update_target_gives_up_when_the_successor_is_ambiguous() {
        let files = [
            named(1, "Mod 1.0", "OLD_VERSION", 100),
            named(2, "Mod 2.0 Lite", "MAIN", 200),
            named(3, "Mod 2.0 Full", "MAIN", 300),
        ];
        assert_eq!(update_target(&files, &[], Some("1"), true), None);
        // nothing after the installed file, or the file unknown
        assert_eq!(update_target(&files, &[], Some("3"), true), None);
        assert_eq!(update_target(&files, &[], Some("99"), true), None);
    }

    #[test]
    fn update_target_skips_files_the_author_retired() {
        // Nova LUT: the 4.0.0s switcher pack went up as OLD_VERSION
        let files = [
            named(105480, "Nova LUT - LUT Switcher Pack", "OLD_VERSION", 100),
            named(144813, "Nova LUT 4", "MAIN", 200),
            named(144850, "Nova LUT - LUT Switcher Pack", "OLD_VERSION", 201),
        ];
        assert_eq!(update_target(&files, &[], Some("105480"), true).as_deref(), Some("144813"));
        assert_eq!(update_target(&files, &[], Some("105480"), false), None);
    }

    #[test]
    fn update_target_follows_a_rename_only_when_the_mod_has_one_record() {
        // LUT Switcher: a pack renamed later must not get the core file
        let files = [
            named(144848, "LUT Switcher - Nova LUT Pack", "OLD_VERSION", 100),
            named(146114, "LUT Switcher 2 - Core", "OLD_VERSION", 200),
            named(154976, "LUT Pack - Nova LUT", "OPTIONAL", 300),
            named(157888, "LUTSwitcher", "MAIN", 400),
        ];
        assert_eq!(update_target(&files, &[], Some("144848"), false), None);
        assert_eq!(update_target(&files, &[], Some("146114"), false), None);
        assert_eq!(update_target(&files, &[], Some("146114"), true).as_deref(), Some("157888"));
    }

    #[test]
    fn update_target_follows_the_chain_the_author_declared() {
        // Nova LUT, 25.09: 3 and 4 have different names, and the mod is
        // installed as several records — only the chain names 4 for 3
        let files = [
            named(83367, "Nova LUT 2", "OLD_VERSION", 100),
            named(105479, "Nova LUT 3", "OLD_VERSION", 200),
            named(105480, "Nova LUT - LUT Switcher Pack", "OLD_VERSION", 201),
            named(144813, "Nova LUT 4", "MAIN", 300),
            named(144850, "Nova LUT - LUT Switcher Pack", "OLD_VERSION", 301),
        ];
        let chain = [(83367, 105479), (105479, 144813), (105480, 144850)];
        assert_eq!(update_target(&files, &[], Some("105479"), false), None);
        assert_eq!(update_target(&files, &chain, Some("105479"), false).as_deref(), Some("144813"));
        // through a retired link to the live end
        assert_eq!(update_target(&files, &chain, Some("83367"), false).as_deref(), Some("144813"));
        // a chain that ends in a retired file names nothing
        assert_eq!(update_target(&files, &chain, Some("105480"), false), None);
        // and doesn't override the rules when it names nothing
        assert_eq!(update_target(&files, &chain, Some("105480"), true).as_deref(), Some("144813"));
    }

    #[test]
    fn update_target_survives_a_looping_chain() {
        let files = [named(1, "A", "OLD_VERSION", 100), named(2, "B", "OLD_VERSION", 200)];
        assert_eq!(update_target(&files, &[(1, 2), (2, 1)], Some("1"), false), None);
        let files = [named(1, "A", "OLD_VERSION", 100), named(2, "B", "MAIN", 200)];
        assert_eq!(update_target(&files, &[(1, 2), (2, 1)], Some("1"), false).as_deref(), Some("2"));
    }

    #[test]
    fn update_target_ignores_a_chain_to_a_file_nexus_no_longer_lists() {
        let files = [named(1, "A", "OLD_VERSION", 100), named(3, "C", "MAIN", 300)];
        assert_eq!(update_target(&files, &[(1, 2)], Some("1"), false), None);
    }

    #[test]
    fn dates_match_the_ui_format() {
        assert_eq!(format_iso_date("2026-08-31T20:25:11Z").as_deref(), Some("31 Aug 2026"));
        assert_eq!(format_timestamp_day(1788207911).as_deref(), Some("31 Aug 2026"));
        assert_eq!(format_iso_date("2026-09-03T00:00:00Z").as_deref(), Some("3 Sep 2026"));
    }
}
