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


def format_processor(raw: str) -> str:
    if not raw:
        return "N/A"
    s = raw.replace("(R)", "").replace("(TM)", "").replace("(tm)", "")

    if re.match(r"Apple\s+M\d", s):
        return s.strip()

    ghz_m = re.search(r"@\s*(\d+(?:\.\d+)?)\s*GHz", s, re.IGNORECASE)
    ghz = f"{round(float(ghz_m.group(1)), 1)}" if ghz_m else None

    core_m = re.search(r"Core\s+(i[3579])", s)
    if core_m:
        family = f"Intel Core {core_m.group(1)}"
    elif "Xeon" in s:
        family = "Intel Xeon"
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

    return f"{ghz} GHz {family}" if ghz else family


def format_memory(total_mb: int, speed_mhz: int | None, type_: str | None) -> str:
    if not total_mb:
        return "N/A"
    gb = round(total_mb / 1024)
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
            vram = entry.get("vram_mb")
            return f"{name} {vram} MB" if vram else name
    if not gpu_raw:
        return "N/A"
    name = re.sub(r"^Mesa\s+", "", gpu_raw)
    name = re.sub(r"\s*\([^)]*\)\s*", " ", name)
    name = re.sub(r"\s+", " ", name).strip()
    if vram_mb:
        return f"{name} {vram_mb} MB"
    return name


def format_all(info: SystemInfo) -> dict:
    return {
        "os_name": info.os_name or "Linux",
        "os_version": info.os_version or "",
        "distro_id": info.distro_id,
        "logo_id": info.logo_id,
        "processor": format_processor(info.cpu_raw),
        "memory": format_memory(info.mem_total_mb, info.mem_speed_mhz, info.mem_type),
        "graphics": format_graphics(info.gpu_raw, info.gpu_pci_id, info.gpu_vram_mb),
        "serial": info.serial or "N/A",
    }
