"""Tests for macabout.linux_collectors — parser functions only, no subprocess calls."""
from __future__ import annotations

import textwrap

import pytest

from macabout.linux_collectors import (
    _parse_dmidecode_memory,
    _parse_lspci_vga,
    _parse_meminfo_total_mb,
    _parse_os_release,
)


# ---------------------------------------------------------------------------
# _parse_os_release
# ---------------------------------------------------------------------------

class TestParseOsRelease:
    def test_basic_ubuntu(self):
        content = textwrap.dedent("""\
            NAME="Ubuntu"
            VERSION="22.04.3 LTS (Jammy Jellyfish)"
            ID=ubuntu
            VERSION_ID="22.04"
            LOGO=ubuntu-logo
        """)
        result = _parse_os_release(content)
        assert result["NAME"] == "Ubuntu"
        assert result["ID"] == "ubuntu"
        assert result["LOGO"] == "ubuntu-logo"

    def test_comments_and_blank_lines_ignored(self):
        content = "# This is a comment\n\nNAME=Debian\nID=debian\n"
        result = _parse_os_release(content)
        assert result["NAME"] == "Debian"
        assert "#" not in result

    def test_unquoted_value(self):
        result = _parse_os_release("ID=fedora\n")
        assert result["ID"] == "fedora"

    def test_quoted_value_strips_quotes(self):
        result = _parse_os_release('NAME="Zorin OS"\n')
        assert result["NAME"] == "Zorin OS"

    def test_empty_content(self):
        assert _parse_os_release("") == {}

    def test_zorin_full(self):
        content = textwrap.dedent("""\
            NAME="Zorin OS"
            VERSION="17.1"
            ID=zorin
            ID_LIKE=ubuntu
            LOGO=zorin-os-logo
            VERSION_ID="17"
        """)
        result = _parse_os_release(content)
        assert result["NAME"] == "Zorin OS"
        assert result["VERSION"] == "17.1"
        assert result["LOGO"] == "zorin-os-logo"


# ---------------------------------------------------------------------------
# _parse_meminfo_total_mb
# ---------------------------------------------------------------------------

class TestParseMeminfTotalMb:
    def test_8gb(self):
        content = "MemTotal:       8126464 kB\nMemFree:        1234567 kB\n"
        assert _parse_meminfo_total_mb(content) == 7936  # integer division

    def test_16gb(self):
        content = "MemTotal:      16384000 kB\n"
        assert _parse_meminfo_total_mb(content) == 16000

    def test_missing_memtotal(self):
        assert _parse_meminfo_total_mb("MemFree: 123456 kB\n") is None

    def test_empty(self):
        assert _parse_meminfo_total_mb("") is None


# ---------------------------------------------------------------------------
# _parse_dmidecode_memory
# ---------------------------------------------------------------------------

_DMI_SAMPLE = """\
# dmidecode 3.3
Getting SMBIOS data from sysfs.

Handle 0x1100, DMI type 17, 40 bytes
Memory Device
\tArray Handle: 0x1000
\tError Information Handle: Not Provided
\tTotal Width: 64 bits
\tData Width: 64 bits
\tSize: 8 GB
\tForm Factor: Row Of Chips
\tSet: None
\tLocator: LPDDR4
\tBank Locator: BANK 0
\tType: LPDDR4
\tType Detail: Synchronous Unbuffered (Unregistered)
\tSpeed: 2133 MT/s
\tManufacturer: Samsung
\tSerial Number: Not Specified
\tAsset Tag: Not Specified
\tPart Number: K4EBE304EB-EGCG

"""

_DMI_NO_MODULE = """\
Handle 0x1101, DMI type 17, 40 bytes
Memory Device
\tSize: No Module Installed
\tType: Unknown

Handle 0x1102, DMI type 17, 40 bytes
Memory Device
\tSize: 16 GB
\tType: DDR4
\tSpeed: 3200 MT/s

"""

_DMI_UNKNOWN_TYPE = """\
Handle 0x1100, DMI type 17, 40 bytes
Memory Device
\tSize: 8 GB
\tType: Unknown
\tSpeed: 2400 MT/s

"""


class TestParseDmidecodeMemory:
    def test_basic_lpddr4(self):
        result = _parse_dmidecode_memory(_DMI_SAMPLE)
        assert result["type"] == "LPDDR4"
        assert result["speed_mhz"] == 2133

    def test_skips_no_module_installed(self):
        result = _parse_dmidecode_memory(_DMI_NO_MODULE)
        assert result["type"] == "DDR4"
        assert result["speed_mhz"] == 3200

    def test_unknown_type_becomes_none(self):
        result = _parse_dmidecode_memory(_DMI_UNKNOWN_TYPE)
        assert result["type"] is None
        assert result["speed_mhz"] == 2400

    def test_empty_output(self):
        result = _parse_dmidecode_memory("")
        assert result == {"type": None, "speed_mhz": None}

    def test_mhz_unit_accepted(self):
        content = (
            "Memory Device\n"
            "\tSize: 4 GB\n"
            "\tType: DDR3\n"
            "\tSpeed: 1600 MHz\n\n"
        )
        result = _parse_dmidecode_memory(content)
        assert result["speed_mhz"] == 1600


# ---------------------------------------------------------------------------
# _parse_lspci_vga
# ---------------------------------------------------------------------------

_LSPCI_NVIDIA = (
    "01:00.0 VGA compatible controller [0300]: NVIDIA Corporation GA106 [GeForce RTX 3060] "
    "[10de:2503] (rev a1)\n"
)

_LSPCI_AMD = (
    "03:00.0 Display controller [0380]: Advanced Micro Devices, Inc. [AMD/ATI] "
    "Navi 10 [Radeon RX 5700 XT] [1002:731f] (rev c1)\n"
)

_LSPCI_INTEL = (
    "00:02.0 VGA compatible controller [0300]: Intel Corporation "
    "UHD Graphics 620 [8086:3ea0] (rev 07)\n"
)

_LSPCI_NO_GPU = (
    "00:1f.2 SATA controller [0106]: Intel Corporation 8 Series SATA Controller 1 [AHCI mode] [8086:9c03]\n"
)


class TestParseLspciVga:
    def test_nvidia_rtx(self):
        result = _parse_lspci_vga(_LSPCI_NVIDIA)
        assert result is not None
        assert result["pci_id"] == "10de:2503"
        assert "GeForce RTX 3060" in result["raw"]  # marketing name kept in raw; format_graphics strips it later

    def test_amd_display_controller(self):
        result = _parse_lspci_vga(_LSPCI_AMD)
        assert result is not None
        assert result["pci_id"] == "1002:731f"

    def test_intel_vga(self):
        result = _parse_lspci_vga(_LSPCI_INTEL)
        assert result is not None
        assert result["pci_id"] == "8086:3ea0"

    def test_non_gpu_line_ignored(self):
        assert _parse_lspci_vga(_LSPCI_NO_GPU) is None

    def test_empty(self):
        assert _parse_lspci_vga("") is None

    def test_3d_controller_matches(self):
        line = (
            "01:00.0 3D controller [0302]: NVIDIA Corporation TU117M [GeForce GTX 1650 Mobile] "
            "[10de:1f9d] (rev a1)\n"
        )
        result = _parse_lspci_vga(line)
        assert result is not None
        assert result["pci_id"] == "10de:1f9d"
