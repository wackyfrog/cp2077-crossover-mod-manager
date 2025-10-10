# RED4ext Compatibility Guide

## What is RED4ext?

RED4ext is a native code extension framework for Cyberpunk 2077 that allows advanced mods to hook directly into the game's engine. It's a powerful but complex modding framework.

## ⚠️ Critical Compatibility Warning for macOS/Crossover Users

**RED4ext has limited compatibility when running Cyberpunk 2077 through Crossover/Wine on macOS.**

### Why RED4ext Often Fails on macOS

1. **Native Code Injection**: RED4ext uses advanced Windows-specific techniques to inject code into the game process
2. **Wine Limitations**: Wine's translation layer doesn't perfectly emulate all Windows API calls that RED4ext requires
3. **DLL Loading Issues**: Complex dependency chains and loading order problems in Wine environments
4. **Memory Management**: Native code memory allocation patterns that don't translate well through Wine

### Common Error Messages

If you see errors like:

- "RED4ext could not be loaded"
- "Module not found"
- "Failed to initialize RED4ext"
- Game crashes on startup after installing RED4ext

These are typical signs that RED4ext is incompatible with your Wine/Crossover setup.

## Recommended Alternatives

### For Most Users (macOS + Crossover)

1. **Redscript Mods**: Use `.reds` script-based mods instead
2. **Cyber Engine Tweaks (CET)**: Better Wine compatibility for scripting
3. **Archive Mods**: Pure asset replacement mods (`.archive` files)
4. **REDmod**: Official CDPR modding system

### For Windows Users

RED4ext generally works well on native Windows installations with:

- Visual C++ Redistributable 2019 or newer
- Proper antivirus exclusions
- Administrator privileges for first-time setup

## Installation Notes

Our mod manager will:

- ✅ **Detect RED4ext** mods during installation
- ⚠️ **Warn about compatibility** issues on macOS
- 📁 **Install files correctly** to the proper locations:
  - `RED4ext.dll` → `bin/x64/RED4ext.dll`
  - Plugin DLLs → `red4ext/plugins/`
  - Configuration files → `red4ext/config/`

## Troubleshooting Steps

### If RED4ext Fails to Load

1. **Check Visual C++ Redistributables** (Windows):

   ```
   Download from Microsoft: VC++ Redistributable 2019 x64
   ```

2. **Verify File Locations**:

   ```
   Game Directory/
   ├── bin/x64/RED4ext.dll          ← Core RED4ext
   ├── red4ext/
   │   ├── plugins/                 ← Plugin DLLs go here
   │   └── config/                  ← Config files
   ```

3. **Check Game Version Compatibility**:

   - Ensure RED4ext version matches your Cyberpunk 2077 version
   - Check mod description for supported game versions

4. **Run as Administrator** (Windows):
   - Some RED4ext features require elevated privileges

### For Crossover/Wine Users

1. **Try CET-based alternatives** first
2. **Install VC++ Redistributables** in the bottle:
   ```
   CrossOver → Bottle → Install Software →
   Microsoft Visual C++ 2019 Redistributable
   ```
3. **Check Wine version**: Newer Wine versions may have better compatibility
4. **Consider dual-boot** for extensive RED4ext modding

## Alternative Modding Options

### Redscript (.reds files)

- ✅ Excellent Wine compatibility
- ✅ Active development community
- ✅ Many powerful mods available
- 📁 Install to: `r6/scripts/`

### Cyber Engine Tweaks

- ✅ Good Wine compatibility with configuration
- ✅ Lua scripting support
- ✅ In-game console
- 🔧 Requires Wine library configuration

### Archive Mods

- ✅ Perfect compatibility
- ✅ Asset replacement and additions
- ✅ No runtime dependencies
- 📁 Install to: `archive/pc/mod/`

## Getting Help

If you're having RED4ext issues:

1. **Check Mod Compatibility**: Look for Redscript or CET versions
2. **Community Forums**: Visit r/cyberpunkgame or Nexus Mods discussions
3. **Mod Manager Logs**: Check installation logs for specific errors
4. **Wine AppDB**: Check Wine Application Database for latest compatibility reports

## Summary

- 🖥️ **Windows**: RED4ext generally works with proper setup
- 🍎 **macOS/Crossover**: Limited compatibility, use alternatives when possible
- 🔧 **Always check**: Mod descriptions for alternative versions
- 📋 **Best practice**: Try Redscript or CET-based mods first on macOS

---

_Last updated: October 9, 2025_
