# CI/CD Pipeline Documentation

This document describes the Continuous Integration and Continuous Deployment (CI/CD) setup for the Crossover Mod Manager project.

## Overview

The project uses GitHub Actions for automated building, testing, and releasing. There are two main workflows:

1. **Build and Test** (`build.yml`) - Runs on every push and pull request
2. **Release** (`release.yml`) - Creates releases and builds distribution packages

## Build and Test Workflow

**Trigger**: Every push to `main` branch and all pull requests

**Jobs**:

### 1. Build Frontend

- Runs on Ubuntu
- Installs Node.js dependencies
- Builds the React frontend with Vite
- Uploads the `dist/` folder as an artifact

### 2. Check Rust Code

- Runs on Ubuntu
- Sets up Rust toolchain
- Installs Linux system dependencies (webkit2gtk, GTK, etc.)
- Runs `cargo check` to verify code compiles
- Runs `cargo test` to execute unit tests
- Uses caching for faster builds:
  - Cargo registry
  - Cargo git index
  - Cargo build artifacts

### 3. Lint

- Runs on Ubuntu
- Checks Rust code formatting with `cargo fmt --check`
- Runs Clippy with strict mode (`-D warnings`)
- Ensures code follows Rust best practices

### 4. Security Audit

- Runs on Ubuntu
- Uses `cargo-audit` to check for known security vulnerabilities
- Runs as a warning (doesn't fail the build)

## Release Workflow

**Trigger**:

- Push of BETA version tags (e.g., `v1.6.0-beta1`, `v1.7.0-beta2`)
- Manual workflow dispatch

**Note**: All releases are currently marked as BETA/pre-release

**Jobs**:

### 1. Create Release

- Extracts version from tag or `tauri.conf.json`
- Reads changelog from `CHANGELOG.md`
- Creates a GitHub Release with:
  - Release notes from changelog
  - Download links
  - Installation instructions

### 2. Build macOS Apple Silicon

Builds for Apple Silicon (M1/M2/M3/M4) Macs:

- Runs on `macos-14` (native Apple Silicon runner)
- Targets `aarch64-apple-darwin`

**Steps**:

1. Setup Node.js and Rust
2. Install npm dependencies
3. Build frontend
4. Build Tauri app for Apple Silicon
5. Rename DMG to standardized format:
   - `Crossover.Mod.Manager_{version}_aarch64.dmg`
6. Upload to GitHub Release
7. Upload as workflow artifact

### 3. Post-Release Notifications

- Checks status of the build job
- Reports success or failure
- Can be extended to send notifications (Slack, Discord, etc.)

## Creating a Release

### Automatic Release (Recommended)

1. **Update version** in `src-tauri/tauri.conf.json`:

   ```json
   {
     "version": "0.2.0"
   }
   ```

2. **Update CHANGELOG.md** with the new version:

   ```markdown
   ## [0.2.0] - 2025-10-13

   ### Added

   - New feature description

   ### Fixed

   - Bug fix description
   ```

3. **Commit changes**:

   ```bash
   git add src-tauri/tauri.conf.json CHANGELOG.md
   git commit -m "chore: BETA Release v0.2.0-beta1"
   git push origin main
   ```

4. **Create and push BETA tag**:

   ```bash
   git tag v0.2.0-beta1
   git push origin v0.2.0-beta1
   ```

   For subsequent betas, increment the number: `v0.2.0-beta2`, `v0.2.0-beta3`, etc.5. **Monitor the release**:

   - Go to Actions tab on GitHub
   - Watch the Release workflow progress
   - Once complete, check the Releases page

### Manual Release

You can also trigger a release manually from the GitHub Actions tab:

1. Go to Actions → Release workflow
2. Click "Run workflow"
3. Select the branch
4. Click "Run workflow"

## Release Assets

Each release includes:

### macOS Apple Silicon

- `Crossover.Mod.Manager_{version}_aarch64.dmg` - Apple Silicon (M1/M2/M3/M4)

## System Requirements

### Build Requirements

**macOS Builds**:

- macOS 14+ (Apple Silicon runner)
- Xcode Command Line Tools
- Node.js 20+
- Rust stable toolchain with aarch64-apple-darwin target

### Runtime Requirements

**macOS**:

- macOS 11.0+ (Big Sur or later)
- Apple Silicon Mac (M1/M2/M3/M4)
- CrossOver 23+ recommended

## Caching Strategy

The workflows use aggressive caching to speed up builds:

1. **npm cache**: Caches node_modules based on package-lock.json
2. **Cargo registry**: Caches downloaded crates
3. **Cargo git**: Caches git dependencies
4. **Cargo build**: Caches compiled dependencies

Typical build times:

- Cold build: 5-8 minutes
- Cached build: 2-3 minutes

## Troubleshooting

### Build Failures

**"No space left on device"**:

- GitHub runners have limited disk space
- Clean up artifacts: `cargo clean` before build
- Consider splitting into multiple jobs

**"Could not find libwebkit2gtk-4.1"**:

- Update system dependencies in workflow
- Ensure using Ubuntu 22.04+

**DMG signing failures on macOS**:

- Currently using ad-hoc signing (`-`)
- For distribution, set up proper signing:
  - Add Apple Developer certificates to secrets
  - Configure signing identity in workflow

### Failed Releases

If a release job fails:

1. **Check the Actions logs** for specific error
2. **Re-run failed jobs** from Actions tab
3. **Delete and recreate tag** if needed:
   ```bash
   git tag -d v1.7.0
   git push origin :refs/tags/v1.7.0
   git tag v1.7.0
   git push origin v1.7.0
   ```

## Security Considerations

### Secrets

The workflows use these GitHub secrets:

- `GITHUB_TOKEN` - Automatically provided, used for releases

### Future Enhancements

Consider adding:

- **Code signing** for macOS (requires Apple Developer account)
- **Notarization** for macOS (requires Apple Developer account)
- **Auto-update** mechanism in the app
- **Delta updates** for smaller downloads
- **Update server** integration

## Monitoring

### Build Status Badge

Add to README.md:

```markdown
[![Build Status](https://github.com/beneccles/crossover-mod-manager/workflows/Build%20and%20Test/badge.svg)](https://github.com/beneccles/crossover-mod-manager/actions)
```

### Release Notifications

Future improvements:

- Discord webhook for release notifications
- Automatic changelog generation from commits
- Version bump automation

## Best Practices

1. **Always update CHANGELOG.md** before releasing
2. **Test locally** with `npm run tauri build` before pushing tags
3. **Use semantic versioning**: MAJOR.MINOR.PATCH
4. **Document breaking changes** in release notes
5. **Keep dependencies updated** (Dependabot)

## Local Testing

Test the release build locally before pushing:

```bash
# Build frontend
npm run build

# Build Tauri app
npm run tauri build

# Test the built app
open "src-tauri/target/release/bundle/macos/Crossover Mod Manager.app"
```

## Continuous Improvement

Planned enhancements:

- [ ] Add Windows build support
- [ ] Implement auto-update functionality
- [ ] Add performance benchmarks
- [ ] Set up code coverage reporting
- [ ] Add integration tests
- [ ] Implement A/B testing for UI changes
- [ ] Add telemetry (opt-in) for crash reporting

## Support

For CI/CD issues:

1. Check [GitHub Actions documentation](https://docs.github.com/en/actions)
2. Check [Tauri v2 CI/CD guide](https://v2.tauri.app/distribute/ci-cd/)
3. Open an issue with the `ci/cd` label
