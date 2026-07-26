# Setup & Troubleshooting

How to configure Crossover Mod Manager and fix the most common problem:
**"the manager says the mod is installed, but nothing shows up in the game."**

<a id="toc"></a>
## Contents

- [Requirements](#requirements)
- [First-time setup](#setup)
- [Verify it actually works](#verify)
- [Troubleshooting: mods don't appear in the game](#not-appearing)
- [Troubleshooting: files are there, but mods don't load in-game](#not-loading)
- [Troubleshooting: installed by v1.2 or earlier into a wrapper folder](#wrapper-folder)
- [Configuring the CrossOver bottle for advanced mods](#bottle-config)
- [Mod-type compatibility under CrossOver](#compatibility)
- [Community resources](#help)
- [Where the manager stores things](#storage)
- [How the game directory is laid out](#layout)
- [Collecting logs for a bug report](#logs)

---

<a id="requirements"></a>
## Requirements

- macOS 11.0+ (Apple Silicon)
- [CrossOver](https://www.codeweavers.com/crossover) 25+
- Cyberpunk 2077 installed **inside a CrossOver bottle** (Steam, GOG, or Epic)
- A [NexusMods](https://www.nexusmods.com) account (API key + "Download with Mod Manager")
- Optional, for faster/robust extraction of `.rar` / `.7z` mods:
  `brew install p7zip unrar`

---

<a id="setup"></a>
## First-time setup

1. **Launch the app.** On first run it tries to auto-detect the game and registers
   the `nxm://` URL handler.

2. **Set the game path** — Config → *Game path*:
   - Click **Auto-Detect**. As of v1.2.0 the manager scans every CrossOver bottle
     (Steam / GOG / Epic / custom) and, if it finds more than one installation,
     lets you **pick the one you actually launch**.
   - If Auto-Detect finds nothing, click **Browse** and select the game folder
     manually. The correct folder is the one that contains
     `bin/x64/Cyberpunk2077.exe` (see [layout](#layout)).
   - On **Save**, the manager warns you if the path doesn't look like a real
     Cyberpunk 2077 install.

3. **Add your NexusMods API key** — Config → *NexusMods API key*
   (get one at [nexusmods.com/users/myaccount?tab=api+access](https://www.nexusmods.com/users/myaccount?tab=api+access)).
   Required for sync and for downloading via NXM links.

4. **Install a mod** — on NexusMods, open a Cyberpunk 2077 mod and click
   **"Download with Mod Manager"**. The app downloads, extracts, and installs it
   into the game directory, then tracks it.

> **Changing the game path later?** If you already have mods installed and you
> change the game path, v1.2.0 offers to **move (or copy) your installed mod
> files** to the new location, so you don't end up with mods stranded in the old
> folder. Choose *Move* to relocate cleanly, *Copy* to keep the old files as a
> backup, or *Just change path* to leave files untouched.

---

<a id="verify"></a>
## Verify it actually works

After installing one mod, confirm the files really landed in the game:

1. Open the game folder from [where the manager stores things](#storage) —
   specifically the path shown in Config as *Game path*.
2. Check that new files appeared under, e.g., `archive/pc/mod/` (most mods) or
   `bin/x64/`, `r6/scripts/`, `red4ext/plugins/` depending on the mod type.
3. Cross-check with the **install log** (see [logs](#logs)): it prints
   `🎯 Target game directory: …`, then `✓ Extracted N files …` and
   `✓ Installed N files to game directory`. **N must be greater than 0.**

If both the log **and** the folder show the files, the manager did its job. If
the mod still doesn't work in-game, jump to [mods don't load](#not-loading).

---

<a id="not-appearing"></a>
## Troubleshooting: mods don't appear in the game

Symptom: the manager reports success, but the game shows no mods **and** the mod
files aren't in the game folder you're looking at.

Almost always this is a **game-path mismatch** — the manager copied files into
one folder while you're launching/browsing a different one.

### Step 1 — Find where the game really is

In Steam or GOG Galaxy: right-click **Cyberpunk 2077** → *Manage → Browse local
files*. That opens the **real** install folder inside the bottle. It must contain
`bin/x64/Cyberpunk2077.exe`. Note this path.

### Step 2 — Compare with what the manager uses

Open `~/.crossover-mod-manager/settings.json` and look at `game_path`.

- If `game_path` **differs** from the folder in Step 1 → that's the cause. Mods
  were being copied to the wrong place.

### Step 3 — Point the manager at the right folder

Config → *Game path* → **Browse** → select the folder from Step 1 → **Save**.

- **v1.2.0:** on Save you'll be offered to **relocate** the already-installed
  mods to the correct folder — choose **Move**. Done.
- **Older versions:** after fixing the path, **reinstall** each mod (download it
  again via "Download with Mod Manager") so the files land in the right place.
  Then remove the stray files left in the old folder (subfolders
  `archive/pc/mod`, `r6/scripts`, `red4ext`; be careful inside `bin/x64`).

### Step 4 — Check the extraction count

In the install log, if you see `✓ Extracted 0 files`, the archive didn't unpack.
This happens with some `.rar` / `.7z` files when the system tools are missing:

```bash
brew install p7zip unrar
```

Then reinstall the mod. (v1.2.0 fails loudly on a zero-file extraction instead of
reporting a phantom install.)

### Step 5 — Validate files on disk

Config → **Mod file validation** ("Scan") lists any tracked files that are
missing from disk, per mod — a quick way to see which mods are broken.

---

<a id="not-loading"></a>
## Troubleshooting: files are there, but mods don't load in-game

If the files **are** in the game folder but the mods have no effect, the issue is
the mod-loader chain in the bottle, not the manager:

- **Script mods (`.reds` / Redscript)** need **RED4ext + Redscript** installed.
  Look for `bin/x64/version.dll` (the RED4ext loader) in the game folder.
- **Cyber Engine Tweaks (CET)** — in-game, the tilde key `~` should open the CET
  console. If it doesn't, CET isn't loading.
- **REDmod mods** (installed under the `mods/` folder) require launching the game
  with the `-modded` parameter, or deploying REDmod.
- Make sure the bottle's Wine/Windows version is compatible — the manager logs
  the detected version during install.

Install the required loaders (RED4ext, Redscript, CET) as mods **first**, then
your content mods.

---

<a id="wrapper-folder"></a>
## Troubleshooting: installed by v1.2 or earlier into a wrapper folder

Applies only to mods installed with **v1.2 or earlier**. Those versions copied an
archive's layout exactly as it came, so a mod packaged inside one folder named
after itself landed a level too deep:

```
{game}/Combat/r6/tweaks/…      <- where it went
{game}/r6/tweaks/…             <- where the loaders look
```

The mod shows as installed and enabled, every file really is on disk, and
nothing reports an error — loaders simply never look inside `{game}/Combat/`.

**How to tell.** Look at the top level of the game folder. Alongside the real
game directories (`archive`, `bin`, `engine`, `mods`, `r6`, `red4ext`, plus GOG
or Steam files), a folder named after a mod is the giveaway. To confirm a
specific mod, check whether it turns up where its loader scans:

```sh
# CET mods
ls "<game>/bin/x64/plugins/cyber_engine_tweaks/mods"
# TweakXL tweaks
ls "<game>/r6/tweaks"
```

**How to fix.** Update to v1.3+ and use **Config → Check for unloadable mods**,
which lists what would move before you commit, then **Repair now**. The mod
database is backed up first, into `~/.crossover-mod-manager/backups/`. Restart
the game afterwards.

Repair skips anything ambiguous — multi-variant and FOMOD archives, and folders
with no recognisable game directory inside — so it may report fewer mods than
you see suspicious folders. Reinstalling a mod fixes it too, since new installs
strip the wrapper.

---

<a id="bottle-config"></a>
## Configuring the CrossOver bottle for advanced mods

Some loaders need a one-time Wine configuration inside the bottle before they
work. Open the bottle's Wine settings once: **CrossOver → right-click your bottle
→ Wine Configuration** (or *Run Command → `winecfg`*).

### Cyber Engine Tweaks (CET): DLL overrides

CET injects through `version.dll` and `winmm.dll`. Wine must be told to use the
mod's native DLLs instead of its built-in stubs, otherwise CET fails to load
silently (the `~` key opens no console):

1. Open **Wine Configuration** for the bottle.
2. Go to the **Libraries** tab.
3. Under *New override for library*, add **`version`** → click **Add** → set it
   to **"Native then Builtin"**.
4. Add **`winmm`** the same way → **"Native then Builtin"**.
5. Click **Apply**, then **OK**, and restart the game launcher (GOG/Steam/Epic).

### RED4ext on CrossOver

RED4ext is native code injection, so it needs a bit more setup — but it *can*
work on CrossOver with the right configuration. The manager prints these same
steps in the install log when it detects a RED4ext mod:

1. Set the bottle to **Windows 10** — Wine Configuration → **Applications** tab →
   *Windows Version* → **Windows 10**.
2. Add a **`version`** library override → **"Native then Builtin"** (Libraries
   tab, as above).
3. Install the **Visual C++ 2019/2022 Redistributables** inside the bottle.
4. Verify **`version.dll`** is in the **game root** (not `bin/x64/`) — the
   manager places it there automatically.

Click **Apply**, then restart the launcher. If a RED4ext mod still crashes the
game on startup, the VC++ Redistributable is the usual missing piece. When a
mod is available as a Redscript or CET-based version, that's easier to set up.

### Wine / Windows version

Most Cyberpunk 2077 mods expect **Windows 10**. The manager logs the bottle's
detected Windows version during install; if it isn't Windows 10, change it:

Wine Configuration → **Applications** tab → *Windows Version* → **Windows 10** →
**Apply**, then restart the game launcher.

---

<a id="compatibility"></a>
## Mod-type compatibility under CrossOver

Not every kind of mod behaves the same through Wine:

| Compatibility | Mod types | Notes |
| --- | --- | --- |
| ✅ Excellent | `.archive` mods, Redscript (`.reds`), REDmod, texture/model swaps | Pure assets or scripts, no native runtime code |
| ⚠️ Good (needs config) | Cyber Engine Tweaks (CET), TweakXL, ArchiveXL | Work well after the bottle configuration above |
| ❌ Limited | RED4ext, native `.dll` mods, anti-cheat mods | Native/kernel code that Wine often can't translate |

---

<a id="help"></a>
## Community resources

For CrossOver/Wine-specific problems beyond the manager:

- [WineHQ AppDB](https://appdb.winehq.org/) — Wine compatibility database
- [CodeWeavers Forums](https://www.codeweavers.com/support/forums) — CrossOver-specific help
- [r/Crossover](https://reddit.com/r/Crossover) — community support

---

<a id="storage"></a>
## Where the manager stores things

All manager data lives in `~/.crossover-mod-manager/`:

| File | What it is |
| --- | --- |
| `settings.json` | Game path, mod storage path, API key, flags |
| `mods.json` | The installed-mods database (names, versions, tracked file paths) |
| `backups/` | Database backups (Config → backup/restore) |
| `logs/` | Activity log `app.log` (+ rotated `app.log.1`…`app.log.5`) |

The **game files themselves** are copied into the folder set as *Game path* in
Config — inside your CrossOver bottle, typically:

```
~/Library/Application Support/CrossOver/Bottles/<Bottle>/drive_c/.../Cyberpunk 2077
```

---

<a id="layout"></a>
## How the game directory is laid out

A valid Cyberpunk 2077 install (and where the manager places mod files by type):

```
Cyberpunk 2077/
├── bin/x64/               # executables, RED4ext core, version.dll, .dll plugins
│   └── Cyberpunk2077.exe  # ← the manager uses this to validate the path
├── archive/pc/mod/        # .archive mods (the majority)
├── r6/scripts/            # .reds Redscript mods
├── red4ext/plugins/       # RED4ext plugins
├── engine/config/         # config mods
└── mods/                  # REDmod mods (need -modded to load)
```

If the folder you selected has no `bin/x64/Cyberpunk2077.exe`, it's the wrong
folder — the manager will refuse to install there in v1.2.0.

---

<a id="logs"></a>
## Collecting logs for a bug report

The manager keeps a detailed activity log, both in the app and on disk.

**On disk:** every log line is also written to
`~/.crossover-mod-manager/logs/app.log` (it survives quitting the app, and each
launch adds a `session start` marker). If it ever grows past ~10 MB it's rotated
**at startup** to `app.log.1` … `app.log.5` in the same folder (never
mid-session). This is the easiest thing to share.

**In the app** — open the log panel (bottom of the window):

1. Filter by level (errors/warnings) and category (installation) if needed.
2. Reproduce the problem (e.g. install one mod) so the relevant lines appear.
3. Click **Copy** to copy the (filtered) log to the clipboard, or **Show in
   Finder** to jump straight to `app.log` for attaching to a bug report.

For an install issue, the important lines are:
- `🎯 Target game directory: …` (where files are being written)
- `✓ Extracted N files using …`
- `✓ Installed N files to game directory`
- `Auto-detect: found N installation(s) → …` and any `Relocate (…)` line
- any `⚠️`/`❌` warnings or errors

Also include:
- The `game_path` value from `~/.crossover-mod-manager/settings.json`
- The **real** game folder from Steam/GOG *Browse local files*
- The app version (About tab)

> **Tip:** use the log panel's **Clear** first, then reproduce the problem, then
> **Show in Finder** — `app.log` will contain just that clean run.
