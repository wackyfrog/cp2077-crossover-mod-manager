# Development Guide

This guide provides information for developers working on the Crossover Mod Manager.

## Development Setup

### Prerequisites

1. **Node.js and npm**

   - Node.js 20 or higher (CI builds on Node 20)
   - npm (bundled with Node.js)
   - Check versions: `node --version && npm --version`

2. **Rust**

   - Rust 1.70 or higher
   - Install via [rustup](https://rustup.rs/)
   - Check version: `rustc --version`

3. **Tauri Dependencies**

   **macOS:**

   ```bash
   xcode-select --install
   ```

   **Linux (Ubuntu/Debian):**

   ```bash
   sudo apt update
   sudo apt install libwebkit2gtk-4.1-dev \
     build-essential \
     curl \
     wget \
     file \
     libssl-dev \
     libgtk-3-dev \
     libayatana-appindicator3-dev \
     librsvg2-dev
   ```

   **Note:** Ubuntu 22.04+ and Debian 12+ use `libwebkit2gtk-4.1-dev`. For older versions, use `libwebkit2gtk-4.0-dev`.

### Initial Setup

1. Clone the repository:

   ```bash
   git clone https://github.com/wackyfrog/cp2077-crossover-mod-manager.git
   cd cp2077-crossover-mod-manager
   ```

2. Install dependencies:
   ```bash
   npm install
   ```

## Development Workflow

### Running the Development Server

Start both the Vite dev server and Tauri app:

```bash
npm run tauri:dev
```

This will:

- Start Vite dev server on `http://localhost:1430`
- Launch the Tauri application
- Enable hot-reload for frontend changes
- Rebuild Rust code on changes

### Frontend-Only Development

If you want to work on the frontend without Tauri:

```bash
npm run dev
```

Note: Tauri API calls will fail in this mode.

### Building for Production

Build the complete application:

```bash
npm run tauri:build
```

Build outputs (this is a macOS-only app):

- **App bundle**: `src-tauri/target/release/bundle/macos/Crossover Mod Manager.app`
- **DMG installer**: `src-tauri/target/release/bundle/dmg/`

## Project Structure

A high-level map — browse `src/` and `src-tauri/src/` for the current set of files
rather than relying on a fixed listing here.

- **`src/`** — React frontend (Vite). Entry `main.jsx` → `App.jsx`; UI components
  live in `src/components/` (mod list, mod details, config/settings, and the themed
  overlays for install / sync / logs).
- **`src-tauri/src/`** — Rust backend modules:
  - `main.rs` — Tauri app entry and `#[tauri::command]` handlers
  - `mod_manager.rs` — mod install / update / remove logic
  - `archive_extractor.rs` — archive extraction (zip; optional 7z/rar via p7zip/unrar)
  - `nexusmods_api.rs` — NexusMods API client
  - `settings.rs` — settings persistence
- **`src-tauri/`** also holds `tauri.conf.json` (Tauri config), `Cargo.toml` (Rust
  deps), and `build.rs` (build script).
- **`dist/`** — Vite build output (generated); **`node_modules/`** — npm deps (generated).
- Root: `package.json` (npm scripts and the app version — `tauri.conf.json` reads
  `"version": "../package.json"`), `vite.config.js`, `index.html`.

## Key Technologies

### Frontend Stack

- **React 19**: UI library
- **Vite 7**: Build tool and dev server
- **CSS**: Styling (no preprocessor)
- **@tauri-apps/api**: Tauri JavaScript API

### Backend Stack

- **Tauri 2**: Desktop app framework (`tauri` 2.10)
- **Rust**: Backend language
- **serde/serde_json**: JSON serialization
- **reqwest**: HTTP client for downloads
- **zip**: Archive extraction
- **walkdir**: File system traversal

## Architecture

### Communication Flow

```
React UI ──invoke()──> Tauri Commands ──> Rust Backend
                                            ├── ModManager
                                            ├── Settings
                                            └── File System
```

### Tauri Commands

The frontend talks to the backend through Tauri commands registered in
`src-tauri/src/main.rs` via `tauri::generate_handler![...]`. They cover the mod
lifecycle (install / update / reinstall / enable / disable / remove), NexusMods
sync, settings, game-path detection, and diagnostics/logs.

For the current, authoritative list, read the `generate_handler!` macro in
`src-tauri/src/main.rs` — each entry is a `#[tauri::command]` function.

### Data Storage

All data is stored in the user's home directory:

```
~/.crossover-mod-manager/
├── mods.json       # Installed mods database
└── settings.json   # Application settings
```

## Adding Features

### Adding a New Frontend Component

1. Create component file: `src/components/NewComponent.jsx`
2. Create styles: `src/components/NewComponent.css`
3. Import in `App.jsx`: `import NewComponent from './components/NewComponent'`
4. Use in render: `<NewComponent />`

### Adding a New Tauri Command

1. Define function in Rust (`src-tauri/src/main.rs` or separate module):

   ```rust
   #[tauri::command]
   fn my_command(param: String) -> Result<String, String> {
       Ok(format!("Processed: {}", param))
   }
   ```

2. Register in `main()`:

   ```rust
   .invoke_handler(tauri::generate_handler![
       // ... existing commands
       my_command
   ])
   ```

3. Call from frontend:

   ```javascript
   import { invoke } from "@tauri-apps/api/core";

   const result = await invoke("my_command", { param: "value" });
   ```

### Modifying Mod Installation Logic

Mod installation lives in `src-tauri/src/mod_manager.rs` (the install / update /
remove flow and install-path resolution), with archive extraction split out into
`src-tauri/src/archive_extractor.rs`. Read those modules for the current function
set rather than relying on a fixed list here.

## Testing

### Manual Testing

1. Start dev server: `npm run tauri:dev`
2. Test mod list display
3. Test settings save/load
4. Test mod installation (requires mock data)
5. Test mod removal

### Testing Mod Installation

Create a test mod data object:

```javascript
const testMod = {
  name: "Test Mod",
  version: "1.0.0",
  author: "Test Author",
  description: "A test mod",
  download_url: "https://example.com/mod.zip",
  mod_id: "123",
  file_id: "456",
};

await invoke("install_mod", { modData: testMod });
```

## Debugging

### Frontend Debugging

- Open DevTools in Tauri window: Right-click → Inspect Element
- Console logs appear in DevTools
- React DevTools extension works

### Backend Debugging

- Add print statements: `println!("Debug: {}", value);`
- Run with console output:
  ```bash
  RUST_LOG=debug npm run tauri:dev
  ```
- Logs appear in terminal

### Common Issues

1. **"Failed to resolve import"**: Check npm dependencies
2. **"Build failed" (Rust)**: Check Cargo.toml versions
3. **"Permission denied"**: Check file system permissions
4. **"Protocol handler not working"**: Re-register the app

## Code Style

### JavaScript/React

- Use functional components with hooks
- Use arrow functions for callbacks
- Use async/await for promises
- Keep components focused and small
- Use CSS modules for component styles

### Rust

- Follow standard Rust formatting (`cargo fmt`)
- Use `Result<T, String>` for errors
- Handle errors with `map_err()`
- Keep functions focused
- Add comments for complex logic

## Contributing

1. Create a feature branch
2. Make changes
3. Test thoroughly
4. Submit pull request
5. Ensure CI passes

## Resources

- [Tauri Documentation](https://tauri.app/)
- [React Documentation](https://react.dev/)
- [Vite Documentation](https://vitejs.dev/)
- [Rust Documentation](https://doc.rust-lang.org/)
- [NexusMods API](https://app.swaggerhub.com/apis-docs/NexusMods/nexus-mods_public_api_params_in_form_data/1.0)
