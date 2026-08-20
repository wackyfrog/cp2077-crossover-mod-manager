Cyberpunk 2077 has run natively on macOS since July 2025 — and almost nothing
the modding scene builds runs with it. The Mac build has no CET, no RED4ext, no
ArchiveXL or TweakXL; the [modding wiki](https://wiki.redmodding.org/cyberpunk-2077-modding/for-mod-users/users-modding-cyberpunk-2077/modding-on-macos)
lists redscript alone as unofficially supported, which rules out most of what
people actually install. So the mods stay where the Windows build is: inside a
CrossOver bottle.

Which leaves the managing of them. Vortex, MO2 and the Nexus app are Windows
programs, so the usual answer is to run one *inside* the same bottle it is
meant to manage — people do get that working. It is still a Windows file
manager wearing a Windows theme, sitting in a Wine prefix, on your Mac.

**Crossover Mod Manager is a native macOS app** that reaches into the bottle
from outside. It speaks NXM links, so *Download with Mod Manager* on
[NexusMods](https://www.nexusmods.com/cyberpunk2077) hands the mod straight to
it, and it writes files where CET, TweakXL and REDmod actually look for them.

It is a pet project, and it looks like one on purpose: the whole thing is
styled after Night City rather than after a file browser. The part underneath
is meant to be dull and correct.

[**Download the latest release**](https://github.com/wackyfrog/cp2077-crossover-mod-manager/releases/latest)
 · [Source on GitHub](https://github.com/wackyfrog/cp2077-crossover-mod-manager)

![The mod list, with details, versions and update alerts](https://raw.githubusercontent.com/wackyfrog/cp2077-crossover-mod-manager/main/docs/screenshots/look.png)

## What it does

- **One-click installs from NexusMods.** Click *Download with Mod Manager* in
  your browser; the app catches the NXM link, downloads, unpacks and installs.
- **Handles ZIP, 7z and RAR**, including archives packed on Windows that store
  their paths with backslashes — on macOS those become part of the filename
  instead of folders, and the mod lands as one long file the game can't follow.
- **Keeps the game's folder casing straight.** Archives spell the game's own
  folders every which way — `Archive/`, `R6/`, `BIN/`. The installer writes
  them under the casing the game uses, so a mod's files join the real folders
  instead of forming a set of near-identical ones beside them.
- **Checks for updates** against NexusMods and tells you which of your mods
  have a newer version.
- **Uninstalls what it installed.** Every file the app puts on disk is tracked
  and goes with the mod, emptied folders included — the switched-off ones too,
  whose files sit on disk under a different name.
- **Repairs mods that installed wrong**, including ones put there by older
  versions of this app: a scan reports what it would move before anything is
  moved.

## What you need

- macOS 11 or later, Apple Silicon
- [CrossOver](https://www.codeweavers.com/crossover) 25 or later
- The **Windows** build of Cyberpunk 2077, installed in a bottle
- A [NexusMods](https://www.nexusmods.com) account, for the API key

## First launch

The app is ad-hoc signed and not notarized — there is no paid Apple Developer
account behind this project — so macOS blocks it the first time. On macOS 15
Sequoia and later, open it once, let it be blocked, then go to **System
Settings → Privacy & Security → Open Anyway**. The
[README](https://github.com/wackyfrog/cp2077-crossover-mod-manager#first-launch-on-macos)
covers the Terminal route and why Control-click → Open no longer works.

Free and open source, MIT licensed. Bugs and feature requests go to
[Issues](https://github.com/wackyfrog/cp2077-crossover-mod-manager/issues).

*Not affiliated with CD PROJEKT RED, CodeWeavers or Nexus Mods.*
