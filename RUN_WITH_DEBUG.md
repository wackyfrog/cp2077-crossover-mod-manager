# Running with Debug Output

To see the debug logging for archive type detection, run the app from the terminal:

## Option 1: Run from Terminal

```bash
# Run the app and see debug output
/Applications/Crossover\ Mod\ Manager.app/Contents/MacOS/crossover-mod-manager
```

Then when you download a mod, you'll see output like:

```
DEBUG: Detecting archive type for: "/var/folders/.../temp_archive_12345.zip"
DEBUG: File extension: zip
DEBUG: Magic bytes (8 bytes): 50 4B 03 04 14 00 00 00
DEBUG: Detected as ZIP by magic bytes
```

Or if it's actually a 7z file:

```
DEBUG: Detecting archive type for: "/var/folders/.../temp_archive_12345.zip"
DEBUG: File extension: zip
DEBUG: Magic bytes (8 bytes): 37 7A BC AF 27 1C 00 05
DEBUG: Detected as 7z by magic bytes
```

## Option 2: Check Console.app

1. Open **Console.app** (in /Applications/Utilities/)
2. Search for "crossover-mod-manager"
3. Watch for DEBUG lines while installing a mod

## What to Look For

The debug output will tell us:

- What file path is being checked
- What extension the file has
- What the actual magic bytes are
- Which detection method succeeded (magic bytes or extension)

## Expected Output for Your Test Mod

For the HD Reworked Project mod that's mislabeled:

**If it's truly a 7z file:**

```
DEBUG: File extension: zip
DEBUG: Magic bytes (8 bytes): 37 7A BC AF 27 1C ...
DEBUG: Detected as 7z by magic bytes
```

**If it's actually a ZIP file:**

```
DEBUG: File extension: zip
DEBUG: Magic bytes (8 bytes): 50 4B 03 04 ...
DEBUG: Detected as ZIP by magic bytes
```

This will tell us definitively what format the file actually is!
