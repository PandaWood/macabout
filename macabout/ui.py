from __future__ import annotations

import platform
import tkinter as tk
from pathlib import Path
from tkinter import font as tkfont

_LIGHT = {"bg": "#ECECEC", "fg": "#1A1A1A", "subtle": "#4D4D4D", "ver": "#AAAAAA"}
_DARK  = {"bg": "#242424", "fg": "#F2F2F2", "subtle": "#AAAAAA", "ver": "#666666"}

BG_COLOR = _LIGHT["bg"]
FG_COLOR = _LIGHT["fg"]
SUBTLE_FG = _LIGHT["subtle"]


def _detect_dark_mode() -> bool:
    import subprocess, os
    if platform.system() == "Linux":
        # When running under sudo, gsettings/dconf need the real user's D-Bus session
        env = os.environ.copy()
        real_home = Path.home()
        sudo_user = os.environ.get("SUDO_USER")
        if sudo_user:
            try:
                import pwd
                uid = pwd.getpwnam(sudo_user).pw_uid
                bus = f"/run/user/{uid}/bus"
                if Path(bus).exists():
                    env["DBUS_SESSION_BUS_ADDRESS"] = f"unix:path={bus}"
                real_home = Path(f"/home/{sudo_user}")
            except Exception:
                pass

        def _run(cmd: list[str]) -> str:
            try:
                r = subprocess.run(cmd, capture_output=True, text=True, timeout=2, env=env)
                return r.stdout if r.returncode == 0 else ""
            except Exception:
                return ""

        # 1. gsettings (GNOME)
        if "dark" in _run(["gsettings", "get", "org.gnome.desktop.interface", "color-scheme"]).lower():
            return True
        if "dark" in _run(["gsettings", "get", "org.gnome.desktop.interface", "gtk-theme"]).lower():
            return True

        # 2. GTK_THEME env var (KDE, Xfce, etc.)
        if "dark" in os.environ.get("GTK_THEME", "").lower():
            return True

        # 3. GTK settings files (use real user's home, not /root)
        from configparser import ConfigParser
        for settings_path in (
            real_home / ".config" / "gtk-4.0" / "settings.ini",
            real_home / ".config" / "gtk-3.0" / "settings.ini",
        ):
            if settings_path.exists():
                cfg = ConfigParser()
                cfg.read(settings_path)
                val = cfg.get("Settings", "gtk-application-prefer-dark-theme", fallback="0")
                if val.strip() in ("1", "true", "True"):
                    return True

        # 4. dconf direct read (via CLI)
        if "dark" in _run(["dconf", "read", "/org/gnome/desktop/interface/color-scheme"]).lower():
            return True

        # 5. dconf binary database — works under sudo with no D-Bus required
        dconf_db = real_home / ".config" / "dconf" / "user"
        if dconf_db.exists():
            try:
                if b"prefer-dark" in dconf_db.read_bytes():
                    return True
            except Exception:
                pass

        # 6. xdg-desktop-portal (DE-agnostic, Wayland + X11)
        out = _run(["gdbus", "call", "--session",
                    "--dest", "org.freedesktop.portal.Desktop",
                    "--object-path", "/org/freedesktop/portal/desktop",
                    "--method", "org.freedesktop.portal.Settings.ReadOne",
                    "org.freedesktop.appearance", "color-scheme"])
        # Returns "(<uint32 1>,)" for dark, "(<uint32 0>,)" for light/no-pref
        if "<uint32 1>" in out:
            return True

    elif platform.system() == "Darwin":
        try:
            r = subprocess.run(
                ["defaults", "read", "-g", "AppleInterfaceStyle"],
                capture_output=True, text=True, timeout=2,
            )
            if r.returncode == 0 and "dark" in r.stdout.lower():
                return True
        except Exception:
            pass
    return False

WINDOW_W = 680
WINDOW_H = 330
ICON_SIZE = 180
LEFT_W = 240

FONT_CANDIDATES = (
    "SF Pro Text",
    "SF Pro Display",
    ".AppleSystemUIFont",
    "Helvetica Neue",
    "Inter",
    "Segoe UI",
    "Ubuntu",
    "DejaVu Sans",
)

# Brand colors for the canvas fallback icon
_BRAND_COLORS: dict[str, str] = {
    "zorin":      "#15A6F0",
    "ubuntu":     "#E95420",
    "debian":     "#A80030",
    "fedora":     "#294172",
    "linuxmint":  "#87C04D",
    "pop":        "#48B9C7",
    "elementary": "#64BAFF",
    "arch":       "#1793D1",
    "manjaro":    "#35BF5C",
    "opensuse":   "#73BA25",
    "kali":       "#268BEE",
    "raspbian":   "#C51A4A",
    "centos":     "#932279",
    "rhel":       "#EE0000",
    "macos":      "#9966CC",
    "linux":      "#F6C519",
}


def _pick_font(root: tk.Tk) -> str:
    available = set(tkfont.families(root))
    for name in FONT_CANDIDATES:
        if name in available:
            return name
    return tkfont.nametofont("TkDefaultFont").actual("family")


