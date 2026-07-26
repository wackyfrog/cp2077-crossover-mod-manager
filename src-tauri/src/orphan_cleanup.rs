//! Cleanup for mod folders left behind in Cyber Engine Tweaks' `mods/` directory.
//!
//! Removing a mod deletes the files the manager tracks, but a CET mod folder
//! usually outlives that: CET writes its own `db.sqlite3`, a log, and often a
//! settings file into the folder while the mod runs, and none of those are in
//! any manifest. The folder therefore survives with no `init.lua` in it, and CET
//! reports it at every launch — `Ignoring mod which does not contain init.lua!`.
//! Folders orphaned by the pre-fix removal bug (see docs/bugs.md B2) look the
//! same, except they still hold the mod's real files under `.disabled`.
//!
//! Nothing here runs on its own. Scanning is read-only, and deletion only ever
//! touches folders the caller passed in, after re-checking they are unclaimed —
//! a folder can hold files belonging to a *different* installed mod (mods do
//! ship presets for each other), and those must never be swept away.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

/// CET's mod directory, relative to the game root.
pub const CET_MODS_REL: &str = "bin/x64/plugins/cyber_engine_tweaks/mods";

/// What a leftover folder holds, which decides how safe it is to delete.
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OrphanKind {
    /// Nothing but CET's own runtime droppings and empty subfolders.
    RuntimeOnly,
    /// Ghosted (`.disabled`) files that no installed mod claims — B2 leftovers.
    OrphanedGhosts,
    /// Settings or user data; deleting the folder throws those away.
    HoldsUserData,
}

/// A CET mod folder that the loader ignores, with enough detail to decide on it.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OrphanDir {
    pub name: String,
    pub path: String,
    pub kind: OrphanKind,
    pub file_count: usize,
    pub bytes: u64,
    /// File names, so the user can see what would go. Capped for display.
    pub sample: Vec<String>,
}

/// Files CET creates by itself while a mod runs. They carry no user intent, so
/// they are the one thing safe to delete without asking.
fn is_runtime_dropping(name: &str) -> bool {
    if name == "db.sqlite3" || crate::local_archive::is_ignorable(name) {
        return true;
    }
    if name.ends_with(".log") {
        return true;
    }
    // Rotated logs: `jb_third_person_mod.1.log` is caught above, `app.log.1`
    // here.
    match name.rsplit_once('.') {
        Some((head, tail)) => head.ends_with(".log") && tail.chars().all(|c| c.is_ascii_digit()),
        None => false,
    }
}

/// Decide what a folder holds, from the names of the files inside it.
///
/// Order matters: anything the user might want back outranks everything else,
/// and ghosted files outrank runtime droppings because they are the mod itself.
pub fn classify(file_names: &[String]) -> OrphanKind {
    let mut has_ghost = false;

    for name in file_names {
        if name.ends_with(".disabled") {
            has_ghost = true;
        } else if !is_runtime_dropping(name) {
            return OrphanKind::HoldsUserData;
        }
    }

    if has_ghost {
        OrphanKind::OrphanedGhosts
    } else {
        OrphanKind::RuntimeOnly
    }
}

/// Every file under `dir`, as (full path, file name, size).
fn files_in(dir: &Path) -> Vec<(PathBuf, String, u64)> {
    WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file())
        .map(|e| {
            let size = e.metadata().map(|m| m.len()).unwrap_or(0);
            let name = e.file_name().to_string_lossy().to_string();
            (e.path().to_path_buf(), name, size)
        })
        .collect()
}

/// Find CET mod folders the loader ignores and no installed mod claims.
///
/// `owned` holds the lowercased paths of every file belonging to a mod that is
/// still installed, including the `.disabled` spelling of each. A folder with
/// even one such file is skipped whole: it is in use, whatever it looks like.
pub fn scan(cet_mods_dir: &Path, owned: &HashSet<String>) -> Vec<OrphanDir> {
    let Ok(entries) = std::fs::read_dir(cet_mods_dir) else {
        return Vec::new();
    };

    let mut found = Vec::new();

    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        // The loader's own test: a folder with init.lua is a working mod.
        if dir.join("init.lua").exists() {
            continue;
        }

        let files = files_in(&dir);
        let claimed = files
            .iter()
            .any(|(path, _, _)| owned.contains(&path.to_string_lossy().to_lowercase()));
        if claimed {
            continue;
        }

        let names: Vec<String> = files.iter().map(|(_, name, _)| name.clone()).collect();
        found.push(OrphanDir {
            name: entry.file_name().to_string_lossy().to_string(),
            path: dir.to_string_lossy().to_string(),
            kind: classify(&names),
            file_count: files.len(),
            bytes: files.iter().map(|(_, _, size)| size).sum(),
            sample: names.into_iter().take(12).collect(),
        });
    }

    found.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    found
}

