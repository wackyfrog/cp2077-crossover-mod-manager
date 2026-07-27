//! Repair for mods that were installed inside a redundant wrapper folder.
//!
//! Releases up to v1.2 copied an archive's layout verbatim, so a mod packaged as
//! `ModName/r6/tweaks/…` landed in `{game}/ModName/r6/…`. No loader ever looks
//! there: the mod registers as installed and enabled, yet is completely inert.
//! Installing now strips the wrapper (see [`crate::local_archive::find_content_root`]),
//! but mods installed *before* the fix stay broken until their files are moved,
//! which is what this module does.
//!
//! Detection is deliberately conservative — see [`detect_wrapper`].

use std::path::{Path, PathBuf};

use crate::local_archive::CANONICAL_DIRS;

/// One file that needs to move, with the destination already resolved.
#[derive(Debug, Clone, serde::Serialize)]
pub struct PlannedMove {
    pub from: String,
    pub to: String,
    /// Set when the file is currently ghosted (`.disabled` on disk).
    pub disabled: bool,
    /// Something already occupies the destination — this move will be skipped.
    pub blocked: bool,
}

/// A mod detected as installed under a wrapper folder, plus its repair plan.
#[derive(Debug, Clone, serde::Serialize)]
pub struct WrappedMod {
    pub id: String,
    pub name: String,
    pub wrapper: String,
    pub file_count: usize,
    pub blocked_count: usize,
    pub moves: Vec<PlannedMove>,
}

fn is_canonical(component: &str) -> bool {
    CANONICAL_DIRS.contains(&component.to_lowercase().as_str())
}

/// Decide whether a mod's installed files sit under a single wrapper folder.
///
/// Returns the wrapper's name, or `None` when the layout should be left alone.
///
/// Two conditions must both hold, and they exist to avoid making things worse:
///
/// 1. **Every** file lives under one and the same non-canonical top-level folder.
///    A mod with files in both `r6/` and `Variants/` is a multi-variant or FOMOD
///    archive — moving its parts would activate options the user never chose.
/// 2. A canonical directory (`r6`, `bin`, `archive`, …) appears immediately
///    inside that folder. Without this, the folder may not be a wrapper at all:
///    LUT packs ship `Textures/DLC03/…` and `Data/Textures/…`, which are
///    meaningful as-is, and flattening them would scatter loose files across the
///    game root.
///
/// This mirrors [`crate::local_archive::find_content_root`] exactly, so repairing
/// yields the same result a fresh install would.
pub fn detect_wrapper(files: &[String], game_dir: &Path) -> Option<String> {
    if files.is_empty() {
        return None;
    }

    let mut wrapper: Option<String> = None;
    let mut has_canonical_child = false;

    for file in files {
        let path = Path::new(file);
        let rel = path.strip_prefix(game_dir).ok()?;
        let mut components = rel.components();

        let top = components.next()?.as_os_str().to_str()?.to_string();
        // A file sitting directly in the game root has no wrapper.
        let second = components.next();
        if second.is_none() {
            return None;
        }
        if is_canonical(&top) {
            return None;
        }

        match &wrapper {
            None => wrapper = Some(top),
            Some(existing) if *existing == top => {}
            // Files under two different top-level folders: not a simple wrapper.
            Some(_) => return None,
        }

        if let Some(second) = second {
            if let Some(name) = second.as_os_str().to_str() {
                if is_canonical(name) {
                    has_canonical_child = true;
                }
            }
        }
    }

    if has_canonical_child {
        wrapper
    } else {
        None
    }
}

/// Turn a file and its resolved destination into a [`PlannedMove`], or `None`
/// when the file already sits where it belongs.
///
/// Shared by every repair so they agree on the two things that are easy to get
/// wrong: a ghosted mod's files carry a `.disabled` suffix on disk, and a
/// destination that is already occupied must be reported, not overwritten.
pub fn plan_move(file: &str, target: &Path) -> Option<PlannedMove> {
    if target == Path::new(file) {
        return None;
    }

    let disabled = !Path::new(file).exists() && PathBuf::from(format!("{}.disabled", file)).exists();

    let dest_exists = if disabled {
        PathBuf::from(format!("{}.disabled", target.display())).exists()
    } else {
        target.exists()
    };

    Some(PlannedMove {
        from: file.to_string(),
        to: target.to_string_lossy().to_string(),
        disabled,
        blocked: dest_exists,
    })
}

