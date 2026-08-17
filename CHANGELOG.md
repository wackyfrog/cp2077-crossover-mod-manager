# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Documentation

- **The project describes itself in its own words again** — `package.json` and `Cargo.toml` both carried the description of an unrelated project with a near-identical name, word for word, which left search engines treating this repository as a duplicate of it and showing the other one instead. Both now say what this app actually does, and `Cargo.toml` points at its own repository rather than an empty string
- **A landing page on GitHub Pages** — a short page for people who arrive from a search engine rather than from the repository, published from a separate `gh-pages` branch so that nothing under `docs/` is exposed

## [1.6.0] - 2026-08-02

### Upgrading from an earlier version

If a mod of yours installed only *part* of itself into a wrapper folder, no earlier version could tell you — the check skipped it entirely. **Config → Check for unloadable mods** now finds those too, and the startup banner reports them. Nothing is moved until you press a button, and the scan itself changes nothing.

### Fixed

- **A mod that installed *half* of itself into a redundant folder is now reported** — "Check for unloadable mods" only ever looked at mods whose every file sat under one wrapper folder. A mod that put some files where they belong and the rest under a wrapper was skipped entirely, so it never showed up in the scan, in the startup banner, or anywhere else: part of it loaded, the rest was invisible to every loader, and nothing in the app said so. Real case: Guns Redone V3.0 (PL) put one script in `r6/scripts/` and 556 tweaks under a folder named after the variant. These now appear in the scan under their own heading

### New

- **Partial wrappers are repaired one mod at a time, on request** — the layout is genuinely ambiguous: a mod whose base files installed correctly beside an optional FOMOD variant you never selected looks *identical* on disk to one that half-misinstalled. Nothing can tell the two apart, so "Repair now" leaves these alone and each gets its own button instead, with the caveat spelled out. Moving the files activates whatever they contain, which is the user's call to make. Whole-wrapper mods keep repairing in bulk as before

### Changed

- **"Repair mods" in the startup banner now opens the repair** — it and "Open Config" both did the same thing: switch to the Config tab and leave you to find the right scan among the maintenance buttons. The Repair button now runs the scan its warning belongs to and shows the report directly. Every scan is a dry run that changes nothing, so arriving at one is safe
- **The scan states a shared destination once instead of three times** — when every file of a mod lands in the same folder, which is the usual shape, the report printed three near-identical paths that wrapped across half the dialog and said nothing the fourth line didn't. It now names the destination folder once. Mods whose files fan out to several places still list examples as before
- **A report holding only partial wrappers is no longer titled "Unloadable Mods"** — those mods do load, just not all of them, so the title contradicted the first line of its own summary. It reads "Misplaced Mod Files" in that case; anything else keeps the familiar title

### Documentation

- **First-launch instructions now match current macOS** — the README told users to Control-click → Open, which Apple removed as a Gatekeeper bypass in macOS 15 Sequoia. The section now explains why the app is blocked at all (ad-hoc signed, not notarized), gives the System Settings → Privacy & Security → Open Anyway route and the `xattr` one, and cites Apple's own documentation for both

## [1.5.1] - 2026-07-28

### Fixed

- **A successful install no longer ends in a made-up "No response from backend" error** — the Jack In screen ran a two-second timer that fired *after* the backend had already finished, and rewrote whatever was on screen with a failure. Closing the screen in those two seconds was enough to trigger it: the mod was installed, the files were on disk, and the app reported that nothing had happened. Pressing Retry then produced a real but confusing "already installed", because the first install had in fact worked. Nothing needs repairing — mods installed this way were installed correctly. The timer is gone; every outcome now comes from the backend itself
- **A malformed NXM link now says so** — a link matching neither the mod nor the collection shape was logged and then reported as success, which is what the guessing timer above existed to paper over. It is now a proper error naming the shape a link should have

## [1.5.0] - 2026-07-27

### Upgrading from an earlier version

Several of the fixes below only change what happens from now on — files already on disk stay exactly where an earlier version put them. **Config → Maintenance** has a scan for each case, and none of them touch anything until you press Repair:

