//! Turning raw hardware names into the clean ones the dialog shows: "2.9 GHz Dual-Core Intel
//! Core i7", "8 GB 1600 MHz DDR3", "Intel HD Graphics 5000 1536 MB".

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;
use serde::Deserialize;

use crate::parse::re;
use crate::system::SystemInfo;

static APPLE_M: LazyLock<Regex> = LazyLock::new(|| re(r"^Apple\s+M\d"));
static GHZ: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)@\s*([0-9]+(?:\.[0-9]+)?)\s*GHz"));
static CORE_I: LazyLock<Regex> = LazyLock::new(|| re(r"Core\s+(i[3579])"));
static XEON: LazyLock<Regex> = LazyLock::new(|| re(r"Xeon\s+([\w-]+)"));
static RYZEN: LazyLock<Regex> = LazyLock::new(|| re(r"Ryzen\s+(\d+)"));
static CPU_AT: LazyLock<Regex> = LazyLock::new(|| re(r"\s+CPU\s*@.*$"));
static TRAILING_GHZ: LazyLock<Regex> = LazyLock::new(|| re(r"\s+\d+(\.\d+)?\s*GHz$"));
static BRACKET_NAME: LazyLock<Regex> = LazyLock::new(|| re(r"\[([^\[\]]+)\]$"));
static MESA: LazyLock<Regex> = LazyLock::new(|| re(r"^Mesa\s+"));
static VENDOR: LazyLock<Regex> =
    LazyLock::new(|| re(r"^[\w\s,\.]+(?:,\s*Inc\.?)?\s*(?:\[[^\]]+\]\s*)?"));
static PARENS: LazyLock<Regex> = LazyLock::new(|| re(r"\s*\([^)]*\)\s*"));
static SPACES: LazyLock<Regex> = LazyLock::new(|| re(r"\s+"));

/// A card's marketing name and memory, by PCI id.
#[derive(Deserialize)]
struct GpuEntry {
    name: String,
    vram_mb: Option<u64>,
}

static GPU_LOOKUP: LazyLock<HashMap<String, GpuEntry>> = LazyLock::new(|| {
    // INFALLIBLE: a constant file, parsed by the tests.
    serde_json::from_str(include_str!("../data/gpu_lookup.json")).expect("valid gpu_lookup.json")
});

/// "2.9 GHz Dual-Core Intel Core i7" from "Intel(R) Core(TM) i7-3520M CPU @ 2.90GHz".
pub fn processor(raw: &str, cores: Option<u32>) -> String {
    if raw.is_empty() {
        return "N/A".to_string();
    }
    let s = raw
        .replace("(R)", "")
        .replace("(TM)", "")
        .replace("(tm)", "");
    if APPLE_M.is_match(&s) {
        return s.trim().to_string();
    }

    let ghz = GHZ
        .captures(&s)
        .and_then(|m| m[1].parse::<f64>().ok())
        .map(|ghz| {
            let ghz = format!("{ghz:.1}");
            ghz.strip_suffix(".0").unwrap_or(&ghz).to_string()
        });
    let cores = match cores {
        None | Some(0) => String::new(),
        Some(2) => "Dual-Core ".to_string(),
        Some(4) => "Quad-Core ".to_string(),
        Some(n) => format!("{n}-Core "),
    };
    let family = if let Some(m) = CORE_I.captures(&s) {
        format!("Intel Core {}", &m[1])
    } else if s.contains("Xeon") {
        match XEON.captures(&s) {
            Some(m) if !m[1].starts_with("CPU") => format!("Intel Xeon {}", &m[1]),
            _ => "Intel Xeon".to_string(),
        }
    } else if s.contains("Pentium") {
        "Intel Pentium".to_string()
    } else if s.contains("Celeron") {
        "Intel Celeron".to_string()
    } else if s.contains("Atom") {
        "Intel Atom".to_string()
    } else if s.contains("Ryzen") {
        match RYZEN.captures(&s) {
            Some(m) => format!("AMD Ryzen {}", &m[1]),
            None => "AMD Ryzen".to_string(),
        }
    } else if s.contains("EPYC") {
        "AMD EPYC".to_string()
    } else {
        let family = CPU_AT.replace(&s, "");
        TRAILING_GHZ.replace(family.trim(), "").trim().to_string()
    };
    match ghz {
        Some(ghz) => format!("{ghz} GHz {cores}{family}"),
        None => format!("{cores}{family}"),
    }
    .trim()
    .to_string()
}

const STANDARD_GB: [u64; 18] = [
    1, 2, 3, 4, 6, 8, 12, 16, 24, 32, 48, 64, 96, 128, 192, 256, 384, 512,
];

/// Installed memory in whole GB. The kernel reports a little less than is fitted, so a total
/// within 5% of a standard size is that size; anything else is rounded, half to even.
fn snap_gb(total_mb: u64) -> u64 {
    let gb = total_mb as f64 / 1024.0;
    let distance = |size: u64| (size as f64 - gb).abs();
    // `min_by` keeps the first of equal distances: the smaller size.
    let nearest = STANDARD_GB
        .into_iter()
        .min_by(|&a, &b| distance(a).total_cmp(&distance(b)));
    match nearest {
        Some(n) if distance(n) / n as f64 <= 0.05 => n,
        _ => gb.round_ties_even() as u64,
    }
}