/// Build the list of moves that would un-wrap a mod.
///
/// `resolve_target` maps a wrapper-relative path to its real install location —
/// pass the same resolver the installer uses, so repair and install agree.
pub fn plan_moves<F>(
    files: &[String],
    game_dir: &Path,
    wrapper: &str,
    resolve_target: F,
) -> Vec<PlannedMove>
where
    F: Fn(&Path, &Path) -> Result<PathBuf, String>,
{
    let mut moves = Vec::new();

    for file in files {
        let path = Path::new(file);
        let rel = match path.strip_prefix(game_dir) {
            Ok(rel) => rel,
            Err(_) => continue,
        };

        // Guard: only strip the component we actually identified as the wrapper.
        let starts_with_wrapper = rel
            .components()
            .next()
            .and_then(|c| c.as_os_str().to_str())
            .map(|first| first == wrapper)
            .unwrap_or(false);
        if !starts_with_wrapper {
            continue;
        }

        // Drop the wrapper component; what remains is the path the installer
        // would have seen had the wrapper been stripped at install time.
        let inner: PathBuf = rel.components().skip(1).collect();
        if inner.as_os_str().is_empty() {
            continue;
        }

        let target = match resolve_target(game_dir, &inner) {
            Ok(target) => target,
            Err(_) => continue,
        };

        if let Some(mv) = plan_move(file, &target) {
            moves.push(mv);
        }
    }

    moves
}

/// What actually happened when a repair plan was applied.
#[derive(Debug, Default)]
pub struct ApplyOutcome {
    pub moved: usize,
    pub skipped: usize,
    pub failed: usize,
    pub errors: Vec<String>,
    /// Old path -> new path, for the files that really moved. Feed this to the
    /// database so its records follow the files.
    pub remap: std::collections::HashMap<String, String>,
    /// Directories the moves emptied out, for pruning afterwards.
    pub touched_dirs: Vec<PathBuf>,
}

/// Execute a repair plan on disk.
///
/// `set_perms(path, is_dir)` applies the caller's permission policy to anything
/// created — passed in so this module stays free of platform specifics.
///
/// Blocked moves are skipped rather than overwritten: the destination already
/// holds a file, and clobbering another mod's install would trade one silent
/// breakage for another.
pub fn apply_moves<P>(moves: &[PlannedMove], set_perms: P) -> ApplyOutcome
where
    P: Fn(&Path, bool),
{
    let mut out = ApplyOutcome::default();

    for mv in moves {
        if mv.blocked {
            out.skipped += 1;
            continue;
        }

        // A ghosted mod's files carry a .disabled suffix on disk.
        let (src, dest) = if mv.disabled {
            (
                PathBuf::from(format!("{}.disabled", mv.from)),
                PathBuf::from(format!("{}.disabled", mv.to)),
            )
        } else {
            (PathBuf::from(&mv.from), PathBuf::from(&mv.to))
        };

        if !src.exists() {
            // Recorded in the database but already gone from disk. Still rewrite
            // the path so the record stops pointing into the wrapper.
            out.remap.insert(mv.from.clone(), mv.to.clone());
            out.skipped += 1;
            continue;
        }

        if let Some(parent) = dest.parent() {
            if let Err(e) = std::fs::create_dir_all(parent) {
                out.errors.push(format!("{}: {}", parent.display(), e));
                out.failed += 1;
                continue;
            }
            set_perms(parent, true);
        }

        match std::fs::rename(&src, &dest) {
            Ok(_) => {
                set_perms(&dest, false);
                if let Some(parent) = src.parent() {
                    out.touched_dirs.push(parent.to_path_buf());
                }
                out.remap.insert(mv.from.clone(), mv.to.clone());
                out.moved += 1;
            }
            Err(e) => {
                out.errors.push(format!("{}: {}", src.display(), e));
                out.failed += 1;
            }
        }
    }

    out.touched_dirs.sort();
    out.touched_dirs.dedup();
    // Deepest first, so nested directories empty out before their parents.
    out.touched_dirs.reverse();

    out
}

