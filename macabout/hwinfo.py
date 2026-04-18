from __future__ import annotations

import platform
from dataclasses import dataclass

from . import linux_collectors, mac_collectors, mock_data


@dataclass
class SystemInfo:
    os_name: str
    os_version: str
    distro_id: str
    logo_id: str
    cpu_raw: str
    mem_total_mb: int
    mem_speed_mhz: int | None
    mem_type: str | None
    gpu_raw: str
    gpu_pci_id: str | None
    gpu_vram_mb: int | None
    serial: str | None


def collect_system_info(force_mock: bool = False) -> SystemInfo:
    sys_platform = platform.system()
    if force_mock:
        backend = mock_data
    elif sys_platform == "Linux":
        backend = linux_collectors
    elif sys_platform == "Darwin":
        backend = mac_collectors
    else:
        backend = mock_data

    os_info = backend.get_os_info() or {}
    mem = backend.get_memory_info() or {}
    gpu = backend.get_graphics_info() or {}

    return SystemInfo(
        os_name=os_info.get("name") or "Linux",
        os_version=os_info.get("version") or "",
        distro_id=os_info.get("id") or "linux",
        logo_id=os_info.get("logo") or "",
        cpu_raw=backend.get_processor_raw() or "",
        mem_total_mb=mem.get("total_mb") or 0,
        mem_speed_mhz=mem.get("speed_mhz"),
        mem_type=mem.get("type"),
        gpu_raw=gpu.get("raw") or "",
        gpu_pci_id=gpu.get("pci_id"),
        gpu_vram_mb=gpu.get("vram_mb"),
        serial=backend.get_serial_number(),
    )