/// Delete the given folders. Returns how many went, and a reason per failure.
///
/// Every path is re-checked against `cet_mods_dir` and `owned` before anything
/// is removed: the scan that produced these paths may be minutes old, and a
/// reinstall in between must not be deleted out from under the user.
pub fn delete_dirs(
    paths: &[String],
    cet_mods_dir: &Path,
    owned: &HashSet<String>,
) -> (usize, Vec<String>) {
    let mut deleted = 0;
    let mut failed = Vec::new();

    for raw in paths {
        let dir = PathBuf::from(raw);

        if dir.parent() != Some(cet_mods_dir) || !dir.is_dir() {
            failed.push(format!("{}: not a CET mod folder", raw));
            continue;
        }
        if dir.join("init.lua").exists() {
            failed.push(format!("{}: has an init.lua now, leaving it alone", raw));
            continue;
        }
        let claimed = files_in(&dir)
            .iter()
            .any(|(path, _, _)| owned.contains(&path.to_string_lossy().to_lowercase()));
        if claimed {
            failed.push(format!("{}: holds files of an installed mod", raw));
            continue;
        }

        match std::fs::remove_dir_all(&dir) {
            Ok(_) => deleted += 1,
            Err(e) => failed.push(format!("{}: {}", raw, e)),
        }
    }

    (deleted, failed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("cmm_orphan_{}_{}", tag, uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn touch(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"x").unwrap();
    }

    #[test]
    fn runtime_droppings_alone_are_safe_to_sweep() {
        assert_eq!(
            classify(&names(&["db.sqlite3", "Vehicle Clone Destroyer.log"])),
            OrphanKind::RuntimeOnly
        );
    }

    #[test]
    fn rotated_logs_still_count_as_droppings() {
        assert_eq!(
            classify(&names(&["jb_third_person_mod.1.log", "app.log.3", "db.sqlite3"])),
            OrphanKind::RuntimeOnly
        );
    }

    #[test]
    fn ghosted_files_are_called_out_separately() {
        assert_eq!(
            classify(&names(&["init.lua.disabled", "db.sqlite3"])),
            OrphanKind::OrphanedGhosts
        );
    }

    #[test]
    fn a_settings_file_outranks_everything() {
        // Real case: Throwable Aim Slow Time kept config.json after removal.
        assert_eq!(
            classify(&names(&["config.json", "db.sqlite3", "x.log"])),
            OrphanKind::HoldsUserData
        );
        assert_eq!(
            classify(&names(&["init.lua.disabled", "settings.json"])),
            OrphanKind::HoldsUserData
        );
    }

    #[test]
    fn an_empty_folder_is_sweepable() {
        assert_eq!(classify(&[]), OrphanKind::RuntimeOnly);
    }

    #[test]
    fn a_loadable_mod_is_never_a_candidate() {
        let mods = scratch("loadable");
        touch(&mods.join("Working/init.lua"));
        touch(&mods.join("Working/db.sqlite3"));

        assert!(scan(&mods, &HashSet::new()).is_empty());
        std::fs::remove_dir_all(&mods).ok();
    }

    #[test]
    fn a_folder_holding_another_mods_file_is_left_alone() {
        // Real case: entSpawner holds a preset shipped by Native Interactions.
        let mods = scratch("claimed");
        let preset = mods.join("entSpawner/data/favorite/NIF.json");
        touch(&preset);

        let owned: HashSet<String> =
            [preset.to_string_lossy().to_lowercase()].into_iter().collect();

        assert!(scan(&mods, &owned).is_empty(), "an in-use folder must not be offered");
        assert_eq!(scan(&mods, &HashSet::new()).len(), 1, "unclaimed, it is a candidate");
        std::fs::remove_dir_all(&mods).ok();
    }

    #[test]
    fn scan_reports_size_and_kind_per_folder() {
        let mods = scratch("report");
        touch(&mods.join("Junk/db.sqlite3"));
        touch(&mods.join("Junk/Junk.log"));
        touch(&mods.join("Ghost/init.lua.disabled"));

        let found = scan(&mods, &HashSet::new());

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].name, "Ghost");
        assert_eq!(found[0].kind, OrphanKind::OrphanedGhosts);
        assert_eq!(found[1].kind, OrphanKind::RuntimeOnly);
        assert_eq!(found[1].file_count, 2);
        assert_eq!(found[1].bytes, 2);
        std::fs::remove_dir_all(&mods).ok();
    }

    #[test]
    fn deletion_refuses_a_path_outside_the_mods_directory() {
        let mods = scratch("outside");
        let elsewhere = scratch("elsewhere");
        touch(&elsewhere.join("keep.txt"));

        let (deleted, failed) =
            delete_dirs(&[elsewhere.to_string_lossy().to_string()], &mods, &HashSet::new());

        assert_eq!(deleted, 0);
        assert_eq!(failed.len(), 1);
        assert!(elsewhere.join("keep.txt").exists());
        std::fs::remove_dir_all(&mods).ok();
        std::fs::remove_dir_all(&elsewhere).ok();
    }

    #[test]
    fn deletion_rechecks_ownership_before_removing() {
        let mods = scratch("recheck");
        let preset = mods.join("entSpawner/data/favorite/NIF.json");
        touch(&preset);
        let owned: HashSet<String> =
            [preset.to_string_lossy().to_lowercase()].into_iter().collect();

        let (deleted, failed) = delete_dirs(
            &[mods.join("entSpawner").to_string_lossy().to_string()],
            &mods,
            &owned,
        );

        assert_eq!(deleted, 0, "ownership acquired after the scan must still block");
        assert_eq!(failed.len(), 1);
        assert!(preset.exists());
        std::fs::remove_dir_all(&mods).ok();
    }

    #[test]
    fn deletion_removes_an_unclaimed_folder() {
        let mods = scratch("sweep");
        touch(&mods.join("Gone/db.sqlite3"));

        let (deleted, failed) = delete_dirs(
            &[mods.join("Gone").to_string_lossy().to_string()],
            &mods,
            &HashSet::new(),
        );

        assert_eq!(deleted, 1);
        assert!(failed.is_empty());
        assert!(!mods.join("Gone").exists());
        std::fs::remove_dir_all(&mods).ok();
    }
}
