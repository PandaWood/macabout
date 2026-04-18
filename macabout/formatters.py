from __future__ import annotations

import json
import re
from pathlib import Path

from .hwinfo import SystemInfo

_LOOKUP_PATH = Path(__file__).parent / "data" / "gpu_lookup.json"


def _load_gpu_lookup() -> dict:
    try:
        return json.loads(_LOOKUP_PATH.read_text())
    except (OSError, json.JSONDecodeError):
        return {}


_GPU_LOOKUP = _load_gpu_lookup()


def format_processor(raw: str, cores: int | None = None) -> str:
    if not raw:
        return "N/A"
    s = raw.replace("(R)", "").replace("(TM)", "").replace("(tm)", "")

    if re.match(r"Apple\s+M\d", s):
        return s.strip()

    ghz_m = re.search(r"@\s*(\d+(?:\.\d+)?)\s*GHz", s, re.IGNORECASE)
    ghz = f"{round(float(ghz_m.group(1)), 1)}" if ghz_m else None

    core_str = f"{cores}-Core " if cores else ""

    core_m = re.search(r"Core\s+(i[3579])", s)
    if core_m:
        family = f"Intel Core {core_m.group(1)}"
    elif "Xeon" in s:
        xeon_m = re.search(r"Xeon\s+(\w+)", s)
        suffix = f" {xeon_m.group(1)}" if xeon_m and not xeon_m.group(1).startswith("CPU") else ""
        family = f"Intel Xeon{suffix}"
    elif "Pentium" in s:
        family = "Intel Pentium"
    elif "Celeron" in s:
        family = "Intel Celeron"
    elif "Atom" in s:
        family = "Intel Atom"
    elif "Ryzen" in s:
        rm = re.search(r"Ryzen\s+(\d+)", s)
        family = f"AMD Ryzen {rm.group(1)}" if rm else "AMD Ryzen"
    elif "EPYC" in s:
        family = "AMD EPYC"
    else:
        family = re.sub(r"\s+CPU\s*@.*$", "", s).strip()
        family = re.sub(r"\s+\d+(\.\d+)?\s*GHz$", "", family).strip()

    if ghz:
        return f"{ghz} GHz {core_str}{family}".strip()
    return f"{core_str}{family}".strip()


_STANDARD_GB = [1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256, 384, 512]


def _snap_gb(total_mb: int) -> int:
    gb = total_mb / 1024
    nearest = min(_STANDARD_GB, key=lambda s: abs(s - gb))
    # Only snap if within 5% of a standard size, otherwise round normally
    return nearest if abs(nearest - gb) / nearest <= 0.05 else round(gb)


def format_memory(total_mb: int, speed_mhz: int | None, type_: str | None) -> str:
    if not total_mb:
        return "N/A"
    gb = _snap_gb(total_mb)
    parts = [f"{gb} GB"]
    if speed_mhz:
        parts.append(f"{speed_mhz} MHz")
    if type_:
        parts.append(type_)
    return " ".join(parts)


def format_graphics(gpu_raw: str, pci_id: str | None, vram_mb: int | None = None) -> str:
    if pci_id:
        entry = _GPU_LOOKUP.get(pci_id.lower())
        if entry:
            name = entry["name"]
            vram_mb = entry.get("vram_mb")
            if vram_mb:
                vram_gb = round(vram_mb / 1024)
                return f"{name} {vram_gb} GB"
            return name
    if not gpu_raw:
        return "N/A"
    # lspci names end with a marketing name in [brackets] — prefer that
    bracket_m = re.search(r'\[([^\[\]]+)\]$', gpu_raw.strip())
    if bracket_m:
        name = bracket_m.group(1)
        # Re-add Intel prefix for integrated GPUs (e.g. "UHD Graphics")
        if "Intel" in gpu_raw and not name.startswith("Intel"):
            name = f"Intel {name}"
    else:
        name = re.sub(r"^Mesa\s+", "", gpu_raw)
        # Strip vendor prefixes left by lspci (e.g. "Advanced Micro Devices, Inc. [AMD/ATI] ")
        name = re.sub(r"^[\w\s,\.]+(?:,\s*Inc\.?)?\s*(?:\[[^\]]+\]\s*)?", "", name).strip() or name
        name = re.sub(r"\s*\([^)]*\)\s*", " ", name)
        name = re.sub(r"\s+", " ", name).strip()
    if vram_mb:
        return f"{name} {round(vram_mb / 1024)} GB"
    return name


def format_all(info: SystemInfo) -> dict:
    return {
        "os_name": info.os_name or "Linux",
        "os_version": info.os_version or "",
        "distro_id": info.distro_id,
        "logo_id": info.logo_id,
        "processor": format_processor(info.cpu_raw, info.cpu_cores),
        "memory": format_memory(info.mem_total_mb, info.mem_speed_mhz, info.mem_type),
        "graphics": format_graphics(info.gpu_raw, info.gpu_pci_id, info.gpu_vram_mb),
        "serial": info.serial or "N/A",
    }
