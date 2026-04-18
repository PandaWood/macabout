# macabout

macOS has a clean, human-readable "About This Mac" dialog that displays your hardware at a glance — processor, memory, graphics, serial number. Linux exposes the same information but buries it in raw technical strings across multiple system tools.

macabout presents your Linux machine's hardware in exactly the same format. If you're selling a laptop, donating a machine, or just want a clean system summary, macabout gives you the same at-a-glance view that Mac users take for granted.

## Running on Linux

```bash
python3 -m macabout
```

Shows real hardware. `python3-tk`, `pciutils`, and `dmidecode` must be present — the `.deb` installer handles all of this automatically (see [Building the .deb](#building-the-deb)).

## Developing on macOS

tkinter ships separately from Python on macOS. Install both via Homebrew, matching the version numbers:

```bash
brew install python@3.14
brew install python-tk@3.14
```

Then set up the virtualenv:

```bash
make dev
source .venv/bin/activate
```

Run:

```bash
make mock     # static Zorin OS sample data (good for UI work)
make run      # real macOS system calls
```

Or directly:

```bash
python -m macabout --mock
```

`requirements.txt` contains only `Pillow`, which enables clean icon resizing. Everything else (`tkinter`, `subprocess`, `pathlib`, etc.) is Python standard library.

**Requires:** Python 3.10+, `python3-tk`

**Optional:** `Pillow` (pip) for clean icon resizing; `dmidecode` for memory speed/type and serial number; `pciutils` for GPU detection.

## What it shows

| Field | Source | Example output |
|---|---|---|
| Distro name | `/etc/os-release` | Zorin OS |
| Version | `/etc/os-release` | Version 17.1 |
| Processor | `/proc/cpuinfo` | 2.9 GHz Intel Core i7 |
| Memory | `/proc/meminfo` + `dmidecode` | 8 GB 1600 MHz DDR3 |
| Graphics | `lspci` + bundled lookup table | Intel HD Graphics 5000 1536 MB |
| Serial Number | `dmidecode` | C02J1234XYZA |

Memory speed/type and serial number require `dmidecode`. If unavailable (e.g. not installed or no root access), those fields degrade gracefully.

## Distro icon

The icon is sourced from the running system's own branding, in this order:

1. `LOGO=` field in `/etc/os-release` (explicit XDG icon name — most authoritative)
2. `distributor-logo` (FreeDesktop standard, present on most distros)
3. `distributor-logo-{id}`, `{id}-logo`, `{id}` (fallback guesses)
4. A bundled PNG at `macabout/data/icons/{distro_id}.png` if present
5. A brand-colored circle with the distro's initial letter (pure tkinter, no extra deps)

To add a bundled icon for a distro, drop a PNG named `{distro_id}.png` (200×200px) into `macabout/data/icons/`. The `distro_id` matches the `ID=` field in `/etc/os-release` (e.g. `zorin.png`, `ubuntu.png`).

## GPU lookup table

Graphics card names and VRAM figures are resolved via `macabout/data/gpu_lookup.json`, keyed by PCI vendor:device ID (e.g. `"8086:0a26"`). The file ships with ~60 entries covering Intel HD/Iris/UHD Graphics from Sandy Bridge through Coffee Lake, plus a handful of common AMD and NVIDIA cards. Add entries to extend coverage without touching application code.

## Building the .deb

```bash
./build.sh
sudo apt install ./build/macabout_1.0.0_all.deb
```

Requires `dpkg-deb` (standard on Debian/Ubuntu). The resulting package declares `python3-tk` and `pciutils` as dependencies, and recommends `dmidecode`, `lshw`, and `python3-pil`.

## Project structure

```
macabout/
├── macabout/
│   ├── app.py                # entry point, argument parsing
│   ├── hwinfo.py             # SystemInfo dataclass + platform dispatch
│   ├── linux_collectors.py   # real hardware queries (Linux)
│   ├── mac_collectors.py     # real hardware queries (macOS, for development)
│   ├── mock_data.py          # static sample data for cross-platform dev
│   ├── formatters.py         # raw strings → clean marketing names
│   ├── ui.py                 # tkinter dialog
│   └── data/
│       ├── gpu_lookup.json   # PCI ID → {name, vram_mb}
│       └── icons/            # optional bundled distro PNGs
├── debian/                   # dpkg-deb package template
└── build.sh                  # builds macabout_x.x.x_all.deb
```
