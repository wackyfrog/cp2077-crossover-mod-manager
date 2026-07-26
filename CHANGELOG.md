# Changelog

All notable changes to this project will be documented in this file.

## [1.3.0] - 2026-07-26

### New

- **Sideload — install a mod from an archive on disk** — not every mod on NexusMods offers "Download with Mod Manager"; some are manual download only, and until now those archives were dead ends. A new **Sideload** button (header and mod-list footer) opens a `.zip`/`.7z`/`.rar` from anywhere on disk, and you can also **drag & drop** the archive straight onto the window. Name, version, author, and Mod ID are pre-filled by parsing the NexusMods filename (e.g. `Some Mod-1464-1-0-1612607650.7z` → *Some Mod*, v1.0, mod 1464) and shown for you to correct before installing. From there it runs the exact same install pipeline as an NXM download — same progress screen, same safety checks, same mod list entry. Keeping the Mod ID lets **Netrun** fetch the thumbnail, summary, and update alerts for a sideloaded mod just like any other; clearing it keeps the mod fully local
- Your original archive is left untouched in place — sideloading reads it where it sits and never moves or deletes it

### Fixed

- **Mods wrapped in a redundant folder now install correctly** — an archive laid out as `ModName/r6/tweaks/…` used to be copied verbatim to `{game}/ModName/r6/…`, where the game never looks: the mod showed as installed and enabled but was silently inert. The wrapper folder's *contents* are now merged into the game directory, so the files land in `{game}/r6/…` as intended. Applies to NXM downloads as well as sideloads; archives with several top-level folders, or with no recognisable game folder inside, are left exactly as they are
- **One install at a time** — clicking "Download with Mod Manager" twice, or dropping an archive while a download is still running, used to start a second installation on top of the first. Both wrote into the same game folder and the same mod database at once, which could interleave files and mix up mod details. A second request is now turned away with a note in the status bar ("*download ignored · already jacking in*") while the running install carries on untouched, and the Jack In / Sideload buttons grey out for the duration. Collections still install their mods one after another as before
- **Repair for mods already installed that way** — the fix above only helps new installs, so anything installed by an earlier version stays broken until its files move. The startup check now reports these mods ("*N mods installed inside a redundant folder and can't be loaded by the game*"), and **Config → Check for unloadable mods** shows exactly what would move before you commit to it. Repairing backs up the mod database first, moves each file to where the loaders actually look, updates the database to match, and clears out the emptied folders. Mods that merely *look* similar — multi-variant and FOMOD archives, LUT packs that ship a `Textures/` or `Data/` tree — are deliberately left alone, and a file whose destination is already occupied is skipped rather than overwritten

## [1.2.0] - 2026-07-06

### New

- **Smarter game-path detection** — Auto-Detect now scans every CrossOver bottle (Steam, GOG, Epic, and custom bottle names) instead of a fixed list of paths, including a bounded fallback search for non-standard layouts. When more than one installation is found, you pick the one you actually launch
- **Mod relocation on path change** — changing the game path in Config now offers to move (or copy) already-installed mod files to the new location, so switching to the correct bottle no longer leaves mods behind or orphaned. Ghosted (unslotted) files are relocated too, and you get a summary report
- **Persistent self-check banner** — startup self-check (game path validity, write access, API key, NXM handler) now shows as a dismissible banner with a jump to Config, instead of a transient footer status that was easy to miss. Re-runs on window focus **and right after saving settings**, so fixing the path clears the banner immediately (no restart needed)
- **Setup & Troubleshooting guide** — new [document](SETUP_AND_TROUBLESHOOTING.md) (linked from the README) covering setup, install verification, and fixing a wrong game path
- **On-disk log file** — all activity is now mirrored to `~/.crossover-mod-manager/logs/app.log` (survives restarts, with a `session start` marker per launch; rotated at startup to `app.log.1`…`app.log.5` if it exceeds ~10 MB, never mid-session), with **Copy** and **Show in Finder** buttons in the log panel, so logs are easy to share for bug reports
- **More diagnostic logging** — Auto-Detect logs how many installations it found and where; relocation logs its move/copy summary

### Fixed

- **Install preflight** — installing now refuses a game path that exists but isn't a real Cyberpunk 2077 install (missing `bin/x64/Cyberpunk2077.exe`), instead of silently copying files where the game can't see them and reporting success
- **Zero-file guard** — an extraction or install that produces no files now fails loudly instead of registering a phantom mod with nothing on disk (e.g. empty/encrypted archives)
- **Game-path validation on save** — Config warns (with override) if the entered path doesn't look like a Cyberpunk 2077 installation
- **Install log** now shows the absolute target game directory, making misconfigured paths easy to spot
- **Startup health check** flags a configured path that isn't a valid Cyberpunk 2077 install
- **NXM handler self-check fixed** — the check now uses LaunchServices directly (was a PyObjC script that isn't installed on most Macs and always false-positived "not registered"); a **Register NXM handler** button in the banner registers the app in one click, no `duti` needed
- **Game path no longer silently overwritten** — saving settings used to reset the internal `first_run` flag (the frontend never sends it, so it defaulted back to `true`), which re-ran auto-detection on the next launch and quietly replaced the configured game path. `first_run` is now preserved server-side, and first-run auto-detect only seeds an *empty* path
- **App bundle identifier** changed to `com.wackyfrog.crossover-mod-manager` (was the upstream `com.beneccles…`), matching the fork. After updating, use the **Register NXM handler** button once to re-associate `nxm://` links
- **App version in the UI** (header, Jack In screen, footer, About) now reflects the actual build version instead of a hardcoded string
- **Jack In screen** now explains the automatic "Download with Mod Manager" install path, not just manual NXM paste

## [1.1.3] - 2026-05-25

### Fixed

- **Update no longer re-slots a ghosted mod** — updating/reinstalling a mod that was unslotted (disabled) used to force it back to slotted; the prior slot state is now preserved (reinstalling a flatlined mod still re-slots it)
- Ghosted mods stay ghosted on disk after an update — freshly installed files are re-disabled to match the preserved state
- Stale-file cleanup during update now also removes orphaned `.disabled` files from the previous version

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
