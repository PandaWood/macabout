#!/usr/bin/env bash
set -euo pipefail

# macabout installer script
# Usage: curl -sSL https://pandawood.github.io/macabout/install.sh | bash
#
# Run this as your normal user, NOT under sudo — the script calls sudo itself
# where it needs root. On Arch this is a hard requirement, because makepkg
# refuses to run as root. Piping to `sudo bash` still works on Debian for
# anyone with the old command bookmarked.

REPO="PandaWood/macabout"
API="https://api.github.com/repos/${REPO}/releases/latest"

TEMP_DIR="$(mktemp -d)"
chmod 755 "$TEMP_DIR"
cleanup() {
  if [ -n "${TEMP_DIR:-}" ]; then
    rm -rf "$TEMP_DIR"
  fi
}
trap cleanup EXIT
trap 'echo ""; echo "❌ Installation cancelled."; exit 1' INT TERM

if [ "$(id -u)" -eq 0 ]; then
  SUDO=""
else
  SUDO="sudo"
fi

# Which install path to take. Everything that isn't recognised falls back to
# the "run from source" instructions in the README.
detect_family() {
  if [ ! -r /etc/os-release ]; then
    echo "unknown"
    return
  fi
  # shellcheck disable=SC1091
  . /etc/os-release
  for id in "${ID:-}" ${ID_LIKE:-}; do
    case "$id" in
      debian | ubuntu) echo "debian"; return ;;
      arch | cachyos | manjaro | endeavouros) echo "arch"; return ;;
    esac
  done
  echo "unknown"
}

# $1 = grep pattern matching the tail of the asset filename
fetch_asset_url() {
  curl -sSL "$API" \
    | grep -o "\"browser_download_url\": *\"[^\"]*$1\"" \
    | head -1 \
    | cut -d '"' -f 4
}

install_debian() {
  echo "🔍 Fetching latest macabout release..."
  local url deb
  url="$(fetch_asset_url '\.deb')"
  if [ -z "$url" ]; then
    echo "❌ Error: Could not find a .deb in the latest release"
    exit 1
  fi

  deb="${TEMP_DIR}/$(basename "$url")"
  echo "📥 Downloading $(basename "$url")..."
  curl -sSL "$url" -o "$deb"
  chmod 644 "$deb"

  echo "📦 Installing macabout..."
  # dpkg exits non-zero when dependencies are missing; apt-get -f resolves them.
  $SUDO dpkg -i "$deb" || true
  $SUDO apt-get install -f -y
}

install_arch() {
  local helper="" url
  for candidate in paru yay; do
    if command -v "$candidate" >/dev/null 2>&1; then
      helper="$candidate"
      break
    fi
  done

  if [ -n "$helper" ]; then
    echo "📦 Installing macabout from the AUR with ${helper}..."
    if "$helper" -S --needed --noconfirm macabout; then
      return
    fi
    echo ""
    echo "⚠️  ${helper} could not install macabout from the AUR."
    echo "   Falling back to building the PKGBUILD from the latest release."
    echo ""
  else
    echo "ℹ️  No AUR helper (paru/yay) found — building from the PKGBUILD instead."
  fi

  echo "📦 Installing build tools..."
  $SUDO pacman -S --needed --noconfirm base-devel

  echo "🔍 Fetching latest macabout release..."
  url="$(fetch_asset_url 'PKGBUILD')"
  if [ -z "$url" ]; then
    echo "❌ Error: Could not find a PKGBUILD in the latest release"
    exit 1
  fi

  echo "📥 Downloading PKGBUILD..."
  curl -sSL "$url" -o "${TEMP_DIR}/PKGBUILD"

  echo "🔨 Building and installing macabout..."
  cd "$TEMP_DIR"
  makepkg -si --noconfirm
}

FAMILY="$(detect_family)"

case "$FAMILY" in
  debian)
    ;;
  arch)
    if [ -z "$SUDO" ]; then
      echo "❌ Do not run this installer as root on Arch — makepkg refuses to"
      echo "   run as root. Re-run it as your normal user:"
      echo ""
      echo "   curl -sSL https://pandawood.github.io/macabout/install.sh | bash"
      exit 1
    fi
    ;;
  *)
    echo "ℹ️  No packaged install is available for this distro yet."
    echo ""
    echo "   macabout itself runs anywhere — see \"Running from source\" at"
    echo "   https://github.com/PandaWood/macabout#running-from-source"
    exit 0
    ;;
esac

# Prime the sudo credential cache up front. sudo reads its prompt from the
# terminal, so this works even though stdin is the piped script — but pacman
# and apt prompts would read EOF, hence --noconfirm / -y throughout.
if [ -n "$SUDO" ]; then
  if ! $SUDO -v; then
    echo "❌ Error: this installer needs sudo access."
    exit 1
  fi
fi

"install_${FAMILY}"

echo ""
echo "✓ macabout installed successfully!"
echo ""
echo "Run: macabout"
