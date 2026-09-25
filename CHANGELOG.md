# Changelog

All notable changes to this project will be documented in this file.

## [Unreleased]

### Changed

- **The log shows long lines whole** — a line longer than the panel, such as a removed file's full path, was cut off with "…" and there was no way to read the rest except copying the log. A **Wrap** button now switches between wrapping long lines and scrolling sideways (the default), and remembers the choice. The category filter also gained **Removal** and **Sync**
- **A mod installed as several parts is updated part by part** — the group's details had an **Update** button, but a group isn't one file: the button couldn't tell which file to ask NexusMods for, opened the Files tab, and whatever you picked there could be installed as another part beside the old one. The button is gone; the group lists which parts are OUTDATED and says to update each from its own row, where Update knows the file to fetch
- **Updates keeps what's left to update on top** — mods you updated stay in the **Updates** filter until you quit, so the row you just updated doesn't vanish from under the cursor; after twenty updates in a row, though, they were mixed in with the ones still OUTDATED and you had to pick the remaining ones out by their badge. Mods still OUTDATED now come first, and the ones updated this session fold into an **Updated this session (N)** row at the bottom. It opens by itself while the selected mod is in it — right after an update — and closes again when you move on to the next one. A mod installed as several parts stays on top while any part is still OUTDATED
- **Update follows the file the author named as the replacement** — when an author gives a new version a new name and the mod is installed as several parts, Update couldn't tell which file replaces yours and opened the Files tab; picking the new file there installed it as another part beside the old one (Nova LUT 4 next to Nova LUT 3, both replacing the same game files). NexusMods keeps the author's own "this file updates that one" links, and NETRUN now fetches them — one extra request, only for outdated mods without an update target — so Update asks for that file and it replaces your mod in place. A link that ends in a file the author has since retired names nothing, as before

### Fixed

- **Search finds the parts of a mod by their file name** — a mod installed as several parts is listed by the mod's name, with each part under its file's name ("LUT Switcher - Nova LUT Pack"), but search only looked at the mod's name, so typing a part's name found nothing. It now matches the file name too, and shows the mod with the matching parts
- **The details pane no longer shows a mod the list has hidden** — after a search or filter that left the selected mod out, its details stayed on screen beside "Zero hits", as if it were a result. The pane is now empty while the list hides the selection, and the mod comes back when the search is cleared
- **A part's changelog is its own file's history** — for a mod installed as several parts, each part's changelog listed the versions of every file on the mod's page in upload order, so LUT Switcher's core showed its 3.2.0 and 3.1.0 with three LUT packs' versions wedged between them, and every pack's Version row pointed at the core's `→ v3.3.0`, as if the packs had moved to 3.x too. NETRUN now works out which files are successive versions of one download — the same name, the same version tag (`1.4.0n` and `1.4.1n` of the Nova LUT pack, across its rename), or the author's own "this file replaces that one" — and a part shows its own line, with the versions of the mod's other files folded into one row that names the file each came from. A pack's Version row points at its own newest version (`1.4.0n → v1.4.1n`). A mod whose parts all use plain version numbers can't be told apart this way and shows every version as before. Run NETRUN once to see it
- **A mod takes the name it has on NexusMods now** — a mod kept the title its page had on the day you installed it, while its picture and version followed the page: after Nova LUT's author renamed "Nova LUT 3.0 (AgX - HDR Support)" to "Nova LUT 4.0 (AgX - New HDR)", the list showed the 3.0 name over the 4.0 picture and version. NETRUN and Update now take the current title, for every part of the mod. The old name isn't lost: search still finds the mod by it, and hovering the name shows what it used to be called. Mods not from NexusMods keep the name you gave them
- **Ghost and Flatline no longer break another mod that uses the same file** — two mods can ship the same file: a patch that replaces a script of the mod it patches (Radio Works in Menus and RadioExt both install `radioManager.lua`), or two tweak mods writing one YAML. Switching one of them off renamed the file to `.disabled`, and flatlining it deleted the file, while the other mod was still on and counted on it — that mod then quietly stopped working. A file another enabled mod also holds now stays active when you Ghost, and stays on disk when you Flatline; the Flatline confirmation says beforehand how many files will go and which stay for which mod, and when it is done a result window says how many files were deleted, lists the ones kept and for which mod, and opens the log on request. The confirmation also names the part being flatlined when a mod is installed as several. If every other mod holding it is switched off, Flatline ghosts the file instead of deleting it, so switching that mod back on still works. Switching a mod on no longer moves its `.disabled` copy over a file another mod keeps active, and updating a switched-off mod — which installs the new version and ghosts it straight away — leaves active any new file an enabled mod also holds; the log lists it. In a mod's details, **Files** shows how many of its files other mods share, and each such file names them. Config → Validate no longer reports such a file as "active while unslotted"
- **An update logs the old files it removes** — updating a mod deletes the files the new version no longer ships, but the log said nothing about it, so the only way to see what left the game folder was to look on disk. Each removed file is now logged the way Flatline logs its files, along with old files kept because another mod still uses them
- **The installer window is laid out as intended** — the disk image opened with small icons sorted by date and the app's `.app` extension showing; it now opens with large icons, the app beside the Applications folder to drag it onto
- **A mod's picture no longer stays blank after an update** — when an update changed which picture the details show (Nova LUT 3's file had its own image, Nova LUT 4's doesn't, so the mod's picture takes its place), the picture could stay a dark placeholder until you selected the mod again. It now appears as soon as it loads

