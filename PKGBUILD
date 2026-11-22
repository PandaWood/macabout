# Maintainer: Peter van der Woude <pandawoude@gmail.com>

pkgname=macabout
pkgver=1.0.11
pkgrel=1
pkgdesc='About This Computer dialog, Apple-style'
arch=('any')
url='https://github.com/PandaWood/macabout'
license=('MIT')
depends=('python' 'tk' 'pciutils')
optdepends=(
  'python-pillow: higher-quality distro icon scaling'
  'librsvg: SVG distro icon rendering'
)
source=("$pkgname-$pkgver.tar.gz::$url/archive/refs/tags/v$pkgver.tar.gz")
# Replaced with the real checksum by .github/workflows/release.yml before the
# PKGBUILD is pushed to the AUR. Kept as SKIP in-repo so that `makepkg -si`
# works from a fresh clone without the tag having been published yet.
sha256sums=('55361c1a7283af269b9b9b65b1ae38d67158bb720299cf31831c0cb4c1bd3f38')

package() {
  cd "$srcdir/$pkgname-$pkgver"

  # The package tree deliberately lives outside the versioned site-packages
  # directory (/usr/lib/python3.X/site-packages) so that a Python minor-version
  # bump doesn't orphan the install and force a rebuild. The /usr/bin launcher
  # sets PYTHONPATH to match. Data files still resolve, because both lookups
  # (formatters.py, ui.py) are relative to Path(__file__).parent.
  install -d "$pkgdir/usr/lib/macabout"
  cp -r macabout "$pkgdir/usr/lib/macabout/"
  find "$pkgdir/usr/lib/macabout" -name __pycache__ -type d -exec rm -rf {} +
  python -m compileall -q --invalidation-mode=unchecked-hash \
    -s "$pkgdir" -p / "$pkgdir/usr/lib/macabout/macabout"

  install -Dm755 arch/macabout.sh "$pkgdir/usr/bin/macabout"
  install -Dm644 debian/usr/share/applications/macabout.desktop \
    "$pkgdir/usr/share/applications/macabout.desktop"
  install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
