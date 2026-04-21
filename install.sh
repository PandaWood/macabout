#!/bin/bash
set -e

# macabout installer script
# Usage: curl -sSL https://pandawood.github.io/macabout/install.sh | sudo bash

REPO="PandaWood/macabout"
TEMP_DIR=$(mktemp -d)
trap 'echo ""; echo "❌ Installation cancelled."; rm -rf "$TEMP_DIR"; exit 1' INT TERM
REQUIRED_DEPS="python3-tk pciutils dmidecode"

echo "🔍 Fetching latest macabout release..."

# Get latest release info from GitHub API
LATEST_URL=$(curl -sSL "https://api.github.com/repos/${REPO}/releases/latest" \
  | grep "browser_download_url.*\.deb" \
  | cut -d '"' -f 4)

if [ -z "$LATEST_URL" ]; then
  echo "❌ Error: Could not find latest release"
  exit 1
fi

DEB_FILE="${TEMP_DIR}/$(basename "$LATEST_URL")"

echo "📥 Downloading $(basename "$LATEST_URL")..."
curl -sSL "$LATEST_URL" -o "$DEB_FILE"

echo "📦 Installing macabout..."
chmod 644 "$DEB_FILE"
apt install -y "$DEB_FILE"

echo "✓ macabout installed successfully!"
echo ""
echo "Run: macabout"

# Cleanup
rm -rf "$TEMP_DIR"
