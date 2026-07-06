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