/// Remove directories left empty after a repair, walking upward from the deepest
/// entry. Stops at `game_dir` and never touches a directory that still holds
/// real content.
///
/// A directory containing nothing but Finder droppings (`.DS_Store`, `__MACOSX`,
/// AppleDouble `._*`) counts as empty — Finder sprinkles those into any folder
/// the user has ever browsed, and leaving a hollow wrapper tree standing because
/// of them defeats the point of the cleanup. That junk is removed along with the
/// directory; nothing else ever is.
pub fn prune_empty_dirs(start: &Path, game_dir: &Path) -> usize {
    let mut removed = 0;
    let mut current = start.to_path_buf();

    while current.starts_with(game_dir) && current != game_dir {
        let entries: Vec<_> = match std::fs::read_dir(&current) {
            Ok(read) => read.flatten().collect(),
            Err(_) => break,
        };

        let only_junk = entries.iter().all(|e| {
            e.path().is_file()
                && e.file_name()
                    .to_str()
                    .map(crate::local_archive::is_ignorable)
                    .unwrap_or(false)
        });
        if !only_junk {
            break;
        }

        for entry in &entries {
            if std::fs::remove_file(entry.path()).is_err() {
                return removed;
            }
        }

        if std::fs::remove_dir(&current).is_err() {
            break;
        }
        removed += 1;
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => break,
        }
    }

    removed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> PathBuf {
        PathBuf::from("/game")
    }

    fn files(paths: &[&str]) -> Vec<String> {
        paths.iter().map(|p| format!("/game/{}", p)).collect()
    }

    #[test]
    fn detects_a_real_wrapper() {
        let f = files(&[
            "Combat/r6/tweaks/x.yaml",
            "Combat/bin/x64/plugins/cyber_engine_tweaks/mods/Combat/init.lua",
        ]);
        assert_eq!(detect_wrapper(&f, &game()), Some("Combat".to_string()));
    }

    #[test]
    fn canonical_layout_is_not_wrapped() {
        let f = files(&["r6/tweaks/x.yaml", "archive/pc/mod/y.archive"]);
        assert_eq!(detect_wrapper(&f, &game()), None);
    }

    #[test]
    fn multi_variant_layout_is_left_alone() {
        // Real case: Ducati 916 — fomod/ options alongside a real archive/ tree.
        let f = files(&[
            "fomod/Options/a.archive",
            "archive/pc/mod/void_Ducati_916_xx_base.archive",
        ]);
        assert_eq!(detect_wrapper(&f, &game()), None);
    }

    #[test]
    fn folder_without_canonical_child_is_left_alone() {
        // Real case: Psycho LUT — Textures/DLC03/… is not a wrapper.
        let f = files(&[
            "Textures/DLC03/Effects/LUTS/a.dds",
            "Textures/Effects/LUTS/b.dds",
        ]);
        assert_eq!(detect_wrapper(&f, &game()), None);
    }

    #[test]
    fn loose_file_in_game_root_is_not_wrapped() {
        let f = files(&["readme.txt"]);
        assert_eq!(detect_wrapper(&f, &game()), None);
    }

    #[test]
    fn files_outside_the_game_dir_abort_detection() {
        let f = vec!["/elsewhere/Combat/r6/x.yaml".to_string()];
        assert_eq!(detect_wrapper(&f, &game()), None);
    }

    #[test]
    fn plan_strips_the_wrapper_component() {
        let f = files(&["Combat/r6/tweaks/x.yaml"]);
        let moves = plan_moves(&f, &game(), "Combat", |game_dir, inner| {
            Ok(game_dir.join(inner))
        });
        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].from, "/game/Combat/r6/tweaks/x.yaml");
        assert_eq!(moves[0].to, "/game/r6/tweaks/x.yaml");
        assert!(!moves[0].disabled);
    }

    #[test]
    fn prunes_a_tree_left_holding_only_finder_junk() {
        let root = std::env::temp_dir().join(format!("cmm_prune_{}", uuid::Uuid::new_v4()));
        let deep = root.join("Wrapper/bin/x64");
        std::fs::create_dir_all(&deep).unwrap();
        for dir in [&root.join("Wrapper"), &root.join("Wrapper/bin"), &deep] {
            std::fs::write(dir.join(".DS_Store"), b"junk").unwrap();
        }

        let removed = prune_empty_dirs(&deep, &root);
        assert_eq!(removed, 3);
        assert!(!root.join("Wrapper").exists());
        assert!(root.exists(), "pruning must stop at the game dir");
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn pruning_stops_at_a_directory_with_real_content() {
        let root = std::env::temp_dir().join(format!("cmm_prune2_{}", uuid::Uuid::new_v4()));
        let deep = root.join("Wrapper/bin");
        std::fs::create_dir_all(&deep).unwrap();
        std::fs::write(root.join("Wrapper/keep.txt"), b"real").unwrap();
        std::fs::write(deep.join(".DS_Store"), b"junk").unwrap();

        let removed = prune_empty_dirs(&deep, &root);
        assert_eq!(removed, 1, "only the junk-only leaf goes");
        assert!(root.join("Wrapper/keep.txt").exists());
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn plan_skips_files_that_would_not_move() {
        let f = files(&["Combat/r6/tweaks/x.yaml"]);
        let moves = plan_moves(&f, &game(), "Combat", |_, _| {
            Ok(PathBuf::from("/game/Combat/r6/tweaks/x.yaml"))
        });
        assert!(moves.is_empty());
    }
}
