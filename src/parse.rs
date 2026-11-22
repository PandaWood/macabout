//! Reading the text of `/etc/os-release`, `/proc/cpuinfo`, `/proc/meminfo`, `lspci -nn`,
//! `lscpu`, `udevadm` and sysfs. No I/O here: `system.rs` reads the files and runs the commands.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::LazyLock;

use regex::Regex;

/// A constant pattern of this crate.
pub(crate) fn re(pattern: &str) -> Regex {
    // INFALLIBLE: every pattern is a constant, compiled by the tests.
    Regex::new(pattern).expect("valid regex")
}

static PARENS: LazyLock<Regex> = LazyLock::new(|| re(r"\s*\(.*\)\s*"));
static MEM_KB: LazyLock<Regex> = LazyLock::new(|| re(r"([0-9]+)\s*kB"));
static VGA: LazyLock<Regex> =
    LazyLock::new(|| re(r"VGA compatible controller|3D controller|Display controller"));
static PCI_ID: LazyLock<Regex> = LazyLock::new(|| re(r"\[([0-9a-f]{4}:[0-9a-f]{4})\]"));
static PCI_ID_TAIL: LazyLock<Regex> = LazyLock::new(|| re(r"\s*\[[0-9a-f]{4}:[0-9a-f]{4}\].*$"));

/// The distro, from `/etc/os-release`.
#[derive(Debug, PartialEq)]
pub struct OsRelease {
    /// `NAME`, or "Linux".
    pub name: String,
    /// `VERSION` without its codename in parentheses, else `VERSION_ID`; may be empty.
    pub version: String,
    /// `ID` in lower case, or "linux".
    pub id: String,
    /// `LOGO`, the icon name; may be empty.
    pub logo: String,
}

impl Default for OsRelease {
    fn default() -> Self {
        os_release("")
    }
}

/// `KEY=value` lines, quotes removed; comments and blank lines skipped. A repeated key keeps
/// its last value.
fn release_fields(content: &str) -> HashMap<&str, &str> {
    content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .filter_map(|line| line.split_once('='))
        .map(|(key, value)| {
            let value = value.trim();
            let unquoted = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .unwrap_or(value);
            (key, unquoted)
        })
        .collect()
}

pub fn os_release(content: &str) -> OsRelease {
    let fields = release_fields(content);
    let get = |key: &str| fields.get(key).copied().unwrap_or_default();
    let or =
        |value: &str, default: &str| if value.is_empty() { default } else { value }.to_string();
    let version = PARENS.replace_all(get("VERSION"), "");
    OsRelease {
        name: or(get("NAME"), "Linux"),
        version: or(version.trim(), get("VERSION_ID")),
        id: or(&get("ID").to_lowercase(), "linux"),
        logo: get("LOGO").to_string(),
    }
}

/// The value of a `key : value` line.
fn value_of(line: &str) -> Option<&str> {
    line.split_once(':').map(|(_, value)| value.trim())
}

/// The first `model name` in `/proc/cpuinfo`.
pub fn cpu_model(cpuinfo: &str) -> Option<String> {
    cpuinfo
        .lines()
        .filter(|line| line.starts_with("model name"))
        .find_map(value_of)
        .filter(|model| !model.is_empty())
        .map(str::to_string)
}

/// The CPU as `lscpu` names it from the part numbers ARM kernels list: "ARM Cortex-A72", or
/// with two kinds of core (big.LITTLE) "ARM Cortex-A55 + Cortex-A76".
pub fn lscpu_model(lscpu: &str) -> Option<String> {
    let mut vendor = None;
    let mut models: Vec<&str> = Vec::new();
    for (key, value) in lscpu.lines().filter_map(|line| line.trim().split_once(':')) {
        let value = value.trim();
        match key {
            "Vendor ID" => vendor = vendor.or(Some(value)),
            "Model name" if value != "-" && !value.is_empty() && !models.contains(&value) => {
                models.push(value);
            }
            _ => {}
        }
    }
    let models = models.join(" + ");
    match vendor {
        _ if models.is_empty() => None,
        Some(vendor) if !models.starts_with(vendor) => Some(format!("{vendor} {models}")),
        _ => Some(models),
    }
}

