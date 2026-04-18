from __future__ import annotations

import re
import subprocess
from pathlib import Path


def _run(cmd: list[str]) -> str | None:
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=3)
    except (FileNotFoundError, subprocess.TimeoutExpired, PermissionError, OSError):
        return None
    if r.returncode != 0:
        return None
    return r.stdout


def _parse_os_release(content: str) -> dict:
    out: dict = {}
    for line in content.splitlines():
        line = line.strip()
        if not line or line.startswith("#") or "=" not in line:
            continue
        k, v = line.split("=", 1)
        v = v.strip()
        if len(v) >= 2 and v[0] == '"' and v[-1] == '"':
            v = v[1:-1]
        out[k] = v
    return out


def get_os_info() -> dict | None:
    try:
        parsed = _parse_os_release(Path("/etc/os-release").read_text())
    except OSError:
        return None
    version = parsed.get("VERSION", "")
    version = re.sub(r"\s*\(.*\)\s*", "", version).strip()
    if not version:
        version = parsed.get("VERSION_ID", "")
    return {
        "name": parsed.get("NAME", "Linux"),
        "version": version,
        "id": parsed.get("ID", "linux").lower(),
        "logo": parsed.get("LOGO", ""),
    }


def get_processor_raw() -> str | None:
    try:
        content = Path("/proc/cpuinfo").read_text()
    except OSError:
        return None
    for line in content.splitlines():
        if line.startswith("model name"):
            return line.split(":", 1)[1].strip()
    return None


def _parse_meminfo_total_mb(content: str) -> int | None:
    for line in content.splitlines():
        if line.startswith("MemTotal:"):
            m = re.search(r"(\d+)\s*kB", line)
            if m:
                return int(m.group(1)) // 1024
    return None


def _parse_dmidecode_memory(output: str) -> dict:
    for block in output.split("\n\n"):
        if "Memory Device" not in block:
            continue
        size_m = re.search(r"^\s*Size:\s*(.+)$", block, re.MULTILINE)
        if not size_m:
            continue
        size_val = size_m.group(1).strip()
        if "No Module" in size_val or size_val == "0 MB" or size_val.lower() == "unknown":
            continue
        type_m = re.search(r"^\s*Type:\s+(\S+)", block, re.MULTILINE)
        speed_m = re.search(r"^\s*Speed:\s+(\d+)\s*M(?:T/s|Hz)", block, re.MULTILINE)
        type_val = type_m.group(1) if type_m else None
        if type_val in ("Unknown", "None"):
            type_val = None
        return {
            "type": type_val,
            "speed_mhz": int(speed_m.group(1)) if speed_m else None,
        }
    return {"type": None, "speed_mhz": None}


def get_memory_info() -> dict | None:
    try:
        content = Path("/proc/meminfo").read_text()
    except OSError:
        return None
    total_mb = _parse_meminfo_total_mb(content)
    if total_mb is None:
        return None
    info = {"total_mb": total_mb, "speed_mhz": None, "type": None}
    dmi = _run(["dmidecode", "-t", "memory"])
    if dmi:
        info.update(_parse_dmidecode_memory(dmi))
    return info


def _parse_lspci_vga(output: str) -> dict | None:
    for line in output.splitlines():
        if not re.search(r"(VGA compatible controller|3D controller|Display controller)", line):
            continue
        ids = re.findall(r"\[([0-9a-f]{4}:[0-9a-f]{4})\]", line)
        if not ids:
            continue
        pci_id = ids[-1].lower()
        parts = line.split("]: ", 1)
        name = ""
        if len(parts) == 2:
            name = re.sub(r"\s*\[[0-9a-f]{4}:[0-9a-f]{4}\].*$", "", parts[1]).strip()
        return {"raw": name, "pci_id": pci_id}
    return None


def get_graphics_info() -> dict | None:
    out = _run(["lspci", "-nn"])
    if not out:
        return None
    return _parse_lspci_vga(out)


def get_serial_number() -> str | None:
    out = _run(["dmidecode", "-s", "system-serial-number"])
    if not out:
        return None
    lines = [ln.strip() for ln in out.strip().splitlines() if ln.strip() and not ln.startswith("#")]
    if not lines:
        return None
    value = lines[-1]
    placeholder = {"", "none", "not specified", "to be filled by o.e.m.", "system serial number", "default string"}
    if value.lower() in placeholder:
        return None
    return value
