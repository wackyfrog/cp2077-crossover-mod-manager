use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

/// Turn an archive entry's stored name into a safe relative path.
///
/// Two things make the raw name unusable as-is:
///
/// 1. **Separators.** ZIP requires `/`, but Windows archivers write `\` often
///    enough that Nexus is full of such files. On macOS `\` is an ordinary
///    filename character, so `extract_dir.join("r6\\scripts\\x.reds")` yields a
///    single file whose *name* is the whole path — the mod installs, validates
///    as present, and no loader ever sees it (docs/bugs.md B5). Both characters
///    are treated as separators here.
/// 2. **Traversal.** An entry named `../../etc/x` would escape the extraction
///    directory. Such an entry is rejected outright (`None`) rather than
///    silently rewritten: an archive that asks for it is not one to guess about.
///
/// Empty and `.` segments are dropped, as is a leading drive letter (`C:`).
/// Returns `None` when nothing usable remains.
pub fn sanitize_entry_path(raw: &str) -> Option<PathBuf> {
    let mut out = PathBuf::new();

    for (i, segment) in raw.split(['/', '\\']).enumerate() {
        match segment {
            "" | "." => continue,
            ".." => return None,
            _ => {}
        }
        // "C:" only ever appears as the first segment of an absolute Windows path.
        if i == 0 && segment.len() == 2 && segment.ends_with(':') {
            let first = segment.as_bytes()[0];
            if first.is_ascii_alphabetic() {
                continue;
            }
        }
        out.push(segment);
    }

    if out.as_os_str().is_empty() {
        None
    } else {
        Some(out)
    }
}

/// Split any file whose *name* holds path separators into a real directory tree.
///
/// System extractors (`7z`, `unrar`) get the entry name handed to them by the
/// archive and it is unverified whether they split `\` themselves — this runs
/// after them so all five extraction paths end up with the same layout.
/// Returns how many files were moved.
fn split_flattened_names(extract_dir: &Path) -> usize {
    // Collect first: renaming while walking would visit moved files again.
    let flattened: Vec<PathBuf> = WalkDir::new(extract_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|e| e.into_path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.contains('\\'))
                .unwrap_or(false)
        })
        .collect();

    let mut moved = 0;
    for path in flattened {
        let (Some(parent), Some(name)) = (path.parent(), path.file_name().and_then(|n| n.to_str()))
        else {
            continue;
        };
        // A name that sanitizes to nothing (or tries to traverse) stays put: it
        // is inert where it is, and inventing a destination would be worse.
        let Some(rel) = sanitize_entry_path(name) else {
            println!("⚠ Refusing to split unsafe entry name: {}", name);
            continue;
        };
        let dest = parent.join(rel);
        if dest == path || dest.exists() {
            continue;
        }
        if let Some(dest_parent) = dest.parent() {
            if fs::create_dir_all(dest_parent).is_err() {
                continue;
            }
        }
        if fs::rename(&path, &dest).is_ok() {
            println!("↳ Split Windows-style entry name: {}", name);
            moved += 1;
        }
    }

    moved
}

#[derive(Debug, Clone)]
pub enum ArchiveType {
    Zip,
    SevenZ,
    Rar,
    Unsupported(String),
}

#[derive(Debug, Clone)]
pub enum ExtractionMethod {
    RustZip,
    RustSevenz,
    RustUnrar,
    SystemP7zip,
    SystemUnrar,
}

pub struct ArchiveExtractor;

impl ArchiveExtractor {
    #[allow(dead_code)]
    pub fn new() -> Self {
        Self
    }

