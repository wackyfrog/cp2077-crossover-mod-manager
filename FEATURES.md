# Crossover Mod Manager - Features

Comprehensive guide to all features and capabilities of the Crossover Mod Manager for Cyberpunk 2077.

## Table of Contents

- [Archive Support](#archive-support)
- [Crossover Compatibility](#crossover-compatibility)
- [File Conflict Detection](#file-conflict-detection)
- [Load Order Management](#load-order-management)
- [Disk Space Checking](#disk-space-checking)
- [Temporary File Cleanup](#temporary-file-cleanup)
- [Unicode Filename Support](#unicode-filename-support)
- [Symlink Detection](#symlink-detection)
- [Red4ext Compatibility](#red4ext-compatibility)

---

## Archive Support

### Multi-Format Extraction

The mod manager supports multiple archive formats using a hybrid extraction approach:

| Format | Extension | System Tool | Rust Fallback | Status          |
| ------ | --------- | ----------- | ------------- | --------------- |
| ZIP    | `.zip`    | Built-in    | `zip` crate   | ✅ Full Support |
| 7-Zip  | `.7z`     | `p7zip`     | `sevenz-rust` | ✅ Full Support |
| RAR    | `.rar`    | `unrar`     | `unrar` crate | ✅ Full Support |

### Hybrid Strategy

1. **Try System Tool First**: Uses command-line tools (`7z`, `unrar`) when available for ~45% faster extraction
2. **Fallback to Rust**: Uses built-in libraries if system tools aren't installed
3. **User Notifications**: Shows which method was used and suggests installing tools for better performance

**Example Output:**

```
📦 Extracting mod archive... (7z format)
✅ Extracted 847 files using system 7z (recommended)
💡 Tip: Install p7zip for faster extraction: brew install p7zip
```

### Magic Byte Detection

The archive extractor detects format by file signature rather than extension:

- **ZIP**: `50 4B 03 04` (PK header)
- **7z**: `37 7A BC AF 27 1C`
- **RAR**: `52 61 72 21 1A 07` (Rar!..)

This ensures correct handling even when files have incorrect extensions.

---

## Crossover Compatibility

### Case Sensitivity Handling

Cyberpunk 2077 on Windows uses case-insensitive paths, but Crossover/Wine on macOS uses case-sensitive filesystems.

**Auto-Detection:**

- Checks for common casing errors (`Archive/`, `archive/`, `ARCHIVE/`)
- Normalizes paths to match game structure (`archive/pc/mod/`)
- Logs warnings about case mismatches

**Example:**

```
⚠️ Case sensitivity issue detected: 'Archive/pc/mod/' should be 'archive/pc/mod/'
🔧 Auto-correcting path casing to match game structure
```

### Path Validation

Validates and corrects common path issues:

- Removes `.cp77` subdirectories (modding tool artifacts)
- Handles `_MACOSX` folders from macOS zip creation
- Detects Red4ext mods and installs to correct location

---

## File Conflict Detection

### Smart Conflict Analysis

Detects when multiple mods modify the same files:

**Archive Conflicts:**

```
⚠️ File Conflict Detection
📦 2 .archive file(s) will override existing mod archives:
  • 'basegame_improved_textures.archive' was previously installed by 'Texture Pack v1'
  • 'basegame_enhanced_models.archive' was previously installed by 'HD Models'

ℹ️  Archive Load Order: Cyberpunk 2077 loads .archive files alphabetically.
💡 The LAST loaded archive wins if multiple mods modify the same assets.
```

**Non-Archive Conflicts:**

```
⚠️ 3 other file(s) will overwrite existing files:
  • 'plugins/cyber_engine_tweaks/mods/config.lua' was previously installed by 'CET Config v2.1'
```

### Conflict Categories

1. **Archive Files**: Special handling due to load order rules
2. **Scripts/Plugins**: Direct file overwrites
3. **Config Files**: User data that may be lost

---

## Load Order Management

### Archive Load Order Detection

Cyberpunk 2077 loads `.archive` files alphabetically. Last-loaded archives override earlier ones.

**Load Order Education:**

```
🔧 To control load order, you can rename archives:
   - Prefix with '0-' to load first (e.g., '0-basegame_textures.archive')
   - Prefix with 'z-' to load last (e.g., 'z-basegame_final.archive')
```

**Current Status:**

- ✅ Conflict detection implemented
- ✅ Load order warnings shown during installation
- ✅ Renaming suggestions provided
- ⏳ Active UI management (planned)

**Example Load Order:**

```
Archives in game/archive/pc/mod/:
  0-basegame_first.archive     ← Loads FIRST
  basegame_middle.archive      ← Loads second
  z-basegame_final.archive     ← Loads LAST (wins conflicts)
```

---

## Disk Space Checking

### Pre-Installation Validation

Checks available disk space before downloading or extracting:

**Safety Multipliers:**

- Downloads: 3x file size (for download + extraction + safety margin)
- Extraction: 2x archive size (for extracted files + safety margin)

**Example:**

```
📦 Download size: 156.8 MB
✓ Sufficient disk space available for download and extraction

Available: 45.2 GB | Required: 470.4 MB (3x safety margin)
```

**Error Handling:**

```
❌ Insufficient disk space!
   Required: 2.3 GB (with 3x safety margin)
   Available: 1.8 GB

💡 Tips to free up space:
   • Delete unused mods from the Mods tab
   • Clear system temporary files
   • Use Disk Utility to free space
```

---

## Temporary File Cleanup

### RAII Pattern

Automatic cleanup using Rust's Drop trait:

```rust
// TempFileGuard automatically cleans up when dropped
let archive_guard = TempFileGuard::new(temp_archive_path, "archive");
// ... installation happens ...
// Cleanup happens automatically even on errors
```

### Safety Features

**Strict Validation:**

- Only removes files matching exact patterns
- Validates numeric IDs (must be purely digits)
- Validates UUID format (8-4-4-4-12 hex structure)
- Never touches user directories

**Protected Patterns:**

```
✅ Will Remove:
  mod_12345_67890.zip
  mod_extract_107_550e8400-e29b-41d4-a716-446655440000

❌ Won't Remove:
  my_mod_backup.zip
  mod_extract_backup/
  user_mod_data/
```

### Cleanup Operations

1. **Automatic (RAII)**: Cleanup on function exit or error
2. **Startup**: Removes orphaned files >1 hour old at app launch
3. **Manual**: User-triggered cleanup via Settings UI

**Startup Log:**

```
🧹 Cleaning up orphaned temporary files...
  🗑️  Removed: /var/folders/.../mod_12345_67890.zip
  🗑️  Removed: /var/folders/.../mod_extract_107_abc123-def4.../
✅ Cleanup complete: 2 files, 1 directory removed
```

---

## Unicode Filename Support

### Automatic Transliteration

Converts non-ASCII characters to filesystem-safe equivalents:

**Transliteration Examples:**

```
café.lua          → cafe.lua
日本語.archive    → ri_ben_yu.archive
Москва.txt        → moskva.txt
naïve.json        → naive.json
```

**Detection & Warnings:**

```
⚠️ Unicode character detected in filename: 'café_mod.archive'
🔧 Sanitizing filename for compatibility: 'cafe_mod.archive'
💡 Crossover/Wine compatibility: Using ASCII-safe filenames
```

### Sanitization Rules

1. **Transliterate**: Non-ASCII → ASCII equivalent (using `unidecode`)
2. **Preserve Structure**: Keep alphanumeric, `-`, `_`, `.`
3. **Convert Spaces**: Space → underscore
4. **Replace Others**: Other characters → underscore

---

## Symlink Detection

### Wine/Crossover Incompatibility

Symbolic links don't work correctly in Wine/Crossover environments.

**Auto-Detection:**

```
⚠️ Symlink Detected: 'mods/link_to_shared_config'
  → Target: '../shared/config'
🔧 Automatically skipping symlink installation (Wine/Crossover compatibility)
💡 Symlinks don't work correctly in Wine/Crossover
   Consider extracting and copying the actual files instead
```

**Handling:**

- Detects symlinks during installation
- Automatically skips symlink files
- Tracks both symlink path and target
- Provides guidance to users about workarounds

---

## Red4ext Compatibility

### Auto-Detection

Recognizes Red4ext mods by:

- Presence of `red4ext` directory
- Files ending in `red4ext.dll`
- `version.dll` in root (Red4ext loader)

### Installation Path

Red4ext mods install to: `{game_root}/red4ext/plugins/{mod_name}/`

**Example:**

```
✅ Red4ext mod detected - installing to red4ext/plugins/ directory
📁 Installing files to: Cyberpunk 2077/red4ext/plugins/CyberEngineTweaks/
```

### Special Handling

- Preserves directory structure within `red4ext/`
- Copies all plugin files (DLLs, configs, assets)
- Warns users about Red4ext dependency

**Warning:**

```
⚠️ This mod requires Red4ext Framework
💡 Install Red4ext from: https://github.com/WopsS/RED4ext
```

---

## Feature Status Summary

| Feature                   | Status      | Version |
| ------------------------- | ----------- | ------- |
| Multi-format archives     | ✅ Complete | v1.0.0  |
| Hybrid extraction         | ✅ Complete | v1.2.0  |
| Case sensitivity handling | ✅ Complete | v1.3.0  |
| File conflict detection   | ✅ Complete | v1.3.0  |
| Load order detection      | ✅ Complete | v1.3.0  |
| Load order UI management  | ⏳ Planned  | TBD     |
| Disk space checking       | ✅ Complete | v1.4.0  |
| Unicode sanitization      | ✅ Complete | v1.3.0  |
| Symlink detection         | ✅ Complete | v1.3.0  |
| Red4ext detection         | ✅ Complete | v1.5.0  |
| Temp file cleanup (RAII)  | ✅ Complete | v1.6.0  |
| Startup cleanup           | ✅ Complete | v1.6.0  |
| Manual cleanup UI         | ✅ Complete | v1.6.0  |

---

## See Also

- [CROSSOVER_COMPATIBILITY.md](CROSSOVER_COMPATIBILITY.md) - Detailed Crossover/Wine compatibility information
- [CHANGELOG.md](CHANGELOG.md) - Version history and feature additions
- [DEVELOPMENT.md](DEVELOPMENT.md) - Development setup and architecture
- [README.md](README.md) - Project overview and installation