## [1.7.0] - 2026-09-24

### Upgrading from an earlier version

Updating a mod in an earlier version could quietly reinstall the file you already had and then show the mod as up to date — it happened whenever the author puts the version in the file name. **Run NETRUN once** after upgrading: it finds those mods, marks them OUTDATED and puts back the version that is actually on disk. A banner at startup asks for it, once, if you have an API key and mods from NexusMods; it goes away after a complete NETRUN or when dismissed. Nothing is reinstalled or moved by the check itself. To get the newer version, press Update — after that NETRUN it asks for the file that replaced yours, and opens the Files tab only when it can't tell which that is.

### Fixed

- **The startup banner offers Open Config only when Config is where the fix is** — it sat under every warning, including ones Config has nothing to do with, such as an unregistered NXM handler, which already has its own button. It now appears for the game path, the API key and leftover CET folders
- **Update could reinstall the version you already had and call it updated** — the Update button asks NexusMods for the newest file with the same name as the one installed. Authors who put the version in the file name ("Native Interactions Framework 1.0.5a") never have a newer file of that name, so the button asked for the installed file itself: NexusMods sent the old version back, it was reinstalled, and the mod was then recorded at the mod's latest version with its update flag cleared — up to date on screen, unchanged on disk. Update now asks for the file that replaced yours: the newest file with the same name, or — when the author renamed it, as SPLAT did from "Splat Physics" to "SPLAT Physics Realistic Ragdoll Overhaul" — the one main file uploaded after yours, if the mod is installed as a single part. A file the author has already moved to *Old versions* is never picked, and when a mod has several parts installed a renamed part isn't guessed at (LUT Switcher's packs would otherwise have been "updated" to its core file). Only when nothing is clear does it open the mod's Files tab. An update that brings back the file already installed is reported as such on the finish screen and leaves the version and the update flag as they were
- **Updating to a renamed file no longer leaves the old version installed beside it** — the app recognised an update only by the same file name, so when an author renamed the file ("Splat Physics" became "SPLAT Physics Realistic Ragdoll Overhaul") the new version was installed as a separate part and the old record stayed, OUTDATED, still claiming files the new one had overwritten. Update now tells the installer which mod it is updating, so the file it asked for replaces that mod whatever it is called
- **Updating a Cyber Engine Tweaks mod keeps your settings** — an update removes the old version's files that the new one no longer ships, and for CET mods that included the settings and database the mod writes into its own folder (`user_settings.json`, `db.sqlite3`…), so updating reset the mod to defaults. Inside a CET mod folder only the `.lua` code is now treated as the mod's; the rest stays on disk and stays with the mod
- **Updating a mod no longer reports it as conflicting with itself** — the conflict check after an install compared the new files with every record, including the one being updated, so each update logged its own files as "replaced from other mods" (47 of them for Native Interactions Framework, all its own) and its archive as overriding itself. The record being updated is now left out, so the warning appears only when another mod really held the file
- **An update no longer deletes a file another mod also uses** — when two mods ship the same file and one of them drops it in a new version, updating that one removed the file from disk while the other mod still counted on it. Files another installed mod also claims are now left in place
- **Mods left behind by that bug are found and corrected on the next NETRUN** — when an author releases a new version, NexusMods moves the old file to *Old versions* (or archives it). NETRUN now checks the file you actually have installed: if its author has retired it, the mod is marked OUTDATED whatever the version numbers say, and if the app had recorded a newer version than that file carries, the record takes the file's version — the one on disk. This repairs mods that an earlier release "updated" to the same old file, and also catches updates the author never reflected in the mod's own version number. Optional parts still listed as current keep their version, even when it's older than the mod's. Where the mod's own version isn't actually newer than yours — the author retired the file without bumping it, or let it fall behind (`1.3.2 → v1`) — the Version row says **newer file on Nexus** instead of drawing an arrow to it
- **The changelog shows what changed, not just version numbers** — clicking a mod's version (or the `→ vX` update badge) opened a list of bare versions with nothing under them. NexusMods does send the text for every version, and the multi-part view already rendered it, but the single-mod view kept an older copy of the window that expected a different shape of data and silently dropped every line along with the upload dates

### Changed

- **The Version row unfolds into the changelog** — the Version row carried two dotted links that did the same thing, and nothing else hinted that a changelog was there. Now the whole row is one toggle with an arrow at its end, like **Files**, and unfolds the changelog in place, with no window over the details. It already says how far behind you are (`2.2.10 → v2.2.11 · 1 month ago`); unfolding it shows what lies in between, and while a mod's changelog is being fetched or has failed to arrive, the row says so. Inside, versions run newest upload first, your installed one is marked where it falls — everything above it was uploaded later — and everything below it folds into one "N earlier versions" line. The list is ordered by upload time rather than version number, because authors number versions in ways that don't sort ("0.65" after "0.7", or "2.3.2b" as a variant beside "2.3.2" rather than its successor), and your version is found by the file you actually installed. It deliberately doesn't count how far behind you are: variants, parts and renamed files look alike on Nexus, and any such count would be wrong somewhere. Flatlined mods get the row too
- **The destructive button moved away from the main one** — in a mod's details, **Reinstall / Update** now sits on the right and **Flatline** on its left, so the button the eye lands on first is the one that doesn't delete anything; a flatlined mod's **Jack In** and **Forget** follow the same order
- **Versions without changelog notes show the file's description instead** — some authors never fill in NexusMods' changelog and describe each upload in the file's description; Native Interactions Framework does this for all eleven of its versions, so its changelog was a list of bare numbers. Where a version has no notes, its file description now appears under it, dimmer and labelled **file description**, because across a few hundred mods about half of such descriptions turn out to be install steps or image links rather than changes. A description that repeats one from an older version is static text and is shown only once, on the version that introduced it. Descriptions arrive with the next NETRUN, and an open changelog picks them up as soon as it finishes
- **UPD is now OUTDATED, and a mod you just updated says LATEST** — "UPD" read as a synonym of "updated", the very state it wasn't. Mods with a newer version on Nexus are now marked **OUTDATED**. A mod you update is marked **LATEST** in green until you quit, in the list and in its details, and its Version row says what it was updated from (`1.4 · updated from 1.3`). Working down the **Updates** filter one mod at a time used to lose your place: the mod you had just updated dropped out of the filter, and an update resets the install date that **Recent** sorts by. It now stays in the filter, in the same spot, and the list scrolls to keep the selected mod in view
- **Unfolding a row brings what it opened into view** — opening **Version** or **Files** near the bottom of the details pane added the list below the visible part, so nothing seemed to happen until you scrolled. The pane now scrolls just enough to show it, without pushing the row itself off the top, and stays put when it already fits
- **The author links to their mods on NexusMods** — the Author row now names the NexusMods account that uploaded the mod and links to that account's mods, fetched by NETRUN along with everything else. The mod's own author field is free text and doesn't always match the account (Native Interactions Framework credits "keanuWheeze" but was uploaded by "NexusGuy999"), so when the two differ the credited name moves to the hover hint. Searching by author finds either name. A mod NETRUN hasn't reached yet shows the credited name without a link
- **The mod page link says what it does** — the Mod Page row read just "nexusmods.com", which looked like a caption rather than something to click. It now reads **Open on NexusMods ↗**, the arrow marking that it leaves the app for the browser
- **NETRUN fetches the changelogs, so they always agree with the OUTDATED badge** — the changelog used to be fetched live when opened, while the OUTDATED badge and the `→ vX` version came from the last NETRUN, so a mod could list versions newer than yours without saying an update was available. Now NETRUN brings the changelog along with everything else and keeps it in `~/.crossover-mod-manager/changelogs.json`: opening it is instant, works offline, and shows exactly what the badge was based on. A mod NETRUN hasn't reached yet (or one you open before your first NETRUN) is fetched on its own when you unfold the row, and its OUTDATED badge refreshes with it; a freshly installed mod gets its changelog during install
- **NETRUN makes about ten requests instead of two per mod** — it now asks NexusMods' GraphQL API about 50 mods at a time, versions and changelogs included, rather than making separate requests for each mod's details and files. With a few hundred mods that is roughly 10 requests instead of ~600, and it leaves the hourly API quota for downloads. Mods split into several parts are fetched once, not once per part, and get one line in the NETRUN log ("· 4 parts") instead of a first line followed by a silent jump in the counter. The counter shows the full count from the start, and while a batch is on its way the header says which mods it is asking for. If the GraphQL API fails, that batch falls back to the old per-mod requests
- **NETRUN stops at the NexusMods request limit instead of failing every remaining mod** — hitting the limit used to mark each mod after that point as an error, one by one, while leaving half the list refreshed and half not with no way to tell which. NETRUN now stops there, says when the limit resets, and leaves the unfinished mods exactly as they were. Each mod's version, files and changelog are saved together or not at all
- **A mod installed in several parts shows a version only when the parts share it** — the group row and its Version row showed the version of whichever part the list happened to sort first, so Nova LUT read `v3.0.0` under Recent and `v3.0.0s` under A–Z, and the details pane could disagree with the list. Parts are separate files with versions of their own; when they differ, the row now says **N parts** and the details say *parts differ*, with the newest version on Nexus beside it. Each part's own version is still shown on its line
- **The footer names the commit the app was built from** — the version in the bottom-right corner now reads `v1.7.0 (f6611d3)`, taken from git at build time, so a bug report or a screenshot says exactly which code it came from. A build with uncommitted changes is marked `-dirty`. The build time that used to follow the version moved to its hover hint

### Documentation

- **Future plans in the README** — names the one known gap left: ghosting or flatlining a mod can still take a file another enabled mod shares
- **Problems of 1.4 and earlier moved out of the README** — archives packed on Windows, leftovers of deleted mods and wrapper folders now live in [OLDER-VERSIONS.md](OLDER-VERSIONS.md); the README keeps the 1.6 update problem and a link. Every screenshot is shown at once, without a spoiler
- **The project describes itself in its own words again** — `package.json` and `Cargo.toml` both carried the description of an unrelated project with a near-identical name, word for word, which left search engines treating this repository as a duplicate of it and showing the other one instead. Both now say what this app actually does, and `Cargo.toml` points at its own repository rather than an empty string
- **The landing page and the README no longer open on a false claim** — both led with the idea that Cyberpunk 2077 has no Mac release. It has had a native macOS build since July 2025; what's missing there is the modding stack, since the Mac build loads no CET, RED4ext, ArchiveXL or TweakXL ([the modding wiki](https://wiki.redmodding.org/cyberpunk-2077-modding/for-mod-users/users-modding-cyberpunk-2077/modding-on-macos) lists redscript alone as unofficially supported). That, not the absence of a port, is why a modded playthrough runs the Windows build inside a bottle — and the Requirements now say **Windows build** rather than leaving the reader to guess. The dismissal of running a Windows mod manager inside the bottle is gone too: people do get that working. What is left is the honest reason this app exists — it is native, and it is a pet project dressed for Night City
- **The landing page stopped claiming a case-sensitivity trap that isn't one** — it said the macOS filesystem under Wine distinguishes `Archive/` from `archive/`, which is not what the installer's casing pass is for. It normalises the casing of the game's own folder names so a mod's files join the real folders instead of a near-identical set beside them. The uninstall bullet lost its "leaves the vanilla game untouched" absolute for the same reason: the app tracks and removes what it installed, and the README's own Housekeeping section lists what other things leave behind
- **The README's "What's New" reached 1.6** — it still ended at 1.5 while the release badge read v1.6.0, even though the body already described 1.6 behaviour

- **A landing page on GitHub Pages** — [wackyfrog.github.io/cp2077-crossover-mod-manager](https://wackyfrog.github.io/cp2077-crossover-mod-manager/), a short page for people who arrive from a search engine rather than from the repository, styled with the app's own palette so it doesn't look like a stranger to the screenshot on it. It is published from a separate `gh-pages` branch, because `docs/` in this branch holds working notes rather than user documentation and publishing that folder would put them online. The README links to it from the top and from the Download section

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
