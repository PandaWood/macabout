#!/bin/sh
# The package tree is installed to /usr/lib/macabout rather than the versioned
# site-packages directory, so point Python at it explicitly. See arch/PKGBUILD.
PYTHONPATH=/usr/lib/macabout exec python3 -m macabout "$@"
