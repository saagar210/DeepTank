#!/bin/bash
# generate-changelog.sh — Auto-generate CHANGELOG.md from git history
# Usage: .github/scripts/generate-changelog.sh

set -e

# Get version from package.json
VERSION=$(grep '"version"' package.json | head -1 | sed 's/.*: "\([^"]*\)".*/\1/')
DATE=$(date +%Y-%m-%d)

# Get last tag (if any) or use "HEAD" if no tags exist
LAST_TAG=$(git describe --tags --abbrev=0 2>/dev/null || echo "HEAD")

if [ "$LAST_TAG" = "HEAD" ]; then
  # No tags exist yet — show all commits
  COMMITS=$(git log --oneline | head -20)
else
  # Show commits since last tag
  COMMITS=$(git log --oneline $LAST_TAG..HEAD | head -20)
fi

# Separate commits by type using conventional commit format
FEATURES=$(echo "$COMMITS" | grep "feat:" | sed 's/^[^ ]* feat: /- /' || echo "")
FIXES=$(echo "$COMMITS" | grep "fix:" | sed 's/^[^ ]* fix: /- /' || echo "")
CHANGES=$(echo "$COMMITS" | grep "refactor:" | sed 's/^[^ ]* refactor: /- /' || echo "")

# Build changelog entry
cat > CHANGELOG_ENTRY.tmp << EOF
## [$VERSION] - $DATE

EOF

if [ -n "$FEATURES" ]; then
  cat >> CHANGELOG_ENTRY.tmp << EOF
### Added
$FEATURES

EOF
fi

if [ -n "$FIXES" ]; then
  cat >> CHANGELOG_ENTRY.tmp << EOF
### Fixed
$FIXES

EOF
fi

if [ -n "$CHANGES" ]; then
  cat >> CHANGELOG_ENTRY.tmp << EOF
### Changed
$CHANGES

EOF
fi

cat >> CHANGELOG_ENTRY.tmp << EOF
---

EOF

# Prepend to existing CHANGELOG.md (if it exists)
if [ -f CHANGELOG.md ]; then
  cat CHANGELOG.md >> CHANGELOG_ENTRY.tmp
fi

mv CHANGELOG_ENTRY.tmp CHANGELOG.md

echo "✓ CHANGELOG.md generated for version $VERSION"
echo "  - Date: $DATE"
echo "  - Features: $(echo "$FEATURES" | wc -l) items"
echo "  - Fixes: $(echo "$FIXES" | wc -l) items"
echo "  - Changes: $(echo "$CHANGES" | wc -l) items"