/// "8 GB 1600 MHz DDR3".
pub fn memory(total_mb: u64, speed_mhz: Option<u32>, kind: Option<&str>) -> String {
    if total_mb == 0 {
        return "N/A".to_string();
    }
    let mut parts = vec![format!("{} GB", snap_gb(total_mb))];
    if let Some(speed) = speed_mhz.filter(|&s| s != 0) {
        parts.push(format!("{speed} MHz"));
    }
    if let Some(kind) = kind.filter(|k| !k.is_empty()) {
        parts.push(kind.to_string());
    }
    parts.join(" ")
}

/// Graphics memory: whole GB when within 3% of one (usable VRAM sits just under the
/// marketing figure), otherwise MB.
fn vram(vram_mb: u64) -> String {
    let gb = (vram_mb as f64 / 1024.0).round_ties_even() as u64;
    let off = vram_mb.abs_diff(gb * 1024);
    if gb != 0 && off as f64 <= 0.03 * (gb * 1024) as f64 {
        format!("{gb} GB")
    } else {
        format!("{vram_mb} MB")
    }
}

/// The card's marketing name from the lookup table by PCI id, else cleaned up from `lspci`'s
/// name; with its memory when known.
pub fn graphics(name: &str, pci_id: Option<&str>, vram_mb: Option<u64>) -> String {
    if let Some(entry) = pci_id.and_then(|id| GPU_LOOKUP.get(&id.to_lowercase())) {
        return match entry.vram_mb.filter(|&v| v != 0) {
            Some(v) => format!("{} {}", entry.name, vram(v)),
            None => entry.name.clone(),
        };
    }
    if name.is_empty() {
        return "N/A".to_string();
    }
    let clean = if let Some(m) = BRACKET_NAME.captures(name.trim()) {
        // "Intel Corporation Whiskey Lake-U GT2 [UHD Graphics 620]": the bracketed name, with
        // the vendor put back for Intel, whose names do not carry it.
        let bracketed = &m[1];
        if name.contains("Intel") && !bracketed.starts_with("Intel") {
            format!("Intel {bracketed}")
        } else {
            bracketed.to_string()
        }
    } else {
        let name = MESA.replace(name, "");
        let without_vendor = VENDOR.replace(&name, "");
        let name = match without_vendor.trim() {
            "" => &name,
            rest => rest,
        };
        let name = PARENS.replace_all(name, " ");
        SPACES.replace_all(&name, " ").trim().to_string()
    };
    match vram_mb.filter(|&v| v != 0) {
        Some(v) => format!("{clean} {}", vram(v)),
        None => clean,
    }
}

/// What the dialog and `--text` show.
pub struct Summary {
    pub os_name: String,
    /// May be empty.
    pub os_version: String,
    pub distro_id: String,
    pub logo_id: String,
    /// May be empty.
    pub model: String,
    pub processor: String,
    pub memory: String,
    pub graphics: String,
}

impl Summary {
    pub fn new(info: SystemInfo) -> Self {
        let memory = match &info.memory {
            Some(m) => memory(m.total_mb, m.details.speed_mhz, m.details.kind.as_deref()),
            None => memory(0, None, None),
        };
        let graphics = match &info.gpu {
            Some(gpu) => {
                let pci_id = gpu.pci.as_ref().map(|pci| pci.id.as_str());
                graphics(&gpu.name, pci_id, info.vram_mb)
            }
            None => graphics("", None, None),
        };
        Summary {
            os_name: info.os.name,
            os_version: info.os.version,
            distro_id: info.os.id,
            logo_id: info.os.logo,
            model: info.model.unwrap_or_default(),
            processor: processor(info.cpu.as_deref().unwrap_or_default(), info.cpu_cores),
            memory,
            graphics,
        }
    }

