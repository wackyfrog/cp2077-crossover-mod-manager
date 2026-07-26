//! Support for installing mods from a local archive on disk ("sideloading").
//!
//! Two independent helpers live here:
//!
//! * [`parse_nexus_filename`] — best-effort metadata recovery from a manually
//!   downloaded NexusMods archive name.
//! * [`find_content_root`] — locates the directory inside an extracted archive
//!   that should be treated as the game root, skipping a redundant wrapper
//!   folder.

use std::path::{Path, PathBuf};

/// Top-level directories a Cyberpunk 2077 mod archive may legitimately contain.
/// Matched case-insensitively — the same set `normalize_game_path` canonicalises.
pub const CANONICAL_DIRS: [&str; 6] = ["archive", "bin", "r6", "engine", "mods", "red4ext"];

/// Archive and Finder noise that must never influence wrapper detection.
pub fn is_ignorable(name: &str) -> bool {
    name == "__MACOSX" || name == ".DS_Store" || name.starts_with("._")
}

/// Metadata recovered from an archive filename. Every field except `name` is
/// optional — the caller is expected to show these as editable defaults, not to
/// trust them blindly.
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ParsedArchiveName {
    pub name: String,
    pub version: Option<String>,
    pub mod_id: Option<String>,
    pub file_stamp: Option<String>,
}

/// Recover mod metadata from a manual-download filename.
///
/// NexusMods names manual downloads roughly as
/// `{Name}-{modId}-{version with dashes}-{unixTimestamp}`, e.g.
/// `Jackie's Arch - All Versions White and Gold-1464-1-0-1612607650`.
///
/// This is a *heuristic*, not a documented contract. It parses from the right —
/// collecting the trailing run of purely numeric tokens — because a mod name may
/// itself contain both dashes and digits. A name ending in a number (e.g.
/// `Cyberpunk-2077-1464-1-0-...`) is genuinely ambiguous and will mis-assign the
/// mod id; that is why the UI asks the user to confirm the fields.
///
/// Anything that does not fit the shape degrades to `name = stem` with no other
/// fields, which is always safe.
pub fn parse_nexus_filename(stem: &str) -> ParsedArchiveName {
    let unparsed = || ParsedArchiveName {
        name: stem.trim().to_string(),
        version: None,
        mod_id: None,
        file_stamp: None,
    };

    let tokens: Vec<&str> = stem.split('-').collect();

    // Collect the trailing run of purely numeric tokens.
    let mut first_numeric = tokens.len();
    while first_numeric > 0 {
        let token = tokens[first_numeric - 1];
        if token.is_empty() || !token.chars().all(|c| c.is_ascii_digit()) {
            break;
        }
        first_numeric -= 1;
    }

    let trailing = &tokens[first_numeric..];

    // Need at least a timestamp plus one more token to say anything useful, and
    // at least one non-numeric token left over to serve as the name.
    if trailing.len() < 2 || first_numeric == 0 {
        return unparsed();
    }

    // The last token must look like a Unix timestamp, otherwise this is just a
    // name that happens to end in numbers.
    let stamp = trailing[trailing.len() - 1];
    let looks_like_timestamp = stamp.len() >= 9
        && stamp.len() <= 11
        && stamp.parse::<u64>().map(|n| n > 1_000_000_000).unwrap_or(false);
    if !looks_like_timestamp {
        return unparsed();
    }

    let name = tokens[..first_numeric].join("-").trim().to_string();
    if name.is_empty() {
        return unparsed();
    }

    // Everything between the name and the timestamp: mod id first, version parts after.
    let group = &trailing[..trailing.len() - 1];

    // A plausible version has at most a handful of parts. A longer run means the
    // numeric tokens most likely belong to the mod name, so refuse to guess an id
    // rather than record a wrong one.
    let (mod_id, version) = if group.len() > 5 {
        (None, None)
    } else {
        let id = group.first().map(|s| s.to_string());
        let version = if group.len() > 1 {
            Some(group[1..].join("."))
        } else {
            None
        };
        (id, version)
    };

    ParsedArchiveName {
        name,
        version,
        mod_id,
        file_stamp: Some(stamp.to_string()),
    }
}

