#!/bin/bash
set -euo pipefail

cd "$(dirname "$0")"

VERSION="$(python3 -c "import sys; sys.path.insert(0, '.'); from macabout import __version__; print(__version__)")"
PKG="macabout_${VERSION}_all"
BUILD="build/${PKG}"

rm -rf build
mkdir -p "${BUILD}/DEBIAN"
mkdir -p "${BUILD}/usr/bin"
mkdir -p "${BUILD}/usr/share/applications"
mkdir -p "${BUILD}/usr/lib/python3/dist-packages"

cp debian/DEBIAN/control "${BUILD}/DEBIAN/control"
cp debian/usr/bin/macabout "${BUILD}/usr/bin/macabout"
chmod 755 "${BUILD}/usr/bin/macabout"
cp debian/usr/share/applications/macabout.desktop "${BUILD}/usr/share/applications/macabout.desktop"
cp -r macabout "${BUILD}/usr/lib/python3/dist-packages/"

dpkg-deb --build --root-owner-group "${BUILD}"
echo ""
echo "Built: build/${PKG}.deb"
echo "Install: sudo apt install ./build/${PKG}.deb"
