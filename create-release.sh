#!/bin/bash
# Script to create GitHub release
# Run this AFTER building the installers on Windows

set -e

VERSION="0.2.11"
TAG="v${VERSION}"

# Check if installers exist
MSI_PATH="deezy/src-tauri/target/release/bundle/msi/Deezy_${VERSION}_x64_en-US.msi"
NSIS_PATH="deezy/src-tauri/target/release/bundle/nsis/Deezy_${VERSION}_x64-setup.exe"

if [ ! -f "$MSI_PATH" ]; then
    echo "Error: MSI installer not found at $MSI_PATH"
    echo "Please build the app first using: npm run tauri build"
    exit 1
fi

if [ ! -f "$NSIS_PATH" ]; then
    echo "Error: NSIS installer not found at $NSIS_PATH"
    echo "Please build the app first using: npm run tauri build"
    exit 1
fi

# Get release notes from the specific version in CHANGELOG
NOTES=$(sed -n "/## \[${VERSION}\]/,/## \[/p" CHANGELOG.md | sed '$d' | tail -n +3 | sed 's/"/\\"/g' | sed ':a;N;$!ba;s/\n/\\n/g')
if [ -z "$NOTES" ]; then
    NOTES="Bug fixes and improvements"
fi

echo "Creating GitHub release $TAG..."

# Ensure the local tag exists
if ! git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then
    echo "Local tag $TAG not found. Creating it at HEAD..."
    git tag "$TAG"
fi

# Ensure we don't conflict with an existing remote tag
if git ls-remote --tags origin "$TAG" | grep -q "refs/tags/$TAG$"; then
    echo "Remote tag $TAG already exists on origin."
else
    echo "Pushing tag $TAG to origin..."
    git push origin "$TAG"
fi

# Create the release with installers
gh release create $TAG \
    --title "Deezy v${VERSION}" \
    --notes "$(echo -e "$NOTES")" \
    "$MSI_PATH" \
    "$NSIS_PATH"

echo ""
echo "Release created successfully!"
echo "View it at: https://github.com/$(gh repo view --json nameWithOwner -q .nameWithOwner)/releases/tag/$TAG"