    /// The label and value rows under the title.
    pub fn rows(&self) -> [(&'static str, &str); 3] {
        [
            ("Processor", &self.processor),
            ("Memory", &self.memory),
            ("Graphics", &self.graphics),
        ]
    }

    /// "Version 17.1", or `None` when the distro has no version (rolling releases).
    pub fn version_line(&self) -> Option<String> {
        Some(&self.os_version)
            .filter(|v| !v.is_empty())
            .map(|v| format!("Version {v}"))
    }

    /// The summary as plain text: what `--text` prints and the copy button copies.
    pub fn copy_text(&self) -> String {
        let mut lines = vec![self.os_name.clone()];
        lines.extend(self.version_line());
        if !self.model.is_empty() {
            lines.push(self.model.clone());
        }
        lines.push(String::new());
        let rows = self.rows();
        let width = rows.iter().map(|(label, _)| label.len()).max().unwrap_or(0);
        for (label, value) in rows {
            lines.push(format!("{label:>width$}  {value}"));
        }
        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn processors() {
        let cases = [
            ("", None, "N/A"),
            (
                "Intel(R) Core(TM) i7-3520M CPU @ 2.90GHz",
                Some(2),
                "2.9 GHz Dual-Core Intel Core i7",
            ),
            (
                "Intel(R) Core(TM) i5-8250U CPU @ 1.60GHz",
                None,
                "1.6 GHz Intel Core i5",
            ),
            (
                "Intel(R) Core(TM) i7-6700K CPU @ 4.00GHz",
                Some(4),
                "4 GHz Quad-Core Intel Core i7",
            ),
            (
                "Intel(R) Core(TM) i9-9900K CPU @ 3.60GHz",
                Some(8),
                "3.6 GHz 8-Core Intel Core i9",
            ),
            ("Apple M1 Pro", None, "Apple M1 Pro"),
            (
                "AMD Ryzen 7 5800X 8-Core Processor @ 3.80GHz",
                Some(8),
                "3.8 GHz 8-Core AMD Ryzen 7",
            ),
            ("AMD Ryzen 5 3600", None, "AMD Ryzen 5"),
            ("AMD EPYC 7502 32-Core Processor", None, "AMD EPYC"),
            (
                "Intel(R) Xeon(R) E5-2670 @ 2.60GHz",
                None,
                "2.6 GHz Intel Xeon E5-2670",
            ),
            (
                "Intel(R) Xeon(R) CPU E5-2670 @ 2.60GHz",
                None,
                "2.6 GHz Intel Xeon",
            ),
            (
                "Intel(R) Celeron(R) CPU N3060 @ 1.60GHz",
                None,
                "1.6 GHz Intel Celeron",
            ),
            (
                "Intel(R) Pentium(R) CPU G4560 @ 3.50GHz",
                None,
                "3.5 GHz Intel Pentium",
            ),
            (
                "Intel(R) Atom(TM) CPU Z3735F @ 1.33GHz",
                None,
                "1.3 GHz Intel Atom",
            ),
            (
                "Intel(R) Core(TM) i7-1165G7 @ 2.80GHz",
                None,
                "2.8 GHz Intel Core i7",
            ),
            ("Foo Bar CPU @ 2.05GHz", Some(6), "2 GHz 6-Core Foo Bar"),
            ("Some Chip 1.8 GHz", None, "Some Chip"),
        ];
        for (raw, cores, want) in cases {
            assert_eq!(processor(raw, cores), want, "{raw}");
        }
    }

    #[test]
    fn memory_sizes_snap_to_standard_ones() {
        assert_eq!(snap_gb(16 * 1024), 16);
        assert_eq!(snap_gb(8126), 8);
        assert_eq!(snap_gb(16256), 16);
        assert_eq!(snap_gb(10 * 1024), 10);
        assert_eq!(snap_gb(2560), 2, "2.5 rounds to even");
        assert_eq!(snap_gb(11776), 12, "11.5 rounds to even");
    }

    #[test]
    fn memory_text() {
        assert_eq!(memory(0, None, None), "N/A");
        assert_eq!(memory(8126, Some(1600), Some("DDR3")), "8 GB 1600 MHz DDR3");
        assert_eq!(memory(16 * 1024, None, None), "16 GB");
        assert_eq!(memory(16 * 1024, Some(3200), None), "16 GB 3200 MHz");
        assert_eq!(memory(16 * 1024, None, Some("DDR4")), "16 GB DDR4");
    }

    #[test]
    fn vram_text() {
        assert_eq!(vram(8 * 1024), "8 GB");
        assert_eq!(vram(512), "512 MB");
        assert_eq!(
            vram(8176),
            "8 GB",
            "AMD reports usable VRAM, a little under 8 GB"
        );
        assert_eq!(vram(4080), "4 GB");
        assert_eq!(vram(1536), "1536 MB", "a real 1.5 GB card stays in MB");
    }

    #[test]
    fn graphics_names() {
        assert_eq!(graphics("", None, None), "N/A");
        assert_eq!(
            graphics("Intel UHD Graphics 620", None, Some(1024)),
            "Intel UHD Graphics 620 1 GB"
        );
        assert_eq!(
            graphics(
                "Advanced Micro Devices, Inc. [AMD/ATI] Navi 10 [Radeon RX 5700 XT]",
                None,
                None
            ),
            "Radeon RX 5700 XT"
        );
        assert_eq!(
            graphics(
                "Intel Corporation Whiskey Lake-U GT2 [UHD Graphics 620]",
                None,
                None
            ),
            "Intel UHD Graphics 620"
        );
        assert_eq!(
            graphics("Mesa Intel(R) UHD Graphics 620 (WHL GT2)", None, None),
            "UHD Graphics 620"
        );
        assert_eq!(
            graphics("Fake GPU Name", Some("ffff:ffff"), None),
            "Fake GPU Name"
        );
    }

    #[test]
    fn graphics_lookup_by_pci_id() {
        assert!(!GPU_LOOKUP.is_empty());
        let (id, entry) = GPU_LOOKUP.iter().next().unwrap();
        assert!(graphics("ignored", Some(&id.to_uppercase()), None).starts_with(&entry.name));
    }
}