def _brand_color(distro_id: str) -> str:
    return _BRAND_COLORS.get(distro_id.lower(), "#607080")


def _find_system_icon(distro_id: str, logo_id: str = "") -> str | None:
    if platform.system() != "Linux":
        return None
    # logo_id from LOGO= in /etc/os-release is the most authoritative name
    candidates = [c for c in [
        logo_id,
        "distributor-logo",
        f"distributor-logo-{distro_id}",
        f"{distro_id}-logo",
        distro_id,
    ] if c]
    dirs = [
        "/usr/share/icons/hicolor/256x256/apps",
        "/usr/share/icons/hicolor/128x128/apps",
        "/usr/share/icons/hicolor/64x64/apps",
        "/usr/share/pixmaps",
    ]
    for name in candidates:
        for d in dirs:
            for ext in ("png", "svg", "xpm"):
                p = Path(f"{d}/{name}.{ext}")
                if p.exists():
                    return str(p)
    return None


def _find_bundled_icon(distro_id: str) -> str | None:
    icons_dir = Path(__file__).parent / "data" / "icons"
    p = icons_dir / f"{distro_id}.png"
    return str(p) if p.exists() else None


def _svg_to_photoimage(path: str, size: int) -> tk.PhotoImage | None:
    import shutil, tempfile
    render_size = size * 2
    tmp = tempfile.NamedTemporaryFile(suffix=".png", delete=False)
    tmp.close()
    try:
        for cmd in (
            ["rsvg-convert", "-w", str(render_size), "-h", str(render_size), path, "-o", tmp.name],
            ["convert", "-background", "none", "-resize", f"{render_size}x{render_size}", path, tmp.name],
            ["inkscape", "--export-type=png", f"--export-filename={tmp.name}",
             f"--export-width={render_size}", f"--export-height={render_size}", path],
        ):
            if not shutil.which(cmd[0]):
                continue
            import subprocess
            r = subprocess.run(cmd, capture_output=True, timeout=5)
            if r.returncode != 0:
                continue
            from PIL import Image, ImageTk
            img = Image.open(tmp.name).convert("RGBA").resize((size, size), Image.LANCZOS)
            return ImageTk.PhotoImage(img)
    except Exception:
        pass
    finally:
        Path(tmp.name).unlink(missing_ok=True)
    return None


