from __future__ import annotations


def get_os_info() -> dict:
    return {"name": "Zorin OS", "version": "17.1", "id": "zorin"}


def get_processor_raw() -> str:
    return "Intel(R) Core(TM) i7-3520M CPU @ 2.90GHz"


def get_memory_info() -> dict:
    return {"total_mb": 8126, "speed_mhz": 1600, "type": "DDR3"}


def get_graphics_info() -> dict:
    return {
        "raw": "Intel Corporation 4th Gen Core Processor Integrated Graphics Controller",
        "pci_id": "8086:0a26",
    }


def get_serial_number() -> str:
    return "C02J1234XYZA"
