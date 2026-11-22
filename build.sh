#!/bin/bash
# Builds the macabout .deb for this machine's architecture (amd64 or arm64).
# The binary links against the glibc of the build machine: release packages are built on
# the oldest supported Ubuntu (see .github/workflows/release.yml).
set -euo pipefail

cd "$(dirname "$0")"

VERSION="$(sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1)"
case "$(uname -m)" in
  x86_64) ARCH=amd64 ;;
  aarch64 | arm64) ARCH=arm64 ;;
  *) echo "Unsupported architecture: $(uname -m)" >&2; exit 1 ;;
esac
PKG="macabout_${VERSION}_${ARCH}"
BUILD="build/${PKG}"

cargo build --release --locked

rm -rf build
mkdir -p "${BUILD}/DEBIAN" "${BUILD}/usr/bin" "${BUILD}/usr/share/applications" \
  "${BUILD}/usr/share/doc/macabout"

sed "s/^Architecture: .*/Architecture: ${ARCH}/" debian/DEBIAN/control > "${BUILD}/DEBIAN/control"
install -m755 target/release/macabout "${BUILD}/usr/bin/macabout"
install -m644 debian/usr/share/applications/macabout.desktop "${BUILD}/usr/share/applications/"
install -m644 LICENSE "${BUILD}/usr/share/doc/macabout/copyright"

dpkg-deb --build --root-owner-group "${BUILD}"
echo ""
echo "Built: build/${PKG}.deb"
echo "Install: sudo apt install ./build/${PKG}.deb"
