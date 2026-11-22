# Design notes

Why the code is the way it is, where that is not obvious from reading it.

## Layout

- `src/main.rs`: the command line (clap).
- `src/system.rs`: reads `/etc/os-release`, `/proc`, sysfs, the device tree, `lspci`, `lscpu`
  and udev, or makes up a computer for `--mock`. All the I/O is here.
- `src/parse.rs`: turns that text into facts. No I/O, so it is unit-tested on stored output.
- `src/format.rs`: turns the facts into the names the dialog shows, and the `--text` summary.
- `src/ui/`: the dialog (egui): layout, fonts, logo, light and dark colours.
- `data/gpu_lookup.json`: marketing names for graphics cards by PCI id, built into the binary.

## Decisions

**egui.** The dialog paints its own colours, so a native widget look would buy nothing, and
egui adds no runtime package that a desktop does not already have. The clipboard goes through
egui-winit (arboard on X11, smithay-clipboard on Wayland).

**Fonts.** egui only knows its built-in fonts, so the dialog loads the desktop's: `fontdb`
finds the first installed family of `FONT_CANDIDATES`, and fontconfig (`fc-match`) names the
desktop's `sans-serif` when none is installed. Glyphs the chosen face lacks come from
`egui_system_font_provider`. Sizes are points at 96 dpi; each label is its text plus 2 px of
padding per side.

**Logos.** The distro's own PNG or SVG, rendered in-process: `resvg` draws an SVG at twice the
size, stretched to fill the square, then `image` scales it down with Lanczos3 on premultiplied
pixels so transparent edges do not darken. It is drawn at the display's pixel density. `.xpm`
logos are not looked for; a distro with only an `.xpm` gets the brand-coloured circle.

**No root.** Everything macabout reads is readable by any user: the model from
`/sys/class/dmi/id/product_name`, memory type and speed from the `MEMORY_DEVICE_*` properties
udev reads from the firmware at boot (`udevadm info`, systemd 248 and later). The serial number
is readable only by root, so it is not shown. Up to 1.0.10 macabout ran `dmidecode` through a
sudoers rule for these; that rule and the dependency are gone.

**ARM.** `/proc/cpuinfo` on ARM has part numbers, not a model name, so the CPU is named by
`lscpu` (run with `LC_ALL=C` for English labels). Without a DMI table the model comes from
`/proc/device-tree/model` ("Raspberry Pi 4 Model B Rev 1.4"). A graphics processor built into
the chip is not on PCI, so it is named after its DRM driver: VideoCore, Mali, Adreno and so on,
without a model number. ARM boards rarely have the firmware memory table, so memory shows only
its size.

**Graphics memory** is read from the sysfs node of the PCI device `lspci` named, so a machine
with two cards never shows one card's name with the other's memory.

**Dark mode.** The first of gsettings, `GTK_THEME`, GTK's `settings.ini`, dconf, dconf's
database file and the XDG desktop portal that says dark wins.

**Start-up.** Reading the computer, finding the colour scheme and loading the fonts each run
commands or read files, so they run on three threads before the window opens. Every command
has a 3-second deadline, which holds even if something the command started keeps its output
open.

**Linux only.** No macOS code. `--mock` is for working on the dialog without real hardware.

**Packaging.** The binary links only libc, libm and libgcc_s and loads the rest at run time
(EGL/GL, Wayland, X11/xcb, xkbcommon, fontconfig), so the `.deb` and PKGBUILD list those as
dependencies by hand. Release `.deb`s are built on Ubuntu 22.04 (amd64 and arm64), the oldest
supported base (Zorin 17), so they need glibc 2.35; a `.deb` built locally on a newer distro
needs that distro's glibc. The PKGBUILD builds with `cargo --frozen`, runs the tests in
`check()`, and keeps `$srcdir` out of the binary with `--remap-path-prefix`.

**Differences from 1.0.x** (the Python version, tag `v1.0.10`). Text is kerned and laid out on
fractional advances, so it runs a few pixels narrower. The copy tooltip stays inside the window.
The dialog honours the Wayland output scale. No window icon is set; on Wayland the desktop file
for app id `macabout` provides it. Mistyped options get clap's messages, and options can no
longer be abbreviated (`--mo` for `--mock`).
