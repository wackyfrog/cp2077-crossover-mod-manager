Cyberpunk 2077 has no Mac release. The way people play it on macOS is inside a
CrossOver bottle — and every mod manager the modding scene uses (Vortex, MO2,
the Nexus app) is a Windows program that expects a Windows filesystem. Running
one *inside* the bottle to manage the game in the same bottle works about as
well as it sounds.

**Crossover Mod Manager is a native macOS app** that reaches into the bottle
from outside. It speaks NXM links, so *Download with Mod Manager* on
[NexusMods](https://www.nexusmods.com/cyberpunk2077) hands the mod straight to
it, and it writes files where CET, TweakXL and REDmod actually look for them.

[**Download the latest release**](https://github.com/wackyfrog/cp2077-crossover-mod-manager/releases/latest)
 · [Source on GitHub](https://github.com/wackyfrog/cp2077-crossover-mod-manager)

![The mod list, with details, versions and update alerts](https://raw.githubusercontent.com/wackyfrog/cp2077-crossover-mod-manager/main/docs/screenshots/look.png)

## What it does

- **One-click installs from NexusMods.** Click *Download with Mod Manager* in
  your browser; the app catches the NXM link, downloads, unpacks and installs.
- **Handles ZIP, 7z and RAR**, including archives packed on Windows that store
  their paths with backslashes — on macOS those become part of the filename
  instead of folders, and the mod lands as one long file the game can't follow.
- **Fixes the case-sensitivity trap.** Windows treats `Archive/` and `archive/`
  as the same folder; the macOS filesystem under Wine does not, and a mod that
  guesses wrong silently never loads. The installer corrects the casing to
  match the game's real layout.
- **Checks for updates** against NexusMods and tells you which of your mods
  have a newer version.
- **Uninstalls cleanly** — every file a mod put on disk is tracked, so removing
  it leaves the vanilla game untouched.
- **Repairs mods that installed wrong**, including ones put there by older
  versions of this app: a scan reports what it would move before anything is
  moved.

## What you need

- macOS 11 or later, Apple Silicon
- [CrossOver](https://www.codeweavers.com/crossover) 25 or later
- Cyberpunk 2077 already installed in a bottle
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
