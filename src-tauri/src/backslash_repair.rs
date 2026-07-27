//! Repair for mods whose folder structure collapsed into a single file name.
//!
//! An archive packed on Windows may store its entries as `r6\scripts\Mod\x.reds`.
//! macOS treats `\` as an ordinary filename character, so releases up to v1.5
//! wrote that entry to disk as *one* file whose name is the entire path. The mod
//! registers as installed and enabled, and validation reports it present — the
//! file really does exist at the path recorded for it. No loader ever finds it,
//! because the folders it names were never created (docs/bugs.md B5).
//!
//! Extraction splits those names now, but mods installed before the fix stay
//! broken until their files are moved, which is what this module does.
//!
//! The signature is exact and cheap: a recorded path holding a `\` inside a
//! single path component can only have come from this defect. Real directories
//! are created by the installer from split components and never contain one.

use std::path::{Path, PathBuf};

use crate::mod_repair::{plan_move, PlannedMove};

/// A mod with at least one collapsed path, plus its repair plan.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MangledMod {
    pub id: String,
    pub name: String,
    pub file_count: usize,
    pub blocked_count: usize,
    pub moves: Vec<PlannedMove>,
}

/// True when any recorded path has a backslash inside one of its components.
///
/// Cheap enough to run over every mod on every health check — it never touches
/// the disk.
pub fn has_mangled_paths(files: &[String]) -> bool {
    files.iter().any(|f| {
        Path::new(f).components().any(|c| {
            c.as_os_str()
                .to_str()
                .map(|s| s.contains('\\'))
                .unwrap_or(false)
        })
    })
}

/// Build the list of moves that would restore a mod's folder structure.
///
/// `resolve_target` maps the split path to its real install location — pass the
/// same resolver the installer uses, so repair and a fresh install agree.
///
/// Files whose recorded path is fine are left out of the plan entirely: unlike
/// the wrapper repair, this defect hits individual entries, so a mod can have
/// one collapsed file among hundreds of healthy ones.
pub fn plan_moves<F>(files: &[String], game_dir: &Path, resolve_target: F) -> Vec<PlannedMove>
where
    F: Fn(&Path, &Path) -> Result<PathBuf, String>,
{
    let mut moves = Vec::new();

    for file in files {
        let path = Path::new(file);
        let Ok(rel) = path.strip_prefix(game_dir) else {
            continue;
        };
        let rel_str = rel.to_string_lossy();
        if !rel_str.contains('\\') {
            continue;
        }

        // Same splitting the extractor now applies, so a repaired file lands
        // exactly where installing the archive today would put it.
        let Some(split) = crate::archive_extractor::sanitize_entry_path(&rel_str) else {
            continue;
        };

        let Ok(target) = resolve_target(game_dir, &split) else {
            continue;
        };

        if let Some(mv) = plan_move(file, &target) {
            moves.push(mv);
        }
    }

    moves
}

#[cfg(test)]
mod tests {
    use super::*;

    fn game() -> PathBuf {
        PathBuf::from("/game")
    }

    fn passthrough(game_dir: &Path, inner: &Path) -> Result<PathBuf, String> {
        Ok(game_dir.join(inner))
    }

    #[test]
    fn detects_a_collapsed_path() {
        let files = vec![r"/game/r6\scripts\Ragdoll Execution Fix\Fix.Global.reds".to_string()];
        assert!(has_mangled_paths(&files));
    }

    #[test]
    fn a_healthy_library_is_not_flagged() {
        let files = vec![
            "/game/r6/scripts/Mod/x.reds".to_string(),
            "/game/archive/pc/mod/y.archive".to_string(),
        ];
        assert!(!has_mangled_paths(&files));
    }

    #[test]
    fn plans_the_move_that_restores_the_tree() {
        let files = vec![r"/game/r6\scripts\Mod\x.reds".to_string()];
        let moves = plan_moves(&files, &game(), passthrough);

        assert_eq!(moves.len(), 1);
        assert_eq!(moves[0].from, r"/game/r6\scripts\Mod\x.reds");
        assert_eq!(moves[0].to, "/game/r6/scripts/Mod/x.reds");
    }

    #[test]
    fn healthy_files_stay_out_of_the_plan() {
        // The real shape: one collapsed entry among a mod's ordinary files.
        let files = vec![
            "/game/r6/scripts/Mod/ok.reds".to_string(),
            r"/game/r6\scripts\Mod\broken.reds".to_string(),
        ];
        let moves = plan_moves(&files, &game(), passthrough);

        assert_eq!(moves.len(), 1);
        assert!(moves[0].from.ends_with(r"broken.reds"));
    }

    #[test]
    fn files_outside_the_game_dir_are_skipped() {
        let files = vec![r"/elsewhere/r6\scripts\x.reds".to_string()];
        assert!(plan_moves(&files, &game(), passthrough).is_empty());
    }

    #[test]
    fn a_resolver_that_refuses_drops_the_file() {
        let files = vec![r"/game/r6\scripts\x.reds".to_string()];
        let moves = plan_moves(&files, &game(), |_, _| Err("blocked".to_string()));
        assert!(moves.is_empty());
    }

    #[test]
    fn a_ghosted_file_is_planned_with_its_disabled_suffix() {
        let root = std::env::temp_dir().join(format!("cmm_bs_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let recorded = root.join(r"r6\scripts\x.reds");
        std::fs::write(format!("{}.disabled", recorded.display()), b"code").unwrap();

        let files = vec![recorded.to_string_lossy().to_string()];
        let moves = plan_moves(&files, &root, passthrough);

        assert_eq!(moves.len(), 1);
        assert!(moves[0].disabled, "the ghosted spelling is what exists on disk");
        assert!(!moves[0].blocked);
        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn an_occupied_destination_is_reported_as_blocked() {
        let root = std::env::temp_dir().join(format!("cmm_bs2_{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(root.join("r6/scripts")).unwrap();
        let recorded = root.join(r"r6\scripts\x.reds");
        std::fs::write(&recorded, b"mine").unwrap();
        std::fs::write(root.join("r6/scripts/x.reds"), b"someone else's").unwrap();

        let files = vec![recorded.to_string_lossy().to_string()];
        let moves = plan_moves(&files, &root, passthrough);

        assert_eq!(moves.len(), 1);
        assert!(moves[0].blocked);
        std::fs::remove_dir_all(&root).ok();
    }
}
