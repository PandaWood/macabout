//! Light and dark colours, and whether the desktop prefers dark.

use eframe::egui::Color32;

use crate::system::run;

/// Dialog colours.
pub struct Palette {
    pub bg: Color32,
    pub fg: Color32,
    pub subtle: Color32,
    pub ver: Color32,
}

pub const LIGHT: Palette = Palette {
    bg: Color32::from_rgb(0xEC, 0xEC, 0xEC),
    fg: Color32::from_rgb(0x1A, 0x1A, 0x1A),
    subtle: Color32::from_rgb(0x4D, 0x4D, 0x4D),
    ver: Color32::from_rgb(0xAA, 0xAA, 0xAA),
};

pub const DARK: Palette = Palette {
    bg: Color32::from_rgb(0x24, 0x24, 0x24),
    fg: Color32::from_rgb(0xF2, 0xF2, 0xF2),
    subtle: Color32::from_rgb(0xAA, 0xAA, 0xAA),
    ver: Color32::from_rgb(0x66, 0x66, 0x66),
};

fn says_dark(program: &str, args: &[&str]) -> bool {
    run(program, args).is_some_and(|out| out.to_lowercase().contains("dark"))
}

/// Whether the desktop prefers dark: the first source that says so wins.
pub fn detect_dark_mode() -> bool {
    let home = std::env::home_dir().unwrap_or_default();
    // 1. gsettings (GNOME)
    let interface = |key| ["get", "org.gnome.desktop.interface", key];
    if says_dark("gsettings", &interface("color-scheme"))
        || says_dark("gsettings", &interface("gtk-theme"))
    {
        return true;
    }
    // 2. GTK_THEME (KDE, Xfce, ...)
    if std::env::var("GTK_THEME").is_ok_and(|theme| theme.to_lowercase().contains("dark")) {
        return true;
    }
    // 3. GTK settings files
    for version in ["gtk-4.0", "gtk-3.0"] {
        let path = home.join(".config").join(version).join("settings.ini");
        let Ok(text) = std::fs::read_to_string(path) else {
            continue;
        };
        if matches!(
            gtk_setting(&text, "gtk-application-prefer-dark-theme"),
            Some("1" | "true")
        ) {
            return true;
        }
    }
    // 4. dconf through its CLI
    if says_dark(
        "dconf",
        &["read", "/org/gnome/desktop/interface/color-scheme"],
    ) {
        return true;
    }
    // 5. dconf's binary database, for when its CLI is missing
    let db = home.join(".config/dconf/user");
    if std::fs::read(db).is_ok_and(|bytes| bytes.windows(11).any(|w| w == b"prefer-dark")) {
        return true;
    }
    // 6. xdg-desktop-portal: "(<uint32 1>,)" is dark
    run(
        "gdbus",
        &[
            "call",
            "--session",
            "--dest",
            "org.freedesktop.portal.Desktop",
            "--object-path",
            "/org/freedesktop/portal/desktop",
            "--method",
            "org.freedesktop.portal.Settings.ReadOne",
            "org.freedesktop.appearance",
            "color-scheme",
        ],
    )
    .is_some_and(|out| out.contains("<uint32 1>"))
}

/// A key's value in the `[Settings]` group of a GTK `settings.ini`, a GLib key file: `#`
/// comments, `key=value` lines, group and key names case-sensitive, the last of a repeated key
/// winning.
fn gtk_setting<'a>(text: &'a str, key: &str) -> Option<&'a str> {
    let mut in_settings = false;
    let mut value = None;
    for line in text.lines().map(str::trim) {
        if line.starts_with('[') {
            in_settings = line == "[Settings]";
        } else if in_settings
            && let Some((k, v)) = line.split_once('=')
            && k.trim_end() == key
        {
            value = Some(v.trim_start());
        }
    }
    value
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "gtk-application-prefer-dark-theme";

    #[test]
    fn reads_the_settings_group() {
        let ini = "[Settings]\ngtk-theme-name=Breeze\ngtk-application-prefer-dark-theme = 1\n";
        assert_eq!(gtk_setting(ini, KEY), Some("1"));
        assert_eq!(gtk_setting("[Other]\n", KEY), None);
        assert_eq!(gtk_setting(&format!("{KEY}=1\n[Settings]\n"), KEY), None);
        assert_eq!(
            gtk_setting(&format!("[Settings]\n{KEY}=0\n{KEY}=true\n"), KEY),
            Some("true")
        );
    }

    #[test]
    fn names_are_case_sensitive_and_comments_skipped() {
        assert_eq!(gtk_setting(&format!("[settings]\n{KEY}=1\n"), KEY), None);
        assert_eq!(
            gtk_setting("[Settings]\nGtk-Application-Prefer-Dark-Theme=1\n", KEY),
            None
        );
        assert_eq!(gtk_setting(&format!("[Settings]\n# {KEY}=1\n"), KEY), None);
    }
}