/// Find the directory within an extracted archive whose contents should be
/// merged into the game root.
///
/// Many mods ship wrapped in a single folder named after the mod
/// (`Combat/r6/tweaks/...`). Installing that verbatim puts everything under
/// `{game}/Combat/`, where the game never looks — the mod is silently inert.
/// This walks past such wrappers until it finds a directory that either holds a
/// canonical game directory or is no longer a lone folder.
///
/// Descends only through directories that are the *sole* entry, so archives with
/// several top-level options are left untouched.
///
/// Crucially, a wrapper is only stripped once a canonical directory is actually
/// found underneath it. Some mods ship a top-level folder that is *not* a
/// wrapper at all (`Textures/DLC03/…`, `Data/Textures/…` for LUT packs) — those
/// carry no canonical directory at any depth, and descending into them blindly
/// would scatter their files across the game root. When nothing canonical turns
/// up, the original directory is returned and the layout is preserved as-is.
pub fn find_content_root(dir: &Path) -> PathBuf {
    let read_visible = |path: &Path| -> Option<Vec<std::fs::DirEntry>> {
        std::fs::read_dir(path).ok().map(|read| {
            read.flatten()
                .filter(|e| !e.file_name().to_str().map(is_ignorable).unwrap_or(false))
                .collect()
        })
    };

    let mut current = dir.to_path_buf();

    for _ in 0..5 {
        let entries = match read_visible(&current) {
            Some(entries) => entries,
            None => return dir.to_path_buf(),
        };

        // A canonical game directory here means this level is the content root.
        let has_canonical = entries.iter().any(|e| {
            e.path().is_dir()
                && e.file_name()
                    .to_str()
                    .map(|n| CANONICAL_DIRS.contains(&n.to_lowercase().as_str()))
                    .unwrap_or(false)
        });
        if has_canonical {
            return current;
        }

        // Descend only through a lone wrapper directory — never past loose files.
        match entries.as_slice() {
            [only] if only.path().is_dir() => current = only.path(),
            _ => return dir.to_path_buf(),
        }
    }

    // Descended as far as allowed without finding anything canonical: this is
    // not a wrapper we understand, so change nothing.
    dir.to_path_buf()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_standard_nexus_manual_download() {
        let parsed =
            parse_nexus_filename("Jackie's Arch - All Versions White and Gold-1464-1-0-1612607650");
        assert_eq!(parsed.name, "Jackie's Arch - All Versions White and Gold");
        assert_eq!(parsed.mod_id.as_deref(), Some("1464"));
        assert_eq!(parsed.version.as_deref(), Some("1.0"));
        assert_eq!(parsed.file_stamp.as_deref(), Some("1612607650"));
    }

    #[test]
    fn parses_three_part_version() {
        let parsed = parse_nexus_filename("Some Mod-3820-1-2-3-1612607650");
        assert_eq!(parsed.name, "Some Mod");
        assert_eq!(parsed.mod_id.as_deref(), Some("3820"));
        assert_eq!(parsed.version.as_deref(), Some("1.2.3"));
    }

    #[test]
    fn parses_missing_version() {
        let parsed = parse_nexus_filename("Some Mod-3820-1612607650");
        assert_eq!(parsed.name, "Some Mod");
        assert_eq!(parsed.mod_id.as_deref(), Some("3820"));
        assert_eq!(parsed.version, None);
        assert_eq!(parsed.file_stamp.as_deref(), Some("1612607650"));
    }

    #[test]
    fn keeps_digits_that_belong_to_the_name() {
        let parsed = parse_nexus_filename("Mod-2-Electric Boogaloo-1464-1-0-1612607650");
        assert_eq!(parsed.name, "Mod-2-Electric Boogaloo");
        assert_eq!(parsed.mod_id.as_deref(), Some("1464"));
        assert_eq!(parsed.version.as_deref(), Some("1.0"));
    }

    #[test]
    fn plain_name_is_left_alone() {
        let parsed = parse_nexus_filename("Just A Mod");
        assert_eq!(parsed.name, "Just A Mod");
        assert_eq!(parsed.mod_id, None);
        assert_eq!(parsed.version, None);
        assert_eq!(parsed.file_stamp, None);
    }

    #[test]
    fn trailing_digits_without_a_timestamp_are_not_metadata() {
        let parsed = parse_nexus_filename("Highway 66-1-0");
        assert_eq!(parsed.name, "Highway 66-1-0");
        assert_eq!(parsed.mod_id, None);
        assert_eq!(parsed.file_stamp, None);
    }

    #[test]
    fn refuses_to_guess_when_the_numeric_run_is_implausibly_long() {
        let parsed = parse_nexus_filename("Mod-1-2-3-4-5-6-1612607650");
        assert_eq!(parsed.mod_id, None);
        assert_eq!(parsed.version, None);
        assert_eq!(parsed.file_stamp.as_deref(), Some("1612607650"));
    }

    // --- find_content_root ---

    fn scratch(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cmm_test_{}_{}",
            label,
            uuid::Uuid::new_v4()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn canonical_layout_is_already_the_root() {
        let root = scratch("canonical");
        std::fs::create_dir_all(root.join("archive/pc/patch")).unwrap();
        assert_eq!(find_content_root(&root), root);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn single_wrapper_folder_is_stripped() {
        let root = scratch("wrapper");
        std::fs::create_dir_all(root.join("Combat/r6/tweaks")).unwrap();
        assert_eq!(find_content_root(&root), root.join("Combat"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn nested_wrappers_are_stripped() {
        let root = scratch("nested");
        std::fs::create_dir_all(root.join("Outer/Inner/bin/x64")).unwrap();
        assert_eq!(find_content_root(&root), root.join("Outer/Inner"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn macos_noise_does_not_block_stripping() {
        let root = scratch("macos");
        std::fs::create_dir_all(root.join("Combat/r6/scripts")).unwrap();
        std::fs::create_dir_all(root.join("__MACOSX")).unwrap();
        std::fs::write(root.join(".DS_Store"), b"x").unwrap();
        assert_eq!(find_content_root(&root), root.join("Combat"));
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn multiple_top_level_entries_are_left_alone() {
        let root = scratch("variants");
        std::fs::create_dir_all(root.join("Option A")).unwrap();
        std::fs::create_dir_all(root.join("Option B")).unwrap();
        assert_eq!(find_content_root(&root), root);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn loose_files_beside_a_folder_stop_the_descent() {
        let root = scratch("loose");
        std::fs::create_dir_all(root.join("Extras")).unwrap();
        std::fs::write(root.join("readme.txt"), b"x").unwrap();
        assert_eq!(find_content_root(&root), root);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn wrapper_without_anything_canonical_is_left_alone() {
        // Real case: "Psycho LUT" ships Textures/DLC03/… — a lone top-level
        // folder that is NOT a wrapper. Descending would scatter .dds files
        // across the game root.
        let root = scratch("lut");
        std::fs::create_dir_all(root.join("Textures/DLC03/Effects/LUTS")).unwrap();
        std::fs::create_dir_all(root.join("Textures/Effects")).unwrap();
        assert_eq!(find_content_root(&root), root);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn deep_lone_chain_without_canonical_is_left_alone() {
        // "Stellar LUT": Data/Textures/effects/… — lone folders all the way
        // down, still nothing canonical.
        let root = scratch("lut_deep");
        std::fs::create_dir_all(root.join("Data/Textures/effects/dlc001/luts")).unwrap();
        assert_eq!(find_content_root(&root), root);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn bare_archive_file_at_root_stays_at_root() {
        let root = scratch("bare");
        std::fs::write(root.join("mod.archive"), b"x").unwrap();
        assert_eq!(find_content_root(&root), root);
        std::fs::remove_dir_all(&root).ok();
    }
}
