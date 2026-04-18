from __future__ import annotations

import functools
import re
import subprocess

_MACOS_CODENAMES: dict[str, str] = {
    "15": "Sequoia",
    "14": "Sonoma",
    "13": "Ventura",
    "12": "Monterey",
    "11": "Big Sur",
    "10.15": "Catalina",
    "10.14": "Mojave",
    "10.13": "High Sierra",
}


def _run(cmd: list[str]) -> str | None:
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=10)
    except (FileNotFoundError, subprocess.TimeoutExpired, PermissionError, OSError):
        return None
    if r.returncode != 0:
        return None
    return r.stdout


@functools.lru_cache(maxsize=1)
def _system_profiler() -> str:
    # One batched call; subsequent getters read from cache at no cost.
    return _run(["system_profiler", "SPHardwareDataType", "SPMemoryDataType", "SPDisplaysDataType"]) or ""


def get_os_info() -> dict | None:
    name_out = _run(["sw_vers", "-productName"])
    version_out = _run(["sw_vers", "-productVersion"])
    if not version_out:
        return None
    name = (name_out or "macOS").strip()
    version = version_out.strip()
    major = version.split(".")[0]
    key = f"10.{version.split('.')[1]}" if major == "10" else major
    codename = _MACOS_CODENAMES.get(key, "")
    if codename:
        name = f"{name} {codename}"
    return {"name": name, "version": version, "id": "macos"}


def get_processor_raw() -> str | None:
    # sysctl is fast and works on both Intel and Apple Silicon
    out = _run(["sysctl", "-n", "machdep.cpu.brand_string"])
    if out and out.strip():
        return out.strip()
    # Apple Silicon fallback: "Chip:" line in system_profiler SPHardwareDataType
    sp = _system_profiler()
    m = re.search(r"Chip:\s*(.+)", sp)
    if m:
        return m.group(1).strip()
    m2 = re.search(r"Processor Name:\s*(.+)", sp)
    return m2.group(1).strip() if m2 else None


def get_memory_info() -> dict | None:
    size_out = _run(["sysctl", "-n", "hw.memsize"])
    if not size_out:
        return None
    try:
        total_mb = int(size_out.strip()) // (1024 * 1024)
    except ValueError:
        return None

    sp = _system_profiler()
    speed_mhz = None
    mem_type = None

    type_m = re.search(r"^\s*Type:\s+(\S+)", sp, re.MULTILINE)
    speed_m = re.search(r"^\s*Speed:\s+(\d+)\s*MHz", sp, re.MULTILINE)

    if type_m:
        t = type_m.group(1)
        if t.lower() not in ("unknown", "none", "empty"):
            mem_type = t
    if speed_m:
        speed_mhz = int(speed_m.group(1))

    return {"total_mb": total_mb, "speed_mhz": speed_mhz, "type": mem_type}


def get_graphics_info() -> dict | None:
    sp = _system_profiler()
    name_m = re.search(r"Chipset Model:\s*(.+)", sp)
    if not name_m:
        return None
    name = name_m.group(1).strip()

    # Scope VRAM search to this GPU's block (before the next "Chipset Model:")
    block_start = name_m.start()
    next_chip = re.search(r"Chipset Model:", sp[name_m.end():])
    block = sp[block_start: name_m.end() + next_chip.start()] if next_chip else sp[block_start:]

    vram_m = re.search(r"VRAM[^:]*:\s*(\d+)\s*(MB|GB)", block)
    vram_mb = None
    if vram_m:
        amount = int(vram_m.group(1))
        vram_mb = amount * 1024 if vram_m.group(2) == "GB" else amount

    return {"raw": name, "pci_id": None, "vram_mb": vram_mb}


def get_serial_number() -> str | None:
    out = _run(["ioreg", "-c", "IOPlatformExpertDevice", "-d", "2"])
    if not out:
        return None
    m = re.search(r'"IOPlatformSerialNumber"\s*=\s*"([^"]+)"', out)
    return m.group(1) if m else None
