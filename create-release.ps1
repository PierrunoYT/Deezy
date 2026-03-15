# Script to create GitHub release
# Run this AFTER building the installers on Windows
# Usage: ./create-release.ps1  (from repo root, in PowerShell)

$ErrorActionPreference = "Stop"

$VERSION = "0.2.14"
$TAG = "v$VERSION"

$MSI_PATH = "deezy/src-tauri/target/release/bundle/msi/Deezy_${VERSION}_x64_en-US.msi"
$NSIS_PATH = "deezy/src-tauri/target/release/bundle/nsis/Deezy_${VERSION}_x64-setup.exe"

if (-not (Test-Path $MSI_PATH)) {
    Write-Error "MSI installer not found at $MSI_PATH`nPlease build the app first using: npm run tauri build"
    exit 1
}

if (-not (Test-Path $NSIS_PATH)) {
    Write-Error "NSIS installer not found at $NSIS_PATH`nPlease build the app first using: npm run tauri build"
    exit 1
}

# Get release notes from CHANGELOG (from ## [VERSION] to next ## [)
$lines = Get-Content "CHANGELOG.md"
$noteLines = @()
$inSection = $false
foreach ($line in $lines) {
    if ($line -match "## \[$([regex]::Escape($VERSION))\]") {
        $inSection = $true
        continue
    }
    if ($inSection -and $line -match "^## \[") { break }
    if ($inSection) { $noteLines += $line }
}
$NOTES = if ($noteLines.Count -gt 0) { $noteLines -join "`n" } else { "Bug fixes and improvements" }
$notesFile = [System.IO.Path]::GetTempFileName()
$NOTES | Out-File -FilePath $notesFile -Encoding utf8

Write-Host "Creating GitHub release $TAG..."

# Ensure the local tag exists
git rev-parse -q --verify "refs/tags/$TAG" 2>$null | Out-Null
if ($LASTEXITCODE -ne 0) {
    Write-Host "Local tag $TAG not found. Creating it at HEAD..."
    git tag $TAG
}

# Check if remote tag exists
$remoteTag = git ls-remote --tags origin $TAG 2>$null
if ($remoteTag -and $remoteTag -match "refs/tags/$TAG") {
    Write-Host "Remote tag $TAG already exists on origin."
} else {
    Write-Host "Pushing tag $TAG to origin..."
    git push origin $TAG
}

# Create the release with installers
try {
    gh release create $TAG `
        --title "Deezy v$VERSION" `
        --notes-file $notesFile `
        $MSI_PATH `
        $NSIS_PATH
} finally {
    Remove-Item $notesFile -ErrorAction SilentlyContinue
}

Write-Host ""
Write-Host "Release created successfully!"
$repo = gh repo view --json nameWithOwner -q .nameWithOwner
Write-Host "View it at: https://github.com/$repo/releases/tag/$TAG"
