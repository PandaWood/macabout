"""Tests for macabout.ui — the GTK settings reader."""
from __future__ import annotations

from macabout.ui import _gtk_setting

KEY = "gtk-application-prefer-dark-theme"


class TestGtkSetting:
    def test_reads_the_settings_group(self):
        ini = f"[Settings]\ngtk-theme-name=Breeze\n{KEY} = 1\n"
        assert _gtk_setting(ini, KEY) == "1"

    def test_other_group_or_no_group(self):
        assert _gtk_setting(f"[Other]\n{KEY}=1\n", KEY) is None
        assert _gtk_setting(f"{KEY}=1\n[Settings]\n", KEY) is None

    def test_repeated_key_last_wins(self):
        assert _gtk_setting(f"[Settings]\n{KEY}=0\n{KEY}=true\n", KEY) == "true"

    def test_names_are_case_sensitive(self):
        assert _gtk_setting(f"[settings]\n{KEY}=1\n", KEY) is None
        assert _gtk_setting("[Settings]\nGtk-Application-Prefer-Dark-Theme=1\n", KEY) is None

    def test_comments_skipped(self):
        assert _gtk_setting(f"[Settings]\n# {KEY}=1\n", KEY) is None
