from __future__ import annotations

import os
import re
import subprocess
from pathlib import Path


def _run(cmd: list[str], env: dict | None = None) -> str | None:
    try:
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=3, env=env)
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
    # ARM kernels list part numbers, not a model name; lscpu turns them into names.
    # LC_ALL=C keeps its labels in English.
    out = _run(["lscpu"], env={**os.environ, "LC_ALL": "C"})
    return _parse_lscpu_model(out) if out else None


def _parse_lscpu_model(output: str) -> str | None:
    """"ARM Cortex-A72", or with two kinds of core (big.LITTLE) "ARM Cortex-A55 + Cortex-A76"."""
    vendor = None
    models: list[str] = []
    for line in output.splitlines():
        key, sep, value = line.strip().partition(":")
        if not sep:
            continue
        value = value.strip()
        if key == "Vendor ID" and vendor is None:
            vendor = value
        elif key == "Model name" and value not in ("", "-") and value not in models:
            models.append(value)
    if not models:
        return None
    name = " + ".join(models)
    if vendor and not name.startswith(vendor):
        return f"{vendor} {name}"
    return name


def get_cpu_core_count() -> int | None:
    try:
        content = Path("/proc/cpuinfo").read_text()
    except OSError:
        return None
    # Count unique (physical id, core id) pairs for physical core count
    cores: set[tuple[str, str]] = set()
    phys_id = "0"
    core_id = None
    for line in content.splitlines():
        if line.startswith("physical id"):
            phys_id = line.split(":", 1)[1].strip()
        elif line.startswith("core id"):
            core_id = line.split(":", 1)[1].strip()
            cores.add((phys_id, core_id))
    if cores:
        return len(cores)
    # Fallback: count processor entries (logical CPUs)
    count = sum(1 for l in content.splitlines() if l.startswith("processor"))
    return count or None


def _parse_meminfo_total_mb(content: str) -> int | None:
    for line in content.splitlines():
        if line.startswith("MemTotal:"):
            m = re.search(r"(\d+)\s*kB", line)
            if m:
                return int(m.group(1)) // 1024
    return None


def _parse_udev_memory(output: str) -> dict:
    """Type and speed of the first populated slot, from the MEMORY_DEVICE_<n>_* properties udev
    reads from the firmware's memory table at boot (systemd 248 and later)."""
    slots: dict[int, dict[str, str]] = {}
    for line in output.splitlines():
        key, _, value = line.partition("=")
        m = re.fullmatch(r"MEMORY_DEVICE_(\d+)_(\w+)", key)
        if m:
            slots.setdefault(int(m.group(1)), {})[m.group(2)] = value.strip()
    for _, slot in sorted(slots.items()):
        size = slot.get("SIZE", "")
        if not size.isdigit() or int(size) == 0:
            continue
        type_val = slot.get("TYPE")
        if type_val in ("", "Unknown", "None"):
            type_val = None
        speed = slot.get("SPEED_MTS", "")
        return {
            "type": type_val,
            "speed_mhz": int(speed) if speed.isdigit() else None,
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
    # Readable by any user, unlike dmidecode, which needs root.
    out = _run(["udevadm", "info", "--query=property", "--path=/devices/virtual/dmi/id"])
    if out:
        info.update(_parse_udev_memory(out))
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
        # The PCI address, "01:00.0"; lspci adds the domain only when there are several.
        return {"raw": name, "pci_id": pci_id, "slot": line.split()[0]}
    return None


def _sysfs_vram_mb(slot: str) -> int | None:
    """The card's own memory, from the sysfs node of the PCI device lspci named (amdgpu)."""
    address = slot if slot.count(":") == 2 else f"0000:{slot}"
    try:
        val = int(Path(f"/sys/bus/pci/devices/{address}/mem_info_vram_total").read_text().strip())
    except (OSError, ValueError):
        return None
    return val // (1024 * 1024) if val > 0 else None


# Graphics built into the chip (ARM boards) is not on PCI: named by its DRM driver.
_DRM_GPUS = {
    "v3d": "Broadcom VideoCore",
    "vc4": "Broadcom VideoCore",
    "panfrost": "Arm Mali",
    "panthor": "Arm Mali",
    "lima": "Arm Mali",
    "msm": "Qualcomm Adreno",
    "etnaviv": "Vivante GPU",
    "asahi": "Apple GPU",
    "tegra": "NVIDIA Tegra",
}


def _drm_gpu() -> dict | None:
    for card in sorted(Path("/sys/class/drm").glob("card*")):
        if not card.name[4:].isdigit():
            continue
        try:
            driver = os.path.basename(os.readlink(card / "device" / "driver"))
        except OSError:
            continue
        if driver in _DRM_GPUS:
            return {"raw": _DRM_GPUS[driver], "pci_id": None, "vram_mb": None}
    return None


def get_graphics_info() -> dict | None:
    out = _run(["lspci", "-nn"])
    info = _parse_lspci_vga(out) if out else None
    if info is None:
        return _drm_gpu()
    info["vram_mb"] = _sysfs_vram_mb(info["slot"])
    return info


_DMI_PLACEHOLDERS = frozenset({
    "", "none", "not specified", "to be filled by o.e.m.", "default string",
})


def _dmi_string(name: str, extra_placeholder: str = "") -> str | None:
    """A value from the firmware's DMI table, as the kernel shows it to any user."""
    try:
        value = Path(f"/sys/class/dmi/id/{name}").read_text().strip()
    except OSError:
        return None
    placeholders = _DMI_PLACEHOLDERS | ({extra_placeholder.lower()} if extra_placeholder else set())
    if value.lower() in placeholders:
        return None
    return value


def _device_tree_model() -> str | None:
    """The board's name on ARM, which has no DMI table: "Raspberry Pi 4 Model B Rev 1.4"."""
    try:
        model = Path("/proc/device-tree/model").read_text().rstrip("\0").strip()
    except OSError:
        return None
    return model or None


def get_machine_model() -> str | None:
    return _dmi_string("product_name", "system product name") or _device_tree_model()
