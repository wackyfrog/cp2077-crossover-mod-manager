# Problems left by older versions (up to 1.4)

These are defects of Crossover Mod Manager 1.4 and earlier, long fixed. Each
could leave something behind on disk that the app still helps you put right.
For the problem in 1.6 and earlier — updates that didn't update — see the
[README](README.md#updates-that-didnt-update).

None of them produces an error anywhere: the files were really copied, the
mod is recorded, the toggle says enabled — and the mod loaders simply never see
them, or leftovers pile up. If a mod isn't working and you used an earlier
version, run the scans under **Config → Maintenance**. They only report;
nothing changes until you press Repair.

<a id="windows-paths"></a>
## Archives packed on Windows (versions up to 1.4)

*Files the game can't follow.*

Some archives store their entries with backslashes: `r6\scripts\Mod\file.reds`
instead of `r6/scripts/Mod/file.reds`. On Windows those are folder separators.
On macOS a backslash is an ordinary character in a filename, so the whole path
became the *name* of one file sitting loose in the game folder, and the folders
it spells out were never created:

```
the archive:                     installed as (wrong):              should have been:
r6\scripts\Mod\file.reds         {game}/r6\scripts\Mod\file.reds    {game}/r6/scripts/Mod/file.reds
                                 (one file, backslashes in name)
```

This one is harder to notice than the wrapper case, because **"Validate mod
files" calls the file present** — it does exist, at exactly the path recorded
for it. Only the loader disagrees.

**How to check and fix it**, in v1.5 or later:

1. The startup banner reports it: *"N mods have M files stored under a
   Windows-style path the game can't follow"*.
2. **Config → Check for scrambled file paths** shows where each file would go.
3. **Repair now** backs up the database, rebuilds the folder structure, and
   updates the records. Then restart the game.

In a real 262-mod library, 1 mod was affected. Reinstalling also fixes it.


<a id="housekeeping"></a>
## Housekeeping (versions up to 1.4)

*Leftovers of deleted mods.*

Clutter earlier versions could leave behind. None of this breaks the game — it
wastes space and produces warnings that look like breakage. Versions up to 1.4
could leave three kinds of leftovers:

- **Files of deleted mods.** Deleting a *switched-off* mod deleted nothing at
  all: its files live under a `.disabled` suffix while unslotted, and removal
  only looked for the active names. The mod vanished from the list while every
  file stayed on disk, now untracked. One library had 18 working `.lua` files
  left over from a single mod this way.
- **Empty mod folders.** Removing a mod left its folders standing, and Cyber
  Engine Tweaks logs *"Ignoring mod which does not contain init.lua!"* for each
  one at every launch. Folders also survive because CET writes its own
  `db.sqlite3`, logs and settings into them — files no mod's manifest knows about.
- **Loose files in the game root.** Some archives ship readmes, `fomod/` option
  trees, or texture folders that no loader reads; those land in the game
  directory alongside the real files.

**To clean up**, in v1.5 or later, under **Config → Maintenance**:

- **Validate mod files** — reports files that are missing, and files that are
  on disk but in the wrong state (a slotted mod whose files are ghosted does
  nothing; an unslotted one whose files are active runs anyway)
- **Check for leftover mod folders** — lists CET folders that hold no mod any
  more, with what each contains and how big it is, and lets you pick. Folders
  holding settings start unchecked, and a folder containing files that belong
  to an *installed* mod is never listed — mods do ship presets for one another
- **Remove duplicate records** and **Clean temporary files** — database
  duplicates and leftover extraction directories in `/tmp`

Deleting a mod through the app now clears the folders it empties, so this is
mostly about tidying what earlier versions left.


<a id="wrapper-folders"></a>
## Wrapper folders (versions up to 1.2)

*Mods installed one folder too deep.*

It affects mods that were installed successfully, show up as enabled, and
still do nothing in the game.

**What went wrong.** Mods are usually packaged with the game's own folder
layout inside the archive (`archive/`, `bin/`, `r6/`…), and those get merged
into the game directory. But some mods ship everything inside one extra folder
named after the mod:

```
the archive:            installed as (wrong):        should have been:
ModName/                {game}/ModName/r6/…          {game}/r6/…
  r6/tweaks/…           {game}/ModName/bin/…         {game}/bin/…
  bin/x64/…
```

Versions up to 1.2 copied that layout verbatim, wrapper and all. Nothing looks
wrong from the app's side — the files really were copied, the mod is recorded,
the toggle says enabled — but no mod loader looks inside `{game}/ModName/`, so
the mod never loads. There is no error anywhere: CET and TweakXL don't report
folders they were never told about.

**Whether it hit you** depends on what the mod is made of. The installer had
fallback rules for `.archive`, `.reds`, `.dll` and `.exe` files, which quietly
rescued those. Everything else — the `.lua` and `.json` of CET mods, the
`.yaml` of TweakXL tweaks, loose textures — fell through and stayed in the
wrapper. In a real 262-mod library, 2 mods were affected.

**How to check and fix it**, in v1.3 or later:

1. The startup banner tells you if anything is affected: *"N mods installed
   inside a redundant folder and can't be loaded by the game"*.
2. **Config → Check for unloadable mods** lists exactly what would move,
   without changing anything yet.
3. **Repair now** backs up the mod database, moves each file to where the
   loaders actually look, updates the database, and clears the emptied folders.
4. Restart the game.

Repair is deliberately cautious and skips anything ambiguous: multi-variant and
FOMOD archives (moving their parts would enable options you never chose), and
mods whose top-level folder holds no recognisable game directory. If a file's
destination is already taken by another mod, it is skipped rather than
overwritten. Reinstalling a mod also fixes it, since new installs strip the
wrapper correctly.

A mod can also land *half* in a wrapper: some files where they belong, the rest
under a folder no loader reads. The scan lists these separately, because that
shape on disk is indistinguishable from a mod whose base files installed
correctly next to an optional variant you never selected. **Repair now** leaves
them alone; each gets its own button, since moving the files activates whatever
they contain and only you know which it is. Check the mod's page first.