- **Check for scrambled file paths** — mods from archives packed on Windows (installed by any version up to 1.4). These are the hardest to spot on your own: the file exists at the recorded path, so validation calls it fine, and only the game disagrees
- **Check for unloadable mods** — mods installed inside a redundant wrapper folder (any version up to 1.2)
- **Validate mod files** and **Check for leftover mod folders** — clutter left behind by removals before 1.5: files of "deleted" mods that were never actually deleted (a switched-off mod's files survived removal entirely, untracked), and Cyber Engine Tweaks folders that outlived the mod they belonged to and get flagged at every game launch

The startup banner reports the first two on its own. The README explains what each defect looked like and why nothing warned you at the time.

### New

- **Check for scrambled file paths** (Config → Maintenance) — the repair for the extraction fix below. A mod installed by an earlier version stays broken until its files move, and nothing else in the app would ever mention it: the file exists exactly where the database says it does, so "Validate mod files" calls it fine. The scan finds the recorded paths that were never split into folders, shows where each file would land, and rebuilds the folder structure on request. The mod database is backed up first and updated to match, and a file whose destination is already occupied is skipped rather than overwritten. The startup check reports these mods too
- **Check for leftover mod folders** (Config → Maintenance) — CET writes its own database, log, and often a settings file into a mod's folder while the mod runs, and none of those belong to the mod as far as the manager is concerned. They keep the folder alive after a removal, so it lingers with no mod in it and gets flagged at every launch. The scan lists what each leftover folder holds and how big it is, and you pick what goes. Folders holding settings start unchecked, since deleting those throws configuration away, and a folder containing files that belong to an *installed* mod is never listed at all — mods do ship presets for one another, and those must not be swept up. The startup check reports leftovers too. Ownership is re-checked at deletion time, so a mod reinstalled between the scan and the click is safe

### Fixed

- **A failed install now shows the failure** — if the Jack In screen wasn't already open when an install started, the failure arrived and was wiped a moment later by the screen's own reset, leaving an empty "paste an NXM link" prompt. The install had really run and really failed, with nothing on screen to say so
- **The Jack In screen stops showing an old failure over a new install** — once anything had failed, that screen kept its FAULT DETECTED header and its Dismiss/Retry buttons for good: a later download's progress ran underneath them, as though the download itself were failing. Starting anything new now clears the previous run
- **Retry can no longer hijack a running download** — with those stale buttons on screen, pressing Retry mid-download resubmitted the *previous* mod and blanked out the running one's name and log. The download itself survived (a second install is refused), but it was left unidentifiable, and the natural next click cancelled it. Retry is now unavailable while anything is in flight
- **Dropping another archive after a failed one works** — the app counted a finished install as still busy, so the second archive was silently refused; the explanation went to the status bar, which the full-screen Jack In panel covers. Dropping an archive onto the NXM input screen also left the sideload form stacked underneath, invisible. Both now hand over to the new archive
- **Download links no longer appear in full in the log** — an NXM link carries a time-limited download key in its query string, and two log lines wrote the link out verbatim. Log files and screenshots get attached to bug reports as they are, so the key went with them. The mod and file are still named; the key is not
- **You can see when a second download is turned away** — clicking "Download with Mod Manager" again during an install was already refused, but the notice went only to the covered status bar, so nothing at all appeared to happen. It now shows in the Jack In log as its own line, without disturbing the install that's still running
- **Mods from archives packed on Windows now install into real folders** — some archives store their entries as `r6\scripts\Mod\file.reds`, with backslashes. macOS treats a backslash as an ordinary character in a filename, so the whole path was written to disk as the *name* of a single file sitting loose in the game folder, and the folders the mod needed were never created. The mod showed as installed and enabled, and every check agreed the file was present — but no loader could find it, so the mod did nothing in the game. Both separators are now recognised when unpacking, in all five extraction paths, so the archive's structure is rebuilt as intended
- **Archives can no longer write outside the folder they're unpacked into** — an entry named `../../something` was joined onto the extraction path as-is. Such entries are now refused, and the file is skipped with a note in the log

- **Deleting a switched-off mod now really deletes it** — an unslotted mod keeps its files on disk under a `.disabled` suffix, but removal only ever looked for the active filenames. It found nothing, deleted nothing, and marked the mod as removed anyway: every file stayed on disk, and with the record emptied the manager could never see them again. Removal now matches both the active and the ghosted name, and deletes both when both are there. The symptom in-game was Cyber Engine Tweaks logging *"Ignoring mod which does not contain init.lua!"* for each abandoned folder at every launch
- **"Validate mod files" no longer calls every switched-off mod broken** — it checked each file by its active name only, so an unslotted mod, whose files sit on disk under a `.disabled` suffix by design, came back as entirely missing. One install reported *910 missing files in 7 mods* with nothing actually missing; the biggest "loss" was simply the biggest mod the user had switched off. Files are now judged against the mod's own state, so ghosted files of an unslotted mod count as present
- **New in that check: files that are on disk but in the wrong state** — a slotted mod whose files are ghosted does nothing in the game, and an unslotted mod whose files are active runs regardless of what the list says. Neither is a missing file and nothing else would tell you, so they are now reported in their own right, marked `~` rather than `×`. Toggling the mod off and on again puts its files back in step

- **Removing a mod now clears the folders it emptied** — deleting a mod's files left its folders standing, and Cyber Engine Tweaks logs *"Ignoring mod which does not contain init.lua!"* for every folder that has no mod in it. Emptied folders are now swept as part of the removal, stopping at anything that still holds content
- **A mod is no longer reported as removed when its files are still there** — if a file can't be deleted (locked by the running game, no permission), the mod now stays in the list with its file list narrowed to exactly what survived, so you can see what happened and retry. Previously the record was flatlined regardless, and anything left behind became untracked. A file that was already gone is not treated as a failure — it just means there was nothing left to delete

## [1.4.0] - 2026-07-26

### Changed

- **Legibility pass across the whole UI** — text is larger throughout, and contrast is substantially higher. The dim red used for mod versions measured 2.44:1 against the background (well under the 4.5:1 that body text needs) and was the hardest thing in the app to read; it and the other muted colours have been lifted, along with 41 semi-transparent text colours in the overlays that were washing out to under 3.6:1. Borders and glows keep their original values, so the look is unchanged
- **Menu reorganised** — order is now Chrome / Jack In / Netrun / Config / About. **Netrun** moved up from the mod-list footer into the main menu, and **Sideload** moved from the menu into the **Jack In** screen, next to the NXM link field — both are ways to install a mod, so they now sit together
- **Escape closes things again** — it only ever worked while a text field had focus, so pressing it after clicking anywhere else did nothing. It now closes whatever is in front, wherever the focus happens to be, including **Netrun**, and clears the mod search when nothing is open. Mid-install and mid-sync it is ignored, so you can't accidentally hide work that's still running
- **The way out is visible** — the exit button in **Jack In** and **Netrun** looked exactly like every other button on those screens, which made leaving them a guessing game. It now stands apart, and carries an `esc` hint so the shortcut is discoverable
- **Splash screen** — click anywhere to skip it, and it's a quarter shorter

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
