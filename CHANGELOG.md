# Changelog

All notable changes to this project will be documented in this file.

## [1.1.2] - 2026-04-15

### New

- **Auto-sync metadata after install** — picture, summary, and file descriptions fetched from Nexus API immediately after install/update, no manual Netrun needed
- **NXM relay restored** — main app forwards NXM URLs to dev instance via Unix socket for development

### Fixed

- Mod details not refreshing after update (stale selectedMod)
- Wrong sub-mod selected after install/update (searched by name instead of id)
- Same file_id but different mod version now treated as update, not "already installed" error
- Dev window not focused after relay install
- Compiler warnings cleaned up

## [1.1.1] - 2026-04-15

### Fixed

- Filter resets to ALL after updating a mod — now stays on current filter (e.g. UPDATES)

## [1.1.0] - 2026-04-15

### New

- **Database backup/restore** — create, restore, and delete backups of mod database from Config page
- **Mod file validation** — scan all mods, verify files exist on disk
- **CONFIG page** — cleaned up, removed test buttons, removed unused Mod Storage setting

### Fixed

- Misc bugfixes

## [1.0.0] - 2026-04-14

Complete rewrite of the UI and major backend improvements. Fork of [crossover-mod-manager](https://github.com/beneccles/crossover-mod-manager) by Benjamin Eccles.

### New

- **Cyberpunk 2077 UI** — full redesign styled after the game aesthetic, themed vocabulary throughout
- **Mod lifecycle** — install, update, reinstall, and remove mods via NXM deep-link handler ("Download with Mod Manager")
- **Enable/Disable** — toggle mods on/off without removing; soft-delete with history
- **Mod details** — thumbnails, descriptions, version info, changelogs, per-file data from Nexus API
- **Multi-part mods** — parts grouped by Nexus Mod ID with summary and per-file views
- **Search, filter, sort** — search installed mods, filter by status, sort by name or install date
- **Sync with NexusMods** — fetch metadata, check for updates, per-file descriptions and images
- **Startup checks** — auto-detect game path, verify permissions, API key, NXM URL handler
- **Path safety** — traversal protection and game directory validation on all file operations
- **Error handling** — verbose logging, conflict detection, detailed status messages

### Credits

- Original project: [crossover-mod-manager](https://github.com/beneccles/crossover-mod-manager) by Benjamin Eccles
- Built with [Claude](https://claude.ai) by Anthropic