/// Physical cores: distinct (`physical id`, `core id`) pairs, else the number of `processor`
/// entries when the kernel lists no core ids.
pub fn cpu_cores(cpuinfo: &str) -> Option<u32> {
    let mut cores = HashSet::new();
    let mut package = "0";
    for line in cpuinfo.lines() {
        if line.starts_with("physical id") {
            package = value_of(line).unwrap_or(package);
        } else if line.starts_with("core id")
            && let Some(core) = value_of(line)
        {
            cores.insert((package, core));
        }
    }
    let count = if cores.is_empty() {
        cpuinfo
            .lines()
            .filter(|line| line.starts_with("processor"))
            .count()
    } else {
        cores.len()
    };
    u32::try_from(count).ok().filter(|&n| n > 0)
}

/// `MemTotal` in `/proc/meminfo`, in MiB.
pub fn mem_total_mb(meminfo: &str) -> Option<u64> {
    meminfo
        .lines()
        .filter(|line| line.starts_with("MemTotal:"))
        .find_map(|line| MEM_KB.captures(line)?[1].parse::<u64>().ok())
        .map(|kb| kb / 1024)
}

/// Type and speed of the installed memory.
#[derive(Debug, Default, PartialEq)]
pub struct MemoryDetails {
    /// "DDR4", "LPDDR5" and so on.
    pub kind: Option<String>,
    pub speed_mhz: Option<u32>,
}

/// The first populated memory slot's type and speed, from the `MEMORY_DEVICE_<n>_*` properties
/// udev reads from the firmware at boot (`udevadm info --query=property` of the DMI device).
pub fn memory_details(udev: &str) -> MemoryDetails {
    let mut slots: BTreeMap<u32, HashMap<&str, &str>> = BTreeMap::new();
    for (key, value) in udev.lines().filter_map(|line| line.split_once('=')) {
        if let Some((slot, field)) = key
            .strip_prefix("MEMORY_DEVICE_")
            .and_then(|rest| rest.split_once('_'))
            && let Ok(slot) = slot.parse()
        {
            slots.entry(slot).or_default().insert(field, value);
        }
    }
    slots
        .values()
        .find(|slot| {
            slot.get("SIZE")
                .and_then(|size| size.parse::<u64>().ok())
                .is_some_and(|bytes| bytes > 0)
        })
        .map(|slot| MemoryDetails {
            kind: slot
                .get("TYPE")
                .filter(|kind| !matches!(**kind, "Unknown" | "None" | ""))
                .map(|kind| kind.to_string()),
            speed_mhz: slot.get("SPEED_MTS").and_then(|s| s.parse().ok()),
        })
        .unwrap_or_default()
}

/// The graphics processor.
#[derive(Debug, PartialEq)]
pub struct Gpu {
    /// The device name, vendor included: "NVIDIA Corporation GA106 [GeForce RTX 3060]".
    pub name: String,
    /// Where it sits on the PCI bus; `None` for one built into the chip.
    pub pci: Option<Pci>,
}

#[derive(Debug, PartialEq)]
pub struct Pci {
    /// "vvvv:dddd", in lower case.
    pub id: String,
    /// The address `lspci` gives, "01:00.0", with the domain only when there are several.
    pub slot: String,
}

/// The first display controller `lspci -nn` lists, with its PCI id (the last `[vvvv:dddd]` on
/// its line).
pub fn lspci_gpu(lspci: &str) -> Option<Gpu> {
    lspci
        .lines()
        .filter(|line| VGA.is_match(line))
        .find_map(|line| {
            let id = PCI_ID.captures_iter(line).last()?;
            let slot = line.split_whitespace().next()?;
            let name = match line.split_once("]: ") {
                Some((_, rest)) => PCI_ID_TAIL.replace_all(rest, "").trim().to_string(),
                None => String::new(),
            };
            Some(Gpu {
                name,
                pci: Some(Pci {
                    id: id[1].to_lowercase(),
                    slot: slot.to_string(),
                }),
            })
        })
}

/// A graphics processor built into the chip (ARM boards), from its DRM driver's name.
pub fn drm_gpu(driver: &str) -> Option<Gpu> {
    let name = match driver {
        "v3d" | "vc4" => "Broadcom VideoCore",
        "panfrost" | "panthor" | "lima" => "Arm Mali",
        "msm" => "Qualcomm Adreno",
        "etnaviv" => "Vivante GPU",
        "asahi" => "Apple GPU",
        "tegra" => "NVIDIA Tegra",
        _ => return None,
    };
    Some(Gpu {
        name: name.to_string(),
        pci: None,
    })
}

/// Values firmware puts where it has nothing to say.
const DMI_PLACEHOLDERS: [&str; 5] = [
    "",
    "none",
    "not specified",
    "to be filled by o.e.m.",
    "default string",
];

