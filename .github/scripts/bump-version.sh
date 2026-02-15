#!/bin/bash
# bump-version.sh — Update version in Cargo.toml and package.json
# This script is called by `npm version patch|minor|major`
# It keeps Cargo.toml and package.json in sync

set -e

# Get the new version from package.json (npm version updates it first)
NEW_VERSION=$(grep '"version"' package.json | head -1 | sed 's/.*: "\([^"]*\)".*/\1/')

# Update Cargo.toml to match
sed -i "s/version = \"[^\"]*\"/version = \"$NEW_VERSION\"/" src-tauri/Cargo.toml

echo "✓ Version bumped to $NEW_VERSION"
echo "  - package.json: $NEW_VERSION"
echo "  - src-tauri/Cargo.toml: $NEW_VERSION"