def _make_icon_widget(parent: tk.Widget, distro_id: str, distro_name: str, logo_id: str = "") -> tk.Widget:
    size = ICON_SIZE
    path = _find_bundled_icon(distro_id) or _find_system_icon(distro_id, logo_id)

    if path and path.endswith(".svg"):
        photo = _svg_to_photoimage(path, size)
        if photo:
            lbl = tk.Label(parent, image=photo, bg=BG_COLOR, bd=0)
            lbl._photo = photo
            return lbl
        path = None  # SVG conversion failed — fall through to canvas

    if path:
        # Try PIL first — clean resize at any ratio
        try:
            from PIL import Image, ImageTk
            img = Image.open(path).convert("RGBA").resize((size, size), Image.LANCZOS)
            photo = ImageTk.PhotoImage(img)
            lbl = tk.Label(parent, image=photo, bg=BG_COLOR, bd=0)
            lbl._photo = photo  # prevent GC
            return lbl
        except Exception:
            pass
        # PIL unavailable — native PhotoImage (PNG only, integer subsample)
        if path.endswith(".png"):
            try:
                photo = tk.PhotoImage(file=path)
                w, h = photo.width(), photo.height()
                if max(w, h) > size:
                    factor = max(w, h) // size + 1
                    photo = photo.subsample(factor, factor)
                lbl = tk.Label(parent, image=photo, bg=BG_COLOR, bd=0)
                lbl._photo = photo
                return lbl
            except Exception:
                pass

    # Canvas fallback: brand-colored circle with distro initial
    color = _brand_color(distro_id)
    c = tk.Canvas(parent, width=size, height=size, bg=BG_COLOR, bd=0, highlightthickness=0)
    pad = size // 18
    c.create_oval(pad, pad, size - pad, size - pad, fill=color, outline="")
    if distro_name:
        c.create_text(
            size // 2, size // 2,
            text=distro_name[0].upper(),
            font=("Helvetica", size // 3, "bold"),
            fill="white",
        )
    return c


def _build_copy_text(display: dict) -> str:
    header = [display["os_name"]]
    if display["os_version"]:
        header.append(f"Version {display['os_version']}")
    rows = (
        ("Processor", display["processor"]),
        ("Memory", display["memory"]),
        ("Graphics", display["graphics"]),
        ("Serial Number", display["serial"]),
    )
    label_w = max(len(lbl) for lbl, _ in rows)
    body = [f"{lbl.rjust(label_w)}  {val}" for lbl, val in rows]
    return "\n".join(header + [""] + body)


def show_dialog(display: dict) -> None:
    global BG_COLOR, FG_COLOR, SUBTLE_FG
    palette = _DARK if _detect_dark_mode() else _LIGHT
    BG_COLOR, FG_COLOR, SUBTLE_FG = palette["bg"], palette["fg"], palette["subtle"]
    ver_color = palette["ver"]

    root = tk.Tk()
    root.title("About This Computer")
    root.configure(bg=BG_COLOR)
    root.resizable(False, False)

    family = _pick_font(root)

    main = tk.Frame(root, bg=BG_COLOR)
    main.pack(fill="both", expand=True, padx=16, pady=(16, 82))

    # Left column — fixed width, icon centered
    left = tk.Frame(main, bg=BG_COLOR, width=LEFT_W)
    left.pack(side="left", fill="y")
    left.pack_propagate(False)

    icon_w = _make_icon_widget(left, display["distro_id"], display["os_name"], display.get("logo_id", ""))
    icon_w.place(relx=0.5, rely=0.5, anchor="center")

    # Right column — title, version, spec rows
    right = tk.Frame(main, bg=BG_COLOR)
    right.pack(side="left", fill="both", expand=True, padx=(0, 48))

    tk.Label(
        right,
        text=display["os_name"],
        font=(family, 24),
        bg=BG_COLOR,
        fg=FG_COLOR,
        anchor="w",
        justify="left",
    ).pack(anchor="w", pady=(36, 4))

    version_text = f"Version {display['os_version']}" if display["os_version"] else ""
    tk.Label(
        right,
        text=version_text,
        font=(family, 12),
        bg=BG_COLOR,
        fg=SUBTLE_FG,
        anchor="w",
    ).pack(anchor="w", pady=(0, 18))

    rows = (
        ("Processor", display["processor"]),
        ("Memory", display["memory"]),
        ("Graphics", display["graphics"]),
        ("Serial Number", display["serial"]),
    )

    grid = tk.Frame(right, bg=BG_COLOR)
    grid.pack(anchor="w")

    for i, (label, value) in enumerate(rows):
        tk.Label(
            grid,
            text=label,
            font=(family, 11, "bold"),
            bg=BG_COLOR,
            fg=FG_COLOR,
            anchor="e",
            width=13,
        ).grid(row=i, column=0, sticky="e", padx=(0, 8), pady=4)
        tk.Label(
            grid,
            text=value,
            font=(family, 11),
            bg=BG_COLOR,
            fg=FG_COLOR,
            anchor="w",
            wraplength=420,
            justify="left",
        ).grid(row=i, column=1, sticky="w", pady=4)

    from . import __version__
    tk.Label(
        root,
        text=f"macabout v{__version__}",
        font=(family, 8),
        bg=BG_COLOR,
        fg=ver_color,
    ).place(relx=1.0, rely=1.0, anchor="se", x=-8, y=-6)

    copy_text = _build_copy_text(display)
    btn_size = 26
    copy_btn = tk.Canvas(
        root, width=btn_size, height=btn_size,
        bg=BG_COLOR, bd=0, highlightthickness=0, cursor="hand2",
    )
    back = copy_btn.create_rectangle(8, 4, 21, 17, outline=SUBTLE_FG, width=2)
    front = copy_btn.create_rectangle(4, 8, 17, 21, outline=SUBTLE_FG, fill=BG_COLOR, width=2)

    def _do_copy(_e=None):
        root.clipboard_clear()
        root.clipboard_append(copy_text)
        root.update()

    tooltip = {"win": None}

    def _on_enter(_e=None):
        copy_btn.itemconfig(back, outline=FG_COLOR)
        copy_btn.itemconfig(front, outline=FG_COLOR)
        if tooltip["win"]:
            return
        tw = tk.Toplevel(root)
        tw.wm_overrideredirect(True)
        tk.Label(
            tw, text="Copy to clipboard",
            font=(family, 9), bg=FG_COLOR, fg=BG_COLOR,
            padx=6, pady=2, bd=0,
        ).pack()
        tw.update_idletasks()
        x = copy_btn.winfo_rootx() + (btn_size - tw.winfo_width()) // 2
        y = copy_btn.winfo_rooty() - tw.winfo_height() - 4
        tw.wm_geometry(f"+{x}+{y}")
        tooltip["win"] = tw

    def _on_leave(_e=None):
        copy_btn.itemconfig(back, outline=SUBTLE_FG)
        copy_btn.itemconfig(front, outline=SUBTLE_FG)
        if tooltip["win"]:
            tooltip["win"].destroy()
            tooltip["win"] = None

    copy_btn.bind("<Button-1>", _do_copy)
    copy_btn.bind("<Enter>", _on_enter)
    copy_btn.bind("<Leave>", _on_leave)
    copy_btn.place(relx=1.0, rely=1.0, anchor="se", x=-12, y=-44)

    root.update_idletasks()
    w = root.winfo_reqwidth()
    h = root.winfo_reqheight()
    x = (root.winfo_screenwidth() - w) // 2
    y = (root.winfo_screenheight() - h) // 2
    root.geometry(f"+{x}+{y}")

    root.mainloop()
