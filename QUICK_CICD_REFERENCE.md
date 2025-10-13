# Quick CI/CD Reference

## Quick Commands

### Create a New BETA Release

```bash
# Easy way (recommended) - Creates a beta release
./scripts/release.sh 0.1.0 1

# This creates tag: v0.1.0-beta1
# For subsequent betas: ./scripts/release.sh 0.1.0 2

# Manual way
# 1. Update version in src-tauri/tauri.conf.json
# 2. Update CHANGELOG.md
# 3. Commit and tag
git add .
git commit -m "chore: BETA Release v0.1.0-beta1"
git tag v0.1.0-beta1
git push origin main --tags
```

### Check Build Status

```bash
# Check if code compiles
cargo check

# Run tests
cargo test

# Check formatting
cargo fmt -- --check

# Run linter
cargo clippy -- -D warnings

# Build locally
npm run tauri build
```

### Trigger Manual Release

1. Go to: https://github.com/beneccles/crossover-mod-manager/actions
2. Select "Release" workflow
3. Click "Run workflow"
4. Select branch and click "Run workflow"

## GitHub Actions URLs

- **All Workflows**: https://github.com/beneccles/crossover-mod-manager/actions
- **Build Workflow**: https://github.com/beneccles/crossover-mod-manager/actions/workflows/build.yml
- **Release Workflow**: https://github.com/beneccles/crossover-mod-manager/actions/workflows/release.yml
- **Releases Page**: https://github.com/beneccles/crossover-mod-manager/releases

## Version Numbering

Use semantic versioning with BETA suffix: `MAJOR.MINOR.PATCH-betaN`

- **MAJOR**: Breaking changes (e.g., 0.1.0 → 1.0.0)
- **MINOR**: New features, backward compatible (e.g., 0.1.0 → 0.2.0)
- **PATCH**: Bug fixes, backward compatible (e.g., 0.1.0 → 0.1.1)
- **BETA**: Pre-release for testing (e.g., 0.1.0-beta1, 0.1.0-beta2)

Current policy: All releases are BETA until further notice

## CHANGELOG Format

```markdown
## [1.7.0] - 2025-10-13

### Added

- New feature that adds X
- Support for Y

### Changed

- Improved performance of Z
- Updated dependency A to version B

### Fixed

- Fixed bug where X would crash
- Resolved issue with Y on macOS

### Security

- Fixed security vulnerability in Z
```

## Troubleshooting

### Build Failing on CI but Works Locally

1. Check the exact error in Actions logs
2. Ensure all dependencies are in package.json/Cargo.toml
3. Try building in a clean environment:
   ```bash
   cargo clean
   npm ci
   npm run tauri build
   ```

### Tag Already Exists

```bash
# Delete local tag (example with beta tag)
git tag -d v0.1.0-beta1

# Delete remote tag
git push origin :refs/tags/v0.1.0-beta1

# Recreate tag
git tag v0.1.0-beta1
git push origin v0.1.0-beta1
```

### Release Artifacts Missing

1. Check if all build jobs completed successfully
2. Look for errors in the "Upload Release Asset" steps
3. Verify artifact paths are correct
4. Re-run failed jobs if needed

## File Locations

- Build workflow: `.github/workflows/build.yml`
- Release workflow: `.github/workflows/release.yml`
- Release script: `scripts/release.sh`
- Version file: `src-tauri/tauri.conf.json`
- Changelog: `CHANGELOG.md`

## Platform-Specific Notes

### macOS Apple Silicon Only

- Builds on macOS-14 (native Apple Silicon runner)
- Produces DMG file for M1/M2/M3/M4 Macs
- Uses ad-hoc signing (for distribution, need proper certificate)
- Targets aarch64-apple-darwin

## Security

- All secrets managed through GitHub Secrets
- No API keys hardcoded in workflows
- GITHUB_TOKEN automatically provided by GitHub

## Monitoring

- Check Actions tab regularly for build status
- Watch for Dependabot alerts
- Review security audit results

## Getting Help

- Full documentation: [CI_CD.md](CI_CD.md)
- Development guide: [DEVELOPMENT.md](DEVELOPMENT.md)
- Open an issue: https://github.com/beneccles/crossover-mod-manager/issues/new
