# Release Process

This document describes how to cut a new release of DeepTank.

## Pre-Release Checklist

Before releasing, ensure:

- [ ] All tests pass locally: `npm test`
- [ ] Frontend builds: `npm run build`
- [ ] Desktop app builds locally: `npm run tauri build`
- [ ] No TODOs, FIXMEs, or console.error calls in code
- [ ] Git history is clean (no dirty working directory)
- [ ] Decide on version number:
  - `patch` (0.0.X) — Bug fixes only
  - `minor` (0.X.0) — New features, backwards compatible
  - `major` (X.0.0) — Breaking changes

## Release Steps

### 1. Bump Version

```bash
npm version patch  # or minor/major
```

This command:
1. Updates version in `package.json`
2. Runs `.github/scripts/bump-version.sh` (updates `Cargo.toml`)
3. Creates a git commit with message "1.0.0" (or whatever version)
4. Tags commit with `v1.0.0`

### 2. Generate Changelog

```bash
.github/scripts/generate-changelog.sh
```

This command:
1. Extracts commits since last tag
2. Groups by type (feat, fix, refactor, docs)
3. Generates markdown sections
4. Prepends to `CHANGELOG.md`

### 3. Review and Commit Changes

```bash
git diff CHANGELOG.md              # Review changes
git add CHANGELOG.md
git commit -m "docs: update CHANGELOG for v1.0.0"
```

> **Note:** `npm version` already created a commit, so this is an additional commit.

### 4. Push to Remote

```bash
git push origin main
git push origin v1.0.0  # Push the tag
```

Pushing the tag `v1.0.0` triggers `.github/workflows/release.yml` which:
- Builds on Linux, macOS (x64 + arm64), and Windows
- Creates artifacts (.AppImage, .dmg, .msi)
- Uploads to GitHub Releases page
- Links to CHANGELOG.md

This typically takes 15-30 minutes depending on GitHub's CI queue.

### 5. Verify Release

1. Go to https://github.com/saagar210/DeepTank/releases
2. Verify release appears (may take 1-2 minutes)
3. Verify all binaries are uploaded:
   - `deeptank-linux-x64.AppImage`
   - `deeptank-macos-x64.dmg`
   - `deeptank-macos-arm64.dmg`
   - `deeptank-windows-x64.msi`
4. Download one binary and test it runs locally (sanity check)
5. Publish release (uncheck draft if needed)

## Post-Release

### Update Documentation
```bash
# Update README with latest version
# Example:
#   Latest Release: [v1.0.0](https://github.com/saagar210/DeepTank/releases/tag/v1.0.0)
```

### Announce Release (Optional)
- Tweet/post on social media if applicable
- Create GitHub Discussion thread
- Add to project website

## Troubleshooting

### "Tag already exists"
If you see this error, the tag was already pushed. Delete locally and remotely:
```bash
git tag -d v1.0.0
git push origin :v1.0.0  # Delete remote tag
```

Then rerun steps.

### "Workflow failed"
Check GitHub Actions logs at https://github.com/saagar210/DeepTank/actions
- Most common: Missing system dependency on Linux (check build.yml logs)
- Solution: Update `.github/workflows/build.yml` and retry

### "Binaries not appearing"
Wait 5 minutes and refresh the Releases page. GitHub's artifact upload can be slow.

## Version Numbering

DeepTank uses Semantic Versioning:

- **MAJOR** (X.0.0) — Breaking changes (rare)
- **MINOR** (1.X.0) — New features, backwards compatible
- **PATCH** (1.0.X) — Bug fixes only

Example progression:
- 1.0.0 (initial release)
- 1.1.0 (new feature: AI narration)
- 1.1.1 (bug fix: crash on empty tank)
- 2.0.0 (architecture change, old saves incompatible)

## Automated vs Manual

**Automated (CI/CD):**
- ✓ Cross-platform builds
- ✓ Binary signing (future)
- ✓ GitHub Release creation
- ✓ Artifact upload

**Manual:**
- ✓ Version bumping
- ✓ Changelog generation
- ✓ Git push
- ✓ Release verification