    /// Detect archive type by reading file magic bytes (more reliable than extension)
    pub fn detect_archive_type(archive_path: &Path) -> ArchiveType {
        // Try to read the first few bytes to detect the actual format
        if let Ok(mut file) = fs::File::open(archive_path) {
            let mut magic = [0u8; 8];
            if std::io::Read::read(&mut file, &mut magic).is_ok() {
                // Check magic bytes for each format
                // ZIP: 50 4B 03 04 or 50 4B 05 06 (empty archive) or 50 4B 07 08 (spanned)
                if magic[0] == 0x50
                    && magic[1] == 0x4B
                    && (magic[2] == 0x03 || magic[2] == 0x05 || magic[2] == 0x07)
                {
                    return ArchiveType::Zip;
                }

                // 7z: 37 7A BC AF 27 1C
                if magic[0] == 0x37
                    && magic[1] == 0x7A
                    && magic[2] == 0xBC
                    && magic[3] == 0xAF
                    && magic[4] == 0x27
                    && magic[5] == 0x1C
                {
                    return ArchiveType::SevenZ;
                }

                // RAR: 52 61 72 21 1A 07 (RAR 1.5+) or 52 61 72 21 1A 07 01 00 (RAR 5.0+)
                if magic[0] == 0x52
                    && magic[1] == 0x61
                    && magic[2] == 0x72
                    && magic[3] == 0x21
                    && magic[4] == 0x1A
                    && magic[5] == 0x07
                {
                    return ArchiveType::Rar;
                }
            }
        }

        // Fallback to extension-based detection if magic bytes don't match
        match archive_path.extension().and_then(|s| s.to_str()) {
            Some("zip") => ArchiveType::Zip,
            Some("7z") => ArchiveType::SevenZ,
            Some("rar") => ArchiveType::Rar,
            Some(ext) => ArchiveType::Unsupported(ext.to_string()),
            None => ArchiveType::Unsupported("unknown".to_string()),
        }
    }

    /// Extract archive using hybrid approach (system tools + Rust fallbacks)
    pub fn extract(
        archive_path: &Path,
        extract_dir: &Path,
    ) -> Result<(usize, ExtractionMethod), String> {
        let archive_type = Self::detect_archive_type(archive_path);

        match archive_type {
            ArchiveType::Zip => Self::extract_zip(archive_path, extract_dir),
            ArchiveType::SevenZ => Self::extract_7z_hybrid(archive_path, extract_dir),
            ArchiveType::Rar => Self::extract_rar_hybrid(archive_path, extract_dir),
            ArchiveType::Unsupported(ext) => Err(format!("Unsupported archive format: .{}", ext)),
        }
    }

    /// Extract ZIP using Rust zip crate
    fn extract_zip(
        archive_path: &Path,
        extract_dir: &Path,
    ) -> Result<(usize, ExtractionMethod), String> {
        let file = fs::File::open(archive_path)
            .map_err(|e| format!("Failed to open ZIP archive: {}", e))?;

        let mut archive =
            zip::ZipArchive::new(file).map_err(|e| format!("Failed to read ZIP: {}", e))?;

        let mut count = 0;
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .map_err(|e| format!("Failed to read entry: {}", e))?;

            let Some(rel) = sanitize_entry_path(file.name()) else {
                println!("⚠ Skipping unsafe archive entry: {}", file.name());
                continue;
            };
            let outpath = extract_dir.join(rel);

            if file.name().ends_with('/') || file.name().ends_with('\\') {
                fs::create_dir_all(&outpath).ok();
            } else {
                if let Some(p) = outpath.parent() {
                    fs::create_dir_all(p)
                        .map_err(|e| format!("Failed to create directory: {}", e))?;
                }
                let mut outfile = fs::File::create(&outpath)
                    .map_err(|e| format!("Failed to create file: {}", e))?;
                io::copy(&mut file, &mut outfile)
                    .map_err(|e| format!("Failed to extract file: {}", e))?;
                count += 1;
            }
        }

