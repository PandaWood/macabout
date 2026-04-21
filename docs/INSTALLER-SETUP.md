# One-Line Installer Setup

This setup provides a simple one-line installer for macabout.

## What You Get

Users can install macabout with:

```bash
curl -sSL https://pandawood.github.io/macabout/install.sh | sudo bash
```

The script:
- Downloads the latest .deb from GitHub Releases
- Installs it with apt
- Handles dependencies automatically

## Setup (One-Time)

### 1. Enable GitHub Pages

1. Go to https://github.com/PandaWood/macabout/settings/pages
2. Under "Build and deployment":
   - **Source:** Select "Deploy from a branch"
   - **Branch:** Select `gh-pages` branch and `/ (root)` folder
   - Click **Save**

The `gh-pages` branch will be created automatically when you create your first release.

### 2. Commit These Changes

```bash
git add .
git commit -m "Add one-line installer"
git push origin main
```

### 3. Create a Release

```bash
git tag v1.0.1
git push origin v1.0.1
```

The workflow will:
- Build the .deb
- Deploy install.sh and index.html to GitHub Pages
- Create a GitHub Release with both files

### 4. Test It

After the workflow completes (2-3 minutes):

Visit https://pandawood.github.io/macabout/ to see your landing page.

Test the installer on a Linux machine:
```bash
curl -sSL https://pandawood.github.io/macabout/install.sh | sudo bash
```

## How It Works

1. **install.sh** - Downloads latest .deb from GitHub API and installs it
2. **docs/index.html** - Landing page that shows the install command
3. **Workflow** - Copies install.sh to docs/ and deploys to GitHub Pages on each release

## Files

- `install.sh` - Installer script (committed to repo)
- `docs/index.html` - Landing page (committed to repo)
- `.github/workflows/release.yml` - Updated to deploy to GitHub Pages

## Benefits vs APT Repository

✅ **Simple**: One command, no GPG keys, no apt source management
✅ **Secure**: Downloads from GitHub Releases (GitHub's infrastructure)
✅ **Maintenance**: Zero ongoing maintenance needed
✅ **Fast**: No complex repository metadata to generate

## Limitation

Users need to re-run the command to update. But for a utility app like this, that's fine. If they want automatic updates, they can use a package manager alternative or check for updates manually.
