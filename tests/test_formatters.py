"""Tests for macabout.formatters — all pure functions, no I/O."""
from __future__ import annotations

import pytest

from macabout.formatters import (
    _format_vram,
    _snap_gb,
    format_graphics,
    format_memory,
    format_processor,
)


# ---------------------------------------------------------------------------
# format_processor
# ---------------------------------------------------------------------------

class TestFormatProcessor:
    def test_empty_returns_na(self):
        assert format_processor("") == "N/A"

    def test_intel_core_i7_with_ghz_and_cores(self):
        result = format_processor("Intel(R) Core(TM) i7-3520M CPU @ 2.90GHz", cores=2)
        assert result == "2.9 GHz Dual-Core Intel Core i7"

    def test_intel_core_i5_no_cores(self):
        result = format_processor("Intel(R) Core(TM) i5-8250U CPU @ 1.60GHz")
        assert result == "1.6 GHz Intel Core i5"

    def test_intel_core_i3(self):
        result = format_processor("Intel(R) Core(TM) i3-4130 CPU @ 3.40GHz", cores=2)
        assert result == "3.4 GHz Dual-Core Intel Core i3"

    def test_apple_silicon_passthrough(self):
        assert format_processor("Apple M2") == "Apple M2"
        assert format_processor("Apple M1 Pro") == "Apple M1 Pro"

    def test_amd_ryzen(self):
        result = format_processor("AMD Ryzen 7 5800X 8-Core Processor @ 3.80GHz", cores=8)
        assert result == "3.8 GHz 8-Core AMD Ryzen 7"

    def test_amd_ryzen_no_ghz(self):
        result = format_processor("AMD Ryzen 5 3600")
        assert result == "AMD Ryzen 5"

    def test_amd_epyc(self):
        result = format_processor("AMD EPYC 7502 32-Core Processor")
        assert result == "AMD EPYC"

    def test_intel_xeon(self):
        result = format_processor("Intel(R) Xeon(R) E5-2670 @ 2.60GHz")
        assert result == "2.6 GHz Intel Xeon E5-2670"

    def test_intel_xeon_cpu_suffix_stripped(self):
        # "CPU" immediately after model should not become part of the suffix
        result = format_processor("Intel(R) Xeon(R) CPU E5-2670 @ 2.60GHz")
        assert "Intel Xeon" in result

    def test_intel_celeron(self):
        result = format_processor("Intel(R) Celeron(R) CPU N3060 @ 1.60GHz")
        assert result == "1.6 GHz Intel Celeron"

    def test_intel_pentium(self):
        result = format_processor("Intel(R) Pentium(R) CPU G4560 @ 3.50GHz")
        assert result == "3.5 GHz Intel Pentium"

    def test_intel_atom(self):
        result = format_processor("Intel(R) Atom(TM) CPU Z3735F @ 1.33GHz")
        assert result == "1.3 GHz Intel Atom"  # rounds to 1 decimal place

    def test_quad_core_label(self):
        result = format_processor("Intel(R) Core(TM) i7-6700K CPU @ 4.00GHz", cores=4)
        assert "Quad-Core" in result

    def test_unknown_core_count_numeric(self):
        result = format_processor("Intel(R) Core(TM) i9-9900K CPU @ 3.60GHz", cores=8)
        assert "8-Core" in result

    def test_trademark_symbols_stripped(self):
        result = format_processor("Intel(R) Core(TM) i7-1165G7 @ 2.80GHz")
        assert "(R)" not in result
        assert "(TM)" not in result


# ---------------------------------------------------------------------------
# _snap_gb
# ---------------------------------------------------------------------------

class TestSnapGb:
    def test_exact_standard_size(self):
        assert _snap_gb(16 * 1024) == 16

    def test_near_standard_size_snaps(self):
        # 8126 MB is ~7.94 GB — within 5% of 8 GB
        assert _snap_gb(8126) == 8

    def test_near_standard_size_snaps_16(self):
        # 16256 MB ≈ 15.875 GB — within 5% of 16
        assert _snap_gb(16256) == 16

    def test_non_standard_rounds_normally(self):
        # 10 GB is not a standard size; 10 * 1024 should round to 10
        assert _snap_gb(10 * 1024) == 10

    def test_32gb(self):
        assert _snap_gb(32 * 1024) == 32


# ---------------------------------------------------------------------------
# format_memory
# ---------------------------------------------------------------------------

class TestFormatMemory:
    def test_zero_returns_na(self):
        assert format_memory(0, None, None) == "N/A"

    def test_basic_with_all_fields(self):
        assert format_memory(8126, 1600, "DDR3") == "8 GB 1600 MHz DDR3"

    def test_no_speed_or_type(self):
        assert format_memory(16 * 1024, None, None) == "16 GB"

    def test_speed_no_type(self):
        assert format_memory(16 * 1024, 3200, None) == "16 GB 3200 MHz"

    def test_type_no_speed(self):
        assert format_memory(16 * 1024, None, "DDR4") == "16 GB DDR4"

    def test_32gb_ddr5(self):
        assert format_memory(32 * 1024, 4800, "DDR5") == "32 GB 4800 MHz DDR5"


# ---------------------------------------------------------------------------
# _format_vram
# ---------------------------------------------------------------------------

class TestFormatVram:
    def test_exact_gb(self):
        assert _format_vram(8 * 1024) == "8 GB"

    def test_fractional_mb(self):
        assert _format_vram(512) == "512 MB"

    def test_6gb(self):
        assert _format_vram(6 * 1024) == "6 GB"


# ---------------------------------------------------------------------------
# format_graphics
# ---------------------------------------------------------------------------

class TestFormatGraphics:
    def test_empty_raw_no_pci_id(self):
        assert format_graphics("", None) == "N/A"

    def test_vram_in_gb(self):
        result = format_graphics("Intel UHD Graphics 620", None, vram_mb=1024)
        assert result.endswith("1 GB")

    def test_vram_in_mb(self):
        result = format_graphics("Some GPU", None, vram_mb=512)
        assert result.endswith("512 MB")

    def test_bracketed_name_preferred(self):
        raw = "Advanced Micro Devices, Inc. [AMD/ATI] Navi 10 [Radeon RX 5700 XT]"
        result = format_graphics(raw, None)
        assert result == "Radeon RX 5700 XT"

    def test_intel_prefix_restored_for_integrated(self):
        raw = "Intel Corporation UHD Graphics 620 [8086:3ea0]"
        # Manually craft a line that has Intel in the vendor but not in the bracket
        raw2 = "Intel Corporation Whiskey Lake-U GT2 [UHD Graphics 620]"
        result = format_graphics(raw2, None)
        assert result.startswith("Intel")

    def test_mesa_prefix_stripped(self):
        result = format_graphics("Mesa Intel(R) UHD Graphics 620 (WHL GT2)", None)
        assert not result.startswith("Mesa")

    def test_pci_id_lookup_miss_falls_back_to_raw(self):
        # An ID that definitely isn't in the lookup
        result = format_graphics("Fake GPU Name", "ffff:ffff")
        assert "Fake" in result or result == "N/A"
