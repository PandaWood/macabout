# One-Line Installer Setup

This setup provides a simple one-line installer for macabout.

## What You Get

Users can install macabout with:

```bash
curl -sSL https://pandawood.github.io/macabout/install.sh | bash
```

The script detects the distro family from `/etc/os-release` and takes one of
two paths:

| Family                               | What it does                                                                 |
|--------------------------------------|------------------------------------------------------------------------------|
| Debian, Ubuntu, Mint, Zorin…         | Downloads the latest `.deb` from Releases, installs with `dpkg` + `apt-get -f` |
| Arch, CachyOS, Manjaro, EndeavourOS… | `paru`/`yay` from the AUR, or downloads the release `PKGBUILD` and runs `makepkg -si` |

Anything else gets pointed at the README's "Running from source" section.

> **Note:** the installer must be run as a normal user, not `sudo bash` —
> `makepkg` refuses to run as root. It calls `sudo` itself where it needs root.
> The Debian path still tolerates being run as root, so the old
> `| sudo bash` command keeps working for anyone who bookmarked it.

## Setup (One-Time)

### 1. Enable GitHub Pages

1. Go to https://github.com/PandaWood/macabout/settings/pages
2. Under "Build and deployment":
   - **Source:** Select "Deploy from a branch"
   - **Branch:** Select `gh-pages` branch and `/ (root)` folder
   - Click **Save**

The `gh-pages` branch will be created automatically when you create your first release.

### 2. Set Up AUR Publishing

The release workflow pushes `arch/PKGBUILD` to the AUR on every tag, so Arch
users get `paru -S macabout`. This needs a one-time account setup — until the
secrets exist the workflow simply skips the publish step, so releases keep
working in the meantime.

1. Check the name is free at https://aur.archlinux.org/packages/macabout
2. Create an account at https://aur.archlinux.org/register
3. Generate a dedicated key pair (no passphrase — CI can't type one):

   ```bash
   ssh-keygen -t ed25519 -f ~/.ssh/aur -C "macabout CI" -N ""
   ```

4. Paste `~/.ssh/aur.pub` into the "SSH Public Key" field of your AUR account
   settings
5. Add three repository secrets at
   https://github.com/PandaWood/macabout/settings/secrets/actions:

   | Secret                 | Value                          |
   |------------------------|--------------------------------|
   | `AUR_USERNAME`         | Your AUR username              |
   | `AUR_EMAIL`            | The email on your AUR account  |
   | `AUR_SSH_PRIVATE_KEY`  | Contents of `~/.ssh/aur`       |

The first tagged release after this creates the AUR package; later releases
update it.

### 3. Commit These Changes

```bash
git add .
git commit -m "Add one-line installer"
git push origin main
```

### 4. Create a Release

The version must match in three places before tagging — `macabout/__init__.py`,
`debian/DEBIAN/control` and `arch/PKGBUILD`. CI fails the build if they drift.

```bash
git tag v1.0.10
git push origin v1.0.10
```

The workflow will:
- Build the .deb
- Stamp `arch/PKGBUILD` with the tag version and the source tarball's sha256
- Push the PKGBUILD to the AUR
- Create a GitHub Release with the .deb, the PKGBUILD and install.sh
- Deploy install.sh, the PKGBUILD and index.html to GitHub Pages

### 5. Test It

After the workflow completes (2-3 minutes):

Visit https://pandawood.github.io/macabout/ to see your landing page.

Test the installer on a Linux machine:
```bash
curl -sSL https://pandawood.github.io/macabout/install.sh | bash
```

## How It Works

1. **install.sh** - Detects the distro, then installs the .deb from the GitHub API or the AUR package
2. **arch/PKGBUILD** - Arch build recipe; pkgver and sha256sums are stamped in at release time
3. **docs/index.html** - Landing page that shows the install command
4. **Workflow** - Copies install.sh and the PKGBUILD to docs/ and deploys to GitHub Pages on each release

## Files

- `install.sh` - Installer script (committed to repo)
- `arch/PKGBUILD` - AUR package recipe (committed to repo, stamped at release)
- `docs/index.html` - Landing page (committed to repo)
- `.github/workflows/release.yml` - Builds, publishes to the AUR, deploys to GitHub Pages

## Benefits vs APT Repository

✅ **Simple**: One command, no GPG keys, no apt source management
✅ **Secure**: Downloads from GitHub Releases (GitHub's infrastructure)
✅ **Maintenance**: Zero ongoing maintenance needed
✅ **Fast**: No complex repository metadata to generate

## Limitations

Debian users need to re-run the install command to update. But for a utility app like this, that's fine. Arch users get updates through their normal `paru -Syu`.

The PKGBUILD's `sha256sums` is taken from GitHub's auto-generated tag tarball. Those have historically been stable, but if a checksum ever stops matching, bump `pkgrel` and re-release rather than editing a published tag.
