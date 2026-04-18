from __future__ import annotations

import platform
import tkinter as tk
from pathlib import Path
from tkinter import font as tkfont

BG_COLOR = "#ECECEC"
FG_COLOR = "#1A1A1A"
SUBTLE_FG = "#4D4D4D"

WINDOW_W = 620
WINDOW_H = 370
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
            for ext in ("png", "xpm"):
                p = Path(f"{d}/{name}.{ext}")
                if p.exists():
                    return str(p)
    return None


def _find_bundled_icon(distro_id: str) -> str | None:
    icons_dir = Path(__file__).parent / "data" / "icons"
    p = icons_dir / f"{distro_id}.png"
    return str(p) if p.exists() else None


def _make_icon_widget(parent: tk.Widget, distro_id: str, distro_name: str, logo_id: str = "") -> tk.Widget:
    size = ICON_SIZE
    path = _find_bundled_icon(distro_id) or _find_system_icon(distro_id, logo_id)

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


def show_dialog(display: dict) -> None:
    root = tk.Tk()
    root.title("About This Computer")
    root.configure(bg=BG_COLOR)
    root.resizable(False, False)

    family = _pick_font(root)

    main = tk.Frame(root, bg=BG_COLOR)
    main.pack(fill="both", expand=True)

    # Left column — fixed width, icon centered
    left = tk.Frame(main, bg=BG_COLOR, width=LEFT_W)
    left.pack(side="left", fill="y")
    left.pack_propagate(False)

    icon_w = _make_icon_widget(left, display["distro_id"], display["os_name"], display.get("logo_id", ""))
    icon_w.place(relx=0.5, rely=0.5, anchor="center")

    # Right column — title, version, spec rows
    right = tk.Frame(main, bg=BG_COLOR)
    right.pack(side="left", fill="both", expand=True, padx=(0, 32))

    tk.Label(
        right,
        text=display["os_name"],
        font=(family, 24),
        bg=BG_COLOR,
        fg=FG_COLOR,
        anchor="w",
        justify="left",
    ).pack(anchor="w", pady=(52, 3))

    version_text = f"Version {display['os_version']}" if display["os_version"] else ""
    tk.Label(
        right,
        text=version_text,
        font=(family, 12),
        bg=BG_COLOR,
        fg=SUBTLE_FG,
        anchor="w",
    ).pack(anchor="w", pady=(0, 20))

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
        ).grid(row=i, column=0, sticky="e", padx=(0, 8), pady=2)
        tk.Label(
            grid,
            text=value,
            font=(family, 11),
            bg=BG_COLOR,
            fg=FG_COLOR,
            anchor="w",
        ).grid(row=i, column=1, sticky="w", pady=2)

    root.update_idletasks()
    x = (root.winfo_screenwidth() - WINDOW_W) // 2
    y = (root.winfo_screenheight() - WINDOW_H) // 2
    root.geometry(f"{WINDOW_W}x{WINDOW_H}+{x}+{y}")

    root.mainloop()
