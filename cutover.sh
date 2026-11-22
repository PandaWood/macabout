#!/usr/bin/env bash
# Walks through releasing macabout as 2.0.0, one step at a time.
# Run from anywhere: bash cutover.sh
# Each step says what to do, waits for Enter, checks what it can, and says how to go back.
set -uo pipefail

cd "$(dirname "$0")"
VERSION="2.0.0"
REPO="PandaWood/macabout"

bold() { printf '\n\033[1m%s\033[0m\n' "$*"; }
ok() { printf '  \033[32m✓\033[0m %s\n' "$*"; }
bad() { printf '  \033[31m✗\033[0m %s\n' "$*"; }
back() { printf '  \033[33mWay back:\033[0m %s\n' "$*"; }
wait_enter() { read -r -p "  Press Enter when done (Ctrl-C to stop here)... " _; }
ask() { local a; read -r -p "  $1 [y/N] " a; [[ "$a" == [yY]* ]]; }

bold "macabout cutover to Rust, v${VERSION}"

bold "1. Check the tree"
if cargo test --quiet 2>&1 | tail -3; then
  ok "cargo test passes"
else
  bad "cargo test failed: fix before going on"; exit 1
fi
git status --short | head -20
echo "  Review the changes (git diff) before committing."
back "nothing is published yet; git stash or git checkout undoes local changes."
wait_enter

bold "2. Set the version to ${VERSION}"
echo "  Three files must agree (CI checks): Cargo.toml (and its Cargo.lock),"
echo "  debian/DEBIAN/control, arch/PKGBUILD."
if ask "Set them to ${VERSION} now?"; then
  sed -i "s/^Version: .*/Version: ${VERSION}/" debian/DEBIAN/control
  sed -i "s/^pkgver=.*/pkgver=${VERSION}/" arch/PKGBUILD
  sed -i "0,/^version = .*/s//version = \"${VERSION}\"/" Cargo.toml
  cargo metadata --format-version 1 >/dev/null
fi
found=$(grep -h -o "${VERSION}" debian/DEBIAN/control arch/PKGBUILD Cargo.toml | wc -l)
[ "$found" -eq 3 ] && ok "all three files say ${VERSION}" || bad "only ${found} of 3 files say ${VERSION}"
back "git checkout -- debian/DEBIAN/control arch/PKGBUILD Cargo.toml Cargo.lock"

bold "3. Commit, push, open a pull request into main"
echo "  Commit on the 'rust' branch, push it, and open a PR. Wait for both CI jobs"
echo "  (validate, rust) to pass: the rust job builds the .deb on Ubuntu 22.04."
wait_enter
if command -v gh >/dev/null; then
  gh pr checks --repo "$REPO" 2>/dev/null && ok "PR checks pass" || bad "PR checks not green (or no PR for this branch)"
fi
back "close the PR; main is untouched until you merge."

bold "4. Merge into main"
wait_enter
git fetch -q origin main && git log -1 --format='  main is at %h %s' origin/main
back "git revert the merge commit on main and push."

bold "5. Tag v${VERSION} on main and push the tag"
echo "  git checkout main && git pull && git tag v${VERSION} && git push origin v${VERSION}"
echo "  The release workflow builds amd64 and arm64 .debs, publishes the release, the AUR"
echo "  package (if the AUR secrets are set) and the install page."
wait_enter
if command -v gh >/dev/null; then
  gh run list --repo "$REPO" --workflow Release --limit 1
fi
back "gh release delete v${VERSION} --repo ${REPO} --cleanup-tag --yes, then re-run the v1.0.10 Release workflow (gh run rerun <id>) so 'latest' and the install page point at it again."

bold "6. Check the release assets"
wait_enter
if command -v gh >/dev/null; then
  assets=$(gh release view "v${VERSION}" --repo "$REPO" --json assets -q '.assets[].name' 2>/dev/null)
  for want in "macabout_${VERSION}_amd64.deb" "macabout_${VERSION}_arm64.deb" PKGBUILD install.sh; do
    grep -qx "$want" <<<"$assets" && ok "$want" || bad "$want missing"
  done
else
  echo "  (gh not installed: check https://github.com/${REPO}/releases by hand)"
fi
back "as in step 5."

bold "7. Install on real machines"
echo "  On a Debian/Ubuntu/Zorin machine (amd64, and arm64 if you have one) and on an Arch one:"
echo "    curl -sSL https://pandawood.github.io/macabout/install.sh | bash"
echo "    macabout            # the dialog: compare with the 1.0.10 one"
echo "    macabout --text     # the summary on stdout"
echo "  Check: logo, dark/light mode, the copy button, memory speed/type, no password prompt."
wait_enter
if command -v macabout >/dev/null && macabout --text >/dev/null 2>&1; then
  ok "macabout --text runs on this machine"
fi
back "sudo apt install ./macabout_1.0.10_all.deb (from the v1.0.10 release), or on Arch build the v1.0.10 PKGBUILD."

bold "Done"
echo "  This script has done its job and can be deleted."