/// A `/sys/class/dmi/id` value, unless it is a placeholder: one of [`DMI_PLACEHOLDERS`] or
/// `extra_placeholder`, in any case.
pub fn dmi_value(text: &str, extra_placeholder: &str) -> Option<String> {
    let value = text.trim();
    let lower = value.to_lowercase();
    let placeholder = DMI_PLACEHOLDERS.contains(&lower.as_str())
        || (!extra_placeholder.is_empty() && lower == extra_placeholder.to_lowercase());
    (!placeholder).then(|| value.to_string())
}

/// `mem_info_vram_total` (bytes), in MiB, unless it is zero.
pub fn vram_mb(text: &str) -> Option<u64> {
    text.trim()
        .parse::<u64>()
        .ok()
        .filter(|&bytes| bytes > 0)
        .map(|bytes| bytes / (1024 * 1024))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn os_release_ubuntu() {
        let os = os_release(
            "NAME=\"Ubuntu\"\nVERSION=\"22.04.3 LTS (Jammy Jellyfish)\"\nID=ubuntu\n\
             VERSION_ID=\"22.04\"\nLOGO=ubuntu-logo\n",
        );
        assert_eq!(
            os,
            OsRelease {
                name: "Ubuntu".into(),
                version: "22.04.3 LTS".into(),
                id: "ubuntu".into(),
                logo: "ubuntu-logo".into(),
            }
        );
    }

    #[test]
    fn os_release_comments_quotes_and_fallbacks() {
        let os = os_release("# comment\n\nNAME=Debian\nID=Debian\nVERSION_ID=\"12\"\n");
        assert_eq!((os.name.as_str(), os.id.as_str()), ("Debian", "debian"));
        assert_eq!(os.version, "12");
        assert_eq!(
            os_release("NAME=\"Zorin OS\"\nVERSION=\"17.1\"\n").name,
            "Zorin OS"
        );
        assert_eq!(
            os_release(""),
            OsRelease {
                name: "Linux".into(),
                version: String::new(),
                id: "linux".into(),
                logo: String::new(),
            }
        );
    }

    const CPUINFO: &str = "processor\t: 0\nmodel name\t: Intel(R) Core(TM) i7-3520M CPU @ 2.90GHz\n\
        physical id\t: 0\ncore id\t\t: 0\n\nprocessor\t: 1\nphysical id\t: 0\ncore id\t\t: 1\n\n\
        processor\t: 2\nphysical id\t: 0\ncore id\t\t: 0\n\nprocessor\t: 3\nphysical id\t: 0\n\
        core id\t\t: 1\n";

    #[test]
    fn cpuinfo() {
        assert_eq!(
            cpu_model(CPUINFO).as_deref(),
            Some("Intel(R) Core(TM) i7-3520M CPU @ 2.90GHz")
        );
        assert_eq!(cpu_cores(CPUINFO), Some(2), "hyper-threads are not cores");
        assert_eq!(cpu_cores("processor : 0\nprocessor : 1\n"), Some(2));
        assert_eq!(cpu_cores(""), None);
        assert_eq!(cpu_model("model name\n"), None);
    }

    #[test]
    fn arm_cpus() {
        let pi4 = "processor\t: 0\nBogoMIPS\t: 108.00\nCPU implementer\t: 0x41\nCPU part\t: 0xd08\n\n\
            processor\t: 1\n\nprocessor\t: 2\n\nprocessor\t: 3\n\nHardware\t: BCM2835\n\
            Model\t\t: Raspberry Pi 4 Model B Rev 1.4\n";
        assert_eq!(cpu_model(pi4), None, "ARM kernels give no model name");
        assert_eq!(cpu_cores(pi4), Some(4));

        let lscpu = "Architecture:            aarch64\nCPU(s):                  4\n\
            Vendor ID:               ARM\nModel name:              Cortex-A72\nModel:  3\n";
        assert_eq!(lscpu_model(lscpu).as_deref(), Some("ARM Cortex-A72"));
        let big_little = "Vendor ID:               ARM\n  Model name:            Cortex-A55\n\
            \x20   Model:               0\n  Model name:            Cortex-A76\n";
        assert_eq!(
            lscpu_model(big_little).as_deref(),
            Some("ARM Cortex-A55 + Cortex-A76")
        );
        assert_eq!(lscpu_model("Vendor ID: ARM\nModel name: -\n"), None);
        assert_eq!(lscpu_model(""), None);
    }

    #[test]
    fn meminfo() {
        let meminfo = "MemTotal:       8126464 kB\nMemFree:        1234567 kB\n";
        assert_eq!(mem_total_mb(meminfo), Some(7936));
        assert_eq!(mem_total_mb("MemTotal:      16384000 kB\n"), Some(16000));
        assert_eq!(mem_total_mb("MemFree: 123456 kB\n"), None);
        assert_eq!(mem_total_mb(""), None);
    }

    fn details(kind: Option<&str>, speed_mhz: Option<u32>) -> MemoryDetails {
        MemoryDetails {
            kind: kind.map(str::to_string),
            speed_mhz,
        }
    }

    #[test]
    fn udev_memory() {
        let udev = "MEMORY_ARRAY_LOCATION=System Board Or Motherboard\n\
            MEMORY_DEVICE_0_PRESENT=0\nMEMORY_DEVICE_0_LOCATOR=DIMM_A0\n\
            MEMORY_DEVICE_1_SIZE=8589934592\nMEMORY_DEVICE_1_TYPE=DDR4\n\
            MEMORY_DEVICE_1_SPEED_MTS=2666\nMEMORY_DEVICE_1_CONFIGURED_SPEED_MTS=2400\n\
            MEMORY_DEVICE_2_SIZE=8589934592\nMEMORY_DEVICE_2_TYPE=DDR3\n";
        assert_eq!(memory_details(udev), details(Some("DDR4"), Some(2666)));
        let unknown_type = "MEMORY_DEVICE_10_SIZE=4294967296\nMEMORY_DEVICE_10_TYPE=Unknown\n";
        assert_eq!(memory_details(unknown_type), details(None, None));
        assert_eq!(
            memory_details("ID_FOR_SEAT=dmi\n"),
            details(None, None),
            "no memory table: older systemd, or a VM"
        );
    }

    #[test]
    fn lspci() {
        let nvidia = "01:00.0 VGA compatible controller [0300]: NVIDIA Corporation GA106 \
            [GeForce RTX 3060] [10de:2503] (rev a1)\n";
        assert_eq!(
            lspci_gpu(nvidia),
            Some(Gpu {
                name: "NVIDIA Corporation GA106 [GeForce RTX 3060]".into(),
                pci: Some(Pci {
                    id: "10de:2503".into(),
                    slot: "01:00.0".into(),
                }),
            })
        );
        let amd = "03:00.0 Display controller [0380]: Advanced Micro Devices, Inc. [AMD/ATI] \
            Navi 10 [Radeon RX 5700 XT] [1002:731f] (rev c1)\n";
        assert_eq!(
            lspci_gpu(amd).and_then(|g| g.pci).map(|p| p.id).as_deref(),
            Some("1002:731f")
        );
        let nvidia_3d = "01:00.0 3D controller [0302]: NVIDIA Corporation TU117M \
            [GeForce GTX 1650 Mobile] [10de:1f9d] (rev a1)\n";
        assert_eq!(
            lspci_gpu(nvidia_3d)
                .and_then(|g| g.pci)
                .map(|p| p.id)
                .as_deref(),
            Some("10de:1f9d")
        );
        let sata = "00:1f.2 SATA controller [0106]: Intel Corporation 8 Series SATA Controller 1 \
            [AHCI mode] [8086:9c03]\n";
        assert_eq!(lspci_gpu(sata), None);
        assert_eq!(lspci_gpu(""), None);
        let domain = "0001:00:00.0 VGA compatible controller [0300]: Intel Corporation Device \
            [8086:46a6]\n";
        assert_eq!(
            lspci_gpu(domain)
                .and_then(|g| g.pci)
                .map(|p| p.slot)
                .as_deref(),
            Some("0001:00:00.0")
        );
    }

    #[test]
    fn drm_gpus() {
        assert_eq!(
            drm_gpu("v3d").map(|g| g.name).as_deref(),
            Some("Broadcom VideoCore")
        );
        assert_eq!(drm_gpu("panfrost").and_then(|g| g.pci), None);
        assert_eq!(drm_gpu("amdgpu"), None, "PCI cards come from lspci");
    }

    #[test]
    fn dmi_values() {
        assert_eq!(
            dmi_value("MacBookAir6,2\n", "system product name").as_deref(),
            Some("MacBookAir6,2")
        );
        assert_eq!(dmi_value("To Be Filled By O.E.M.\n", ""), None);
        assert_eq!(
            dmi_value("System Product Name\n", "system product name"),
            None
        );
        assert_eq!(dmi_value("\n", ""), None);
    }

    #[test]
    fn sysfs_vram() {
        assert_eq!(vram_mb("8573157376\n"), Some(8176));
        assert_eq!(vram_mb("0\n"), None);
        assert_eq!(vram_mb("junk"), None);
    }
}