        Ok((count, ExtractionMethod::RustZip))
    }

    /// Extract 7z using hybrid approach (system p7zip or Rust fallback)
    fn extract_7z_hybrid(
        archive_path: &Path,
        extract_dir: &Path,
    ) -> Result<(usize, ExtractionMethod), String> {
        // Try system p7zip first (faster, more compatible)
        if let Ok(count) = Self::try_system_7z(archive_path, extract_dir) {
            return Ok((count, ExtractionMethod::SystemP7zip));
        }

        // Fallback to Rust library
        println!("System p7zip not available, using built-in extractor...");
        Self::extract_7z_rust(archive_path, extract_dir)
    }

    /// Extract 7z using system p7zip command
    fn try_system_7z(archive_path: &Path, extract_dir: &Path) -> Result<usize, String> {
        // Check if 7z is installed
        if !Self::check_command_exists("7z") && !Self::check_command_exists("7za") {
            return Err("7z not installed".to_string());
        }

        let cmd = if Self::check_command_exists("7z") {
            "7z"
        } else {
            "7za"
        };

        // Create extraction directory
        fs::create_dir_all(extract_dir)
            .map_err(|e| format!("Failed to create extraction directory: {}", e))?;

        // Extract archive
        let output = Command::new(cmd)
            .arg("x") // Extract with full paths
            .arg("-y") // Yes to all prompts
            .arg(archive_path)
            .arg(format!("-o{}", extract_dir.display()))
            .output()
            .map_err(|e| format!("Failed to run 7z: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "7z extraction failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        split_flattened_names(extract_dir);

        // Count extracted files
        let count = WalkDir::new(extract_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .count();

        Ok(count)
    }

    /// Extract 7z using Rust sevenz-rust crate
    fn extract_7z_rust(
        archive_path: &Path,
        extract_dir: &Path,
    ) -> Result<(usize, ExtractionMethod), String> {
        use sevenz_rust::*;

        fs::create_dir_all(extract_dir)
            .map_err(|e| format!("Failed to create extraction directory: {}", e))?;

        let mut count = 0;
        let mut sz = SevenZReader::open(archive_path, Password::empty())
            .map_err(|e| format!("Failed to open 7z archive: {}", e))?;

        sz.for_each_entries(|entry, reader| {
            if !entry.is_directory() {
                let Some(rel) = sanitize_entry_path(&entry.name()) else {
                    println!("⚠ Skipping unsafe archive entry: {}", entry.name());
                    return Ok(true);
                };
                let output_path = extract_dir.join(rel);
                if let Some(parent) = output_path.parent() {
                    fs::create_dir_all(parent).ok();
                }

                let mut outfile = fs::File::create(&output_path)?;
                io::copy(reader, &mut outfile)?;

                count += 1;
            }
            Ok(true) // Continue iteration
        })
        .map_err(|e| format!("7z extraction error: {}", e))?;

        Ok((count, ExtractionMethod::RustSevenz))
    }

    /// Extract RAR using hybrid approach (system unrar or Rust fallback)
    fn extract_rar_hybrid(
        archive_path: &Path,
        extract_dir: &Path,
    ) -> Result<(usize, ExtractionMethod), String> {
        // Try system unrar first (faster, more compatible)
        if let Ok(count) = Self::try_system_unrar(archive_path, extract_dir) {
            return Ok((count, ExtractionMethod::SystemUnrar));
        }

        // Fallback to Rust library
        println!("System unrar not available, using built-in extractor...");
        Self::extract_rar_rust(archive_path, extract_dir)
    }

    /// Extract RAR using system unrar command
    fn try_system_unrar(archive_path: &Path, extract_dir: &Path) -> Result<usize, String> {
        // Check if unrar is installed
        if !Self::check_command_exists("unrar") {
            return Err("unrar not installed".to_string());
        }

        // Create extraction directory
        fs::create_dir_all(extract_dir)
            .map_err(|e| format!("Failed to create extraction directory: {}", e))?;

        // Extract archive
        let output = Command::new("unrar")
            .arg("x") // Extract with full paths
            .arg("-y") // Yes to all prompts
            .arg("-o+") // Overwrite existing files
            .arg(archive_path)
            .arg(extract_dir)
            .output()
            .map_err(|e| format!("Failed to run unrar: {}", e))?;

        if !output.status.success() {
            return Err(format!(
                "unrar extraction failed: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }

        split_flattened_names(extract_dir);

        // Count extracted files
        let count = WalkDir::new(extract_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().is_file())
            .count();

        Ok(count)
    }

    /// Extract RAR using Rust unrar crate
    fn extract_rar_rust(
        archive_path: &Path,
        extract_dir: &Path,
    ) -> Result<(usize, ExtractionMethod), String> {
        use unrar::Archive;

        fs::create_dir_all(extract_dir)
            .map_err(|e| format!("Failed to create extraction directory: {}", e))?;

        let mut count = 0;
        let mut archive = Archive::new(archive_path)
            .open_for_processing()
            .map_err(|e| format!("Failed to open RAR archive: {}", e))?;

        loop {
            match archive.read_header() {
                Ok(Some(header)) => {
                    let entry = header
                        .entry()
                        .filename
                        .to_str()
                        .ok_or("Invalid filename in RAR")?
                        .to_string();

                    let target = if header.entry().is_directory() {
                        None
                    } else {
                        let rel = sanitize_entry_path(&entry);
                        if rel.is_none() {
                            println!("⚠ Skipping unsafe archive entry: {}", entry);
                        }
                        rel
                    };

                    match target {
                        Some(rel) => {
                            let output_path = extract_dir.join(rel);
                            if let Some(parent) = output_path.parent() {
                                fs::create_dir_all(parent).ok();
                            }

                            archive = header
                                .extract_to(&output_path)
                                .map_err(|e| format!("Failed to extract RAR file: {}", e))?;

                            count += 1;
                        }
                        None => {
                            archive = header
                                .skip()
                                .map_err(|e| format!("Failed to skip RAR entry: {}", e))?;
                        }
                    }
                }
                Ok(None) => break, // End of archive
                Err(e) => return Err(format!("Failed to read RAR header: {}", e)),
            }
        }

        Ok((count, ExtractionMethod::RustUnrar))
    }

    /// Check if a command exists in PATH
    fn check_command_exists(command: &str) -> bool {
        Command::new("which")
            .arg(command)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    /// Get user-friendly extraction method name
    pub fn method_name(method: &ExtractionMethod) -> &'static str {
        match method {
            ExtractionMethod::RustZip => "Built-in ZIP",
            ExtractionMethod::RustSevenz => "Built-in 7z",
            ExtractionMethod::RustUnrar => "Built-in RAR",
            ExtractionMethod::SystemP7zip => "System p7zip",
            ExtractionMethod::SystemUnrar => "System unrar",
        }
    }

    /// Check which system extractors are available
    pub fn check_system_tools() -> (bool, bool) {
        let p7zip_available = Self::check_command_exists("7z") || Self::check_command_exists("7za");
        let unrar_available = Self::check_command_exists("unrar");
        (p7zip_available, unrar_available)
    }

    /// Get installation hints for missing system tools
    pub fn get_installation_hints() -> Vec<String> {
        let mut hints = Vec::new();
        let (p7zip, unrar) = Self::check_system_tools();

        if !p7zip {
            hints.push("💡 Install p7zip for faster 7z extraction: brew install p7zip".to_string());
        }
        if !unrar {
            hints
                .push("💡 Install unrar for faster RAR extraction: brew install unrar".to_string());
        }

        hints
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(s: &str) -> PathBuf {
        PathBuf::from(s)
    }

    #[test]
    fn splits_windows_separators_into_a_tree() {
        // The real case: Ragdoll Execution Fix shipped a ZIP made on Windows.
        assert_eq!(
            sanitize_entry_path(r"r6\scripts\Ragdoll Execution Fix\Fix.Global.reds"),
            Some(p("r6/scripts/Ragdoll Execution Fix/Fix.Global.reds"))
        );
    }

    #[test]
    fn leaves_a_well_formed_path_alone() {
        assert_eq!(
            sanitize_entry_path("archive/pc/mod/x.archive"),
            Some(p("archive/pc/mod/x.archive"))
        );
    }

    #[test]
    fn handles_mixed_separators() {
        assert_eq!(
            sanitize_entry_path(r"bin/x64\plugins\cyber_engine_tweaks/mods\M\init.lua"),
            Some(p("bin/x64/plugins/cyber_engine_tweaks/mods/M/init.lua"))
        );
    }

    #[test]
    fn rejects_traversal_in_either_spelling() {
        assert_eq!(sanitize_entry_path("../../etc/passwd"), None);
        assert_eq!(sanitize_entry_path(r"..\..\etc\passwd"), None);
        assert_eq!(sanitize_entry_path("r6/../../escape.reds"), None);
    }

    #[test]
    fn a_dotted_filename_is_not_traversal() {
        assert_eq!(
            sanitize_entry_path("r6/scripts/..hidden..reds"),
            Some(p("r6/scripts/..hidden..reds"))
        );
    }

    #[test]
    fn drops_leading_separators_and_drive_letters() {
        assert_eq!(sanitize_entry_path("/r6/scripts/x.reds"), Some(p("r6/scripts/x.reds")));
        assert_eq!(sanitize_entry_path(r"C:\r6\scripts\x.reds"), Some(p("r6/scripts/x.reds")));
        // A drive letter is only special at the start.
        assert_eq!(sanitize_entry_path("r6/C:/x.reds"), Some(p("r6/C:/x.reds")));
    }

    #[test]
    fn drops_empty_and_current_dir_segments() {
        assert_eq!(sanitize_entry_path("./r6//scripts/./x.reds"), Some(p("r6/scripts/x.reds")));
    }

    #[test]
    fn rejects_names_with_nothing_left() {
        assert_eq!(sanitize_entry_path(""), None);
        assert_eq!(sanitize_entry_path("/"), None);
        assert_eq!(sanitize_entry_path(r"\"), None);
        assert_eq!(sanitize_entry_path("./"), None);
    }

    #[test]
    fn splits_flattened_names_left_by_a_system_extractor() {
        let root = std::env::temp_dir().join(format!("cmm_split_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(r"r6\scripts\Mod\x.reds"), b"code").unwrap();
        fs::write(root.join("readme.txt"), b"text").unwrap();

        let moved = split_flattened_names(&root);

        assert_eq!(moved, 1);
        assert!(root.join("r6/scripts/Mod/x.reds").exists());
        assert!(!root.join(r"r6\scripts\Mod\x.reds").exists());
        assert!(root.join("readme.txt").exists(), "ordinary files stay put");
        fs::remove_dir_all(&root).ok();
    }

    /// End-to-end through the real extractor, on a ZIP whose entry names use
    /// Windows separators — exactly what Ragdoll Execution Fix shipped.
    #[test]
    fn extracts_a_windows_made_zip_into_real_folders() {
        use std::io::Write;
        use zip::write::FileOptions;

        let root = std::env::temp_dir().join(format!("cmm_zip_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let archive_path = root.join("mod.zip");

        {
            let mut zip = zip::ZipWriter::new(fs::File::create(&archive_path).unwrap());
            let opts = FileOptions::default();
            zip.start_file(r"r6\scripts\Ragdoll Execution Fix\Fix.Global.reds", opts)
                .unwrap();
            zip.write_all(b"// redscript").unwrap();
            zip.start_file("archive/pc/mod/ok.archive", opts).unwrap();
            zip.write_all(b"archive bytes").unwrap();
            zip.start_file(r"..\..\escape.reds", opts).unwrap();
            zip.write_all(b"// nope").unwrap();
            zip.finish().unwrap();
        }

        let out = root.join("extracted");
        let (count, _) = ArchiveExtractor::extract(&archive_path, &out).unwrap();

        assert_eq!(count, 2, "the traversing entry is skipped, not extracted");
        let reds = out.join("r6/scripts/Ragdoll Execution Fix/Fix.Global.reds");
        assert!(reds.is_file(), "the backslash path became a real tree");
        assert_eq!(fs::read(&reds).unwrap(), b"// redscript");
        assert!(out.join("archive/pc/mod/ok.archive").is_file());
        assert!(!root.join("escape.reds").exists(), "no escape from the extract dir");
        assert!(!out.join(r"r6\scripts\Ragdoll Execution Fix\Fix.Global.reds").exists());

        fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn refuses_to_split_a_traversing_name() {
        let root = std::env::temp_dir().join(format!("cmm_split2_{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(r"..\..\escape.reds"), b"code").unwrap();

        let moved = split_flattened_names(&root);

        assert_eq!(moved, 0);
        assert!(root.join(r"..\..\escape.reds").exists(), "left inert in place");
        assert!(!root.parent().unwrap().join("escape.reds").exists());
        fs::remove_dir_all(&root).ok();
    }
}
