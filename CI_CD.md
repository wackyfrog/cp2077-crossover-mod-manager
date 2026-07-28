# CI/CD Pipeline Documentation

This document describes the Continuous Integration and Continuous Deployment (CI/CD) setup for the Crossover Mod Manager project.

## Overview

The project uses GitHub Actions. There are two workflows:

1. **Build and Test** (`build.yml`) — runs on every push and pull request to `main`.
2. **Release** (`release.yml`) — builds and publishes **BETA** releases automatically when a `v*-beta*` tag is pushed. Stable releases are published manually by the maintainer (see [Creating a Release](#creating-a-release)).

## Build and Test Workflow

**Trigger**: every push to `main` and every pull request targeting `main`.

The jobs run in this order (`check-rust` and `lint` both depend on `build-frontend`):

### 1. Build Frontend

- Runs on Ubuntu (Node.js 20)
- `npm ci`, then `npm run build` (Vite)
- Uploads the `dist/` folder as the `frontend-dist` artifact

### 2. Check Rust Code

- Runs on Ubuntu, downloads the frontend artifact
- Installs Linux system dependencies (webkit2gtk, GTK, glib, etc.) — needed only to compile the Tauri crate on the Linux CI runner; the shipped app is macOS-only
- Runs `cargo check` and `cargo test --no-fail-fast` (in `src-tauri/`)
- Caches the cargo registry, git index, and build target for faster runs

### 3. Lint

- Runs on Ubuntu, downloads the frontend artifact
- `cargo fmt -- --check` (formatting)
- `cargo clippy -- -D warnings` (warnings fail the job)

### 4. Security Audit

- Runs on Ubuntu
- Installs and runs `cargo audit`
- Non-blocking (`continue-on-error`) — reports known vulnerabilities as a warning

## Release Workflow

**Trigger**: push of a BETA tag matching `v*-beta*` (e.g. `v1.2.0-beta1`). A manual `workflow_dispatch` is also exposed. A stable `vX.Y.Z` tag (no `-beta` suffix) does **not** match this trigger and does not start CI — stable releases are built and published manually.

**Jobs**:

### 1. Create Release

- Extracts the version from the pushed tag
- Reads the matching version section from `CHANGELOG.md`
- Determines the channel: a `-beta` tag is published as a GitHub **pre-release** titled "🧪 BETA …"; any other tag would be a full release
- Creates the GitHub Release with release notes, a download list, and install instructions

### 2. Build macOS Apple Silicon

Builds for Apple Silicon (M1/M2/M3/M4) Macs:

- Runs on `macos-14` (native Apple Silicon runner)
- Rust target `aarch64-apple-darwin`
- `npm ci` → `npm run build` → `npm run tauri build -- --target aarch64-apple-darwin`
- Renames the DMG to the standardized asset name:
  - `Crossover.Mod.Manager_{version}_aarch64.dmg`
- Uploads it to the GitHub Release and as a workflow artifact

### 3. Post-Release Notifications

- Checks the build job result and reports success or failure
- Placeholder that can be extended to send notifications (Slack, Discord, etc.)

## Creating a Release

The app version lives in `package.json`; `src-tauri/tauri.conf.json` reads it via `"version": "../package.json"`. **Do not** hand-edit the version in `tauri.conf.json` — that would break the pointer. Bump `package.json` instead (the release helper does this for you).

**`src-tauri/Cargo.toml` carries its own version and must be bumped to match.** It is not derived from `package.json`, and the on-disk log stamps every session with `env!("CARGO_PKG_VERSION")` — which reads Cargo.toml. Leave it behind and the build reports the *previous* version in `~/.crossover-mod-manager/logs/app.log`, the one file bug reports are diagnosed from, while the UI (fed from `package.json` via `__APP_VERSION__`) shows the new one. This bit v1.5.1, which shipped stamping its log `v1.5.0`. `release.sh` now bumps both; on the manual path, bump both by hand.

### BETA release (automated via CI)

BETA builds are published automatically by the Release workflow.

1. Make sure `main` is clean and up to date, and that `CHANGELOG.md` has a `## [X.Y.Z]` section for the version.
2. Run the release helper with a version and a beta number:

   ```bash
   ./scripts/release.sh 1.2.0 1
   ```

   This bumps the version in `package.json`, commits `package.json` + `CHANGELOG.md`, creates the annotated tag `v1.2.0-beta1`, and pushes both `main` and the tag. For subsequent betas, increment the number: `./scripts/release.sh 1.2.0 2`.

3. Pushing the `v*-beta*` tag triggers `release.yml`, which builds the Apple Silicon DMG and publishes a GitHub **pre-release** titled "🧪 BETA …".
4. Monitor progress on the **Actions** tab; once complete, the pre-release appears on the **Releases** page.

### Stable release (manual)

Stable versions (`vX.Y.Z`, no `-beta` suffix) are **not** built by CI — the maintainer builds and publishes them locally.

1. Bump the version in **both** `package.json` and `src-tauri/Cargo.toml` (see the note above — the log's version comes from Cargo.toml), refresh `Cargo.lock`, and add the `## [X.Y.Z]` section to `CHANGELOG.md`; commit them to `main`.
2. Build the app locally (the frontend is built automatically via `beforeBuildCommand`):

   ```bash
   npm run tauri build
   ```

   The DMG lands in `src-tauri/target/release/bundle/dmg/`. It is ad-hoc signed (`signingIdentity: "-"`) and not notarized — that is expected; on first launch users approve it via System Settings → Privacy & Security → **Open Anyway** (see [First launch on macOS](README.md#first-launch-on-macos) in the README).

3. Create the annotated tag and push it:

   ```bash
   git tag -a vX.Y.Z -m "vX.Y.Z — <short description>"
   git push origin vX.Y.Z
   ```

4. Create the GitHub Release with the local DMG, renamed to the standard asset name so it matches the beta convention:

   ```bash
   gh release create vX.Y.Z \
     --title "vX.Y.Z" \
     --notes "<release notes>" \
     "<path-to-dmg>#Crossover.Mod.Manager_X.Y.Z_aarch64.dmg"
   ```

5. Verify the release is not a draft and not a pre-release, and that the `.dmg` asset is attached with the expected size.

## Release Assets

Each release includes:

### macOS Apple Silicon

- `Crossover.Mod.Manager_{version}_aarch64.dmg` — Apple Silicon (M1/M2/M3/M4)

## System Requirements

### Build Requirements

**macOS builds**:

- macOS 14 for the CI runner (local builds work on any recent macOS with Apple Silicon)
- Xcode Command Line Tools
- Node.js 20+
- Rust stable toolchain with the `aarch64-apple-darwin` target

### Runtime Requirements

**macOS**:

- macOS 11.0+ (Big Sur or later)
- Apple Silicon Mac (M1/M2/M3/M4)
- CrossOver 25+

## Caching Strategy

The workflows cache dependencies to speed up builds:

1. **npm cache**: handled by `actions/setup-node` (keyed on `package-lock.json`)
2. **Cargo registry**: caches downloaded crates
3. **Cargo git**: caches git dependencies
4. **Cargo build**: caches the `src-tauri/target` directory

Typical build times (approximate):

- Cold build: 5–8 minutes
- Cached build: 2–3 minutes

## Troubleshooting

### Build Failures

**"No space left on device"**:

- GitHub runners have limited disk space
- Clean up artifacts (`cargo clean`) before building
- Consider splitting into multiple jobs

**DMG signing failures on macOS**:

- The app currently uses ad-hoc signing (`signingIdentity: "-"`)
- For distributable signing, add Apple Developer certificates to secrets and configure a signing identity (requires an Apple Developer account)

### Failed Releases

If a BETA release job fails:

1. **Check the Actions logs** for the specific error
2. **Re-run failed jobs** from the Actions tab
3. **Delete and recreate the tag** if needed:
   ```bash
   git tag -d vX.Y.Z-beta1
   git push origin :refs/tags/vX.Y.Z-beta1
   git tag vX.Y.Z-beta1
   git push origin vX.Y.Z-beta1
   ```

## Security Considerations

### Secrets

The workflows use these GitHub secrets:

- `GITHUB_TOKEN` — automatically provided by GitHub, used to create releases and upload assets

### Signing & Notarization

The macOS DMG is currently ad-hoc signed and not notarized, so users must approve it past Gatekeeper on first launch — System Settings → Privacy & Security → **Open Anyway**, or `xattr -dr com.apple.quarantine <app>`. Control-click → Open is **not** a route on macOS 15 Sequoia and later: Apple removed that bypass ([Updates to runtime protection in macOS Sequoia](https://developer.apple.com/news/?id=saqachfa)). Proper code signing and notarization would require an Apple Developer Program membership ($99/year); this is tracked as future work.

## Monitoring

### Build Status Badge

Add to README.md:

```markdown
[![Build Status](https://github.com/wackyfrog/cp2077-crossover-mod-manager/workflows/Build%20and%20Test/badge.svg)](https://github.com/wackyfrog/cp2077-crossover-mod-manager/actions)
```

## Best Practices

1. **Always update CHANGELOG.md** before releasing
2. **Test locally** with `npm run tauri build` before pushing tags
3. **Use semantic versioning**: `MAJOR.MINOR.PATCH`
4. **Document breaking changes** in release notes
5. **Keep dependencies updated**

## Local Testing

Test the release build locally before pushing:

```bash
# Build frontend
npm run build

# Build the Tauri app
npm run tauri build

# Open the built app
open "src-tauri/target/release/bundle/macos/Crossover Mod Manager.app"
```

## Support

For CI/CD issues:

1. Check the [GitHub Actions documentation](https://docs.github.com/en/actions)
2. Check the [Tauri v2 CI/CD guide](https://v2.tauri.app/distribute/ci-cd/)
3. Open an issue with the `ci/cd` label
