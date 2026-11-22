//! Light and dark colours, and whether the desktop prefers dark.

use std::process::Command;
use std::time::Duration;

use eframe::egui::Color32;

use crate::system::run_command;

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

const TIMEOUT: Duration = Duration::from_secs(2);

/// The command's stdout, or empty when it is missing, fails or times out.
fn run(cmd: &[&str]) -> String {
    let Some((program, args)) = cmd.split_first() else {
        return String::new();
    };
    run_command(Command::new(program).args(args), TIMEOUT).unwrap_or_default()
}

fn says_dark(cmd: &[&str]) -> bool {
    run(cmd).to_lowercase().contains("dark")
}

/// Whether the desktop prefers dark: the first source that says so wins.
pub fn detect_dark_mode() -> bool {
    let home = std::env::home_dir().unwrap_or_default();
    // 1. gsettings (GNOME)
    if says_dark(&[
        "gsettings",
        "get",
        "org.gnome.desktop.interface",
        "color-scheme",
    ]) || says_dark(&[
        "gsettings",
        "get",
        "org.gnome.desktop.interface",
        "gtk-theme",
    ]) {
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
        let value = ini_get(&text, "Settings", "gtk-application-prefer-dark-theme");
        if matches!(value.as_deref().map(str::trim), Some("1" | "true" | "True")) {
            return true;
        }
    }
    // 4. dconf through its CLI
    if says_dark(&["dconf", "read", "/org/gnome/desktop/interface/color-scheme"]) {
        return true;
    }
    // 5. dconf's binary database, for when its CLI is missing
    let db = home.join(".config/dconf/user");
    if std::fs::read(db).is_ok_and(|bytes| bytes.windows(11).any(|w| w == b"prefer-dark")) {
        return true;
    }
    // 6. xdg-desktop-portal: "(<uint32 1>,)" is dark
    run(&[
        "gdbus",
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
    ])
    .contains("<uint32 1>")
}

/// An option's value in an INI file, as GTK settings files are written: `[section]` headers,
/// `key = value` or `key: value`, option names case-insensitive, `#` and `;` comment lines,
/// indented continuation lines, `[DEFAULT]` inherited. A malformed file is read as far as it
/// can be.
fn ini_get(text: &str, section: &str, option: &str) -> Option<String> {
    let mut current: Option<String> = None;
    let mut last_key: Option<String> = None;
    let mut found: Option<String> = None;
    let mut default: Option<String> = None;
    let mut has_section = false;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        let indented = line.starts_with(char::is_whitespace);
        if indented && last_key.is_some() {
            let target = match current.as_deref() {
                Some("DEFAULT") => &mut default,
                Some(s) if s == section => &mut found,
                _ => continue,
            };
            if last_key.as_deref() == Some(option)
                && let Some(value) = target
            {
                value.push('\n');
                value.push_str(trimmed);
            }
            continue;
        }
        if let Some(header) = trimmed
            .strip_prefix('[')
            .and_then(|rest| rest.rfind(']').map(|end| &rest[..end]))
            .filter(|h| !h.is_empty())
        {
            has_section |= header == section;
            current = Some(header.to_string());
            last_key = None;
            continue;
        }
        let Some(at) = trimmed.find(['=', ':']) else {
            last_key = None;
            continue;
        };
        let key = trimmed[..at].trim_end().to_lowercase();
        let value = trimmed[at + 1..].trim_start().to_string();
        match current.as_deref() {
            Some("DEFAULT") if key == option => default = Some(value),
            Some(s) if s == section && key == option => found = Some(value),
            _ => {}
        }
        last_key = Some(key);
    }
    if !has_section {
        return None;
    }
    found.or(default)
}

#[cfg(test)]
mod tests {
    use super::*;

    const KEY: &str = "gtk-application-prefer-dark-theme";

    #[test]
    fn reads_the_settings_section() {
        let ini = "[Settings]\ngtk-theme-name=Breeze\nGtk-Application-Prefer-Dark-Theme = 1\n";
        assert_eq!(ini_get(ini, "Settings", KEY).as_deref(), Some("1"));
        assert_eq!(ini_get("[Other]\n", "Settings", KEY), None);
        assert_eq!(
            ini_get(
                "[DEFAULT]\nGTK-application-prefer-dark-theme: true\n[Settings]\n",
                "Settings",
                KEY
            )
            .as_deref(),
            Some("true")
        );
    }

    #[test]
    fn section_names_are_case_sensitive_and_comments_skipped() {
        assert_eq!(
            ini_get(
                "[settings]\ngtk-application-prefer-dark-theme=1\n",
                "Settings",
                KEY
            ),
            None
        );
        assert_eq!(
            ini_get(
                "[Settings]\n# gtk-application-prefer-dark-theme=1\n",
                "Settings",
                KEY
            ),
            None
        );
    }
}
