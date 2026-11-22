//! Reading this computer's facts: `/etc/os-release`, `/proc`, sysfs, the device tree, `lspci`,
//! `lscpu` and udev, all readable without root. Or, with `--mock`, a made-up computer.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

use crate::parse::{self, Gpu, MemoryDetails, OsRelease, Pci};

/// Everything the summary is made from. `None` where a source was missing or said nothing.
pub struct SystemInfo {
    pub os: OsRelease,
    /// The CPU model as `/proc/cpuinfo` names it, or on ARM as `lscpu` does.
    pub cpu: Option<String>,
    pub cpu_cores: Option<u32>,
    pub memory: Option<Memory>,
    pub gpu: Option<Gpu>,
    /// The graphics card's own memory, where the driver reports it in sysfs.
    pub vram_mb: Option<u64>,
    /// The product name, such as "MacBook Air (13-inch, Mid 2013)" or "Raspberry Pi 4 Model B".
    pub model: Option<String>,
}

pub struct Memory {
    pub total_mb: u64,
    pub details: MemoryDetails,
}

const TIMEOUT: Duration = Duration::from_secs(3);

/// A text file, `None` if it cannot be read. Line ends become `\n`.
fn read_text(path: impl AsRef<Path>) -> Option<String> {
    std::fs::read(path).ok().map(|bytes| text(&bytes))
}

fn text(bytes: &[u8]) -> String {
    String::from_utf8_lossy(bytes)
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

/// A command's stdout, `None` if it is missing, fails, or has not finished after `timeout`.
fn run_command(command: &mut Command, timeout: Duration) -> Option<String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    // Read on a thread of its own, so the deadline holds even when a process the command
    // started (a D-Bus daemon, say) keeps the pipe open after the command exits.
    let (sender, receiver) = mpsc::channel();
    std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.read_to_end(&mut out);
        let _ = sender.send(out);
    });
    let deadline = Instant::now() + timeout;
    let result = receiver.recv_timeout(timeout).ok().and_then(|out| {
        loop {
            match child.try_wait() {
                Ok(Some(status)) => break status.success().then(|| text(&out)),
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(5))
                }
                _ => break None,
            }
        }
    });
    if result.is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
    result
}

/// `program`'s stdout, as [`run_command`] gives it.
pub fn run(program: &str, args: &[&str]) -> Option<String> {
    run_command(Command::new(program).args(args), TIMEOUT)
}

/// The total from `/proc/meminfo`; type and speed from udev, which reads the firmware's memory
/// table at boot and keeps it where anyone can read it (systemd 248 and later).
fn memory() -> Option<Memory> {
    let total_mb = parse::mem_total_mb(&read_text("/proc/meminfo")?)?;
    let udev = run(
        "udevadm",
        &["info", "--query=property", "--path=/devices/virtual/dmi/id"],
    );
    Some(Memory {
        total_mb,
        details: udev
            .map(|out| parse::memory_details(&out))
            .unwrap_or_default(),
    })
}

/// The card's own memory, where its driver reports it in sysfs (amdgpu does).
fn vram_mb(pci: &Pci) -> Option<u64> {
    // lspci leaves out the PCI domain when there is only domain 0.
    let address = if pci.slot.matches(':').count() == 2 {
        pci.slot.clone()
    } else {
        format!("0000:{}", pci.slot)
    };
    read_text(format!(
        "/sys/bus/pci/devices/{address}/mem_info_vram_total"
    ))
    .and_then(|text| parse::vram_mb(&text))
}

/// A graphics processor built into the chip, as on ARM boards: no PCI device, so named after
/// the first DRM card whose driver is known.
fn drm_gpu() -> Option<Gpu> {
    let mut cards: Vec<_> = std::fs::read_dir("/sys/class/drm")
        .ok()?
        .flatten()
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str()?.strip_prefix("card"))
                .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()))
        })
        .collect();
    cards.sort();
    cards.iter().find_map(|card| {
        let driver = std::fs::read_link(card.join("device/driver")).ok()?;
        parse::drm_gpu(driver.file_name()?.to_str()?)
    })
}

/// The CPU's name: `/proc/cpuinfo`'s on x86; ARM kernels give only part numbers there, which
/// `lscpu` turns into names.
fn cpu_model(cpuinfo: &str) -> Option<String> {
    parse::cpu_model(cpuinfo).or_else(|| {
        // English labels, whatever the user's language.
        let lscpu = run_command(Command::new("lscpu").env("LC_ALL", "C"), TIMEOUT)?;
        parse::lscpu_model(&lscpu)
    })
}

/// The product name from the firmware's DMI table, else the device tree's (ARM boards).
fn model() -> Option<String> {
    read_text("/sys/class/dmi/id/product_name")
        .and_then(|name| parse::dmi_value(&name, "system product name"))
        .or_else(|| {
            let model = read_text("/proc/device-tree/model")?;
            parse::dmi_value(model.trim_end_matches('\0'), "")
        })
}

fn this_computer() -> SystemInfo {
    let cpuinfo = read_text("/proc/cpuinfo").unwrap_or_default();
    let gpu = run("lspci", &["-nn"])
        .and_then(|out| parse::lspci_gpu(&out))
        .or_else(drm_gpu);
    SystemInfo {
        os: read_text("/etc/os-release")
            .map(|text| parse::os_release(&text))
            .unwrap_or_default(),
        cpu: cpu_model(&cpuinfo),
        cpu_cores: parse::cpu_cores(&cpuinfo),
        memory: memory(),
        vram_mb: gpu.as_ref().and_then(|gpu| vram_mb(gpu.pci.as_ref()?)),
        gpu,
        model: model(),
    }
}

/// A made-up computer, for working on the dialog and for the tests.
fn mock() -> SystemInfo {
    SystemInfo {
        os: OsRelease {
            name: "Zorin OS".to_string(),
            version: "17.1".to_string(),
            id: "zorin".to_string(),
            logo: String::new(),
        },
        cpu: Some("Intel(R) Core(TM) i7-3520M CPU @ 2.90GHz".to_string()),
        cpu_cores: Some(2),
        memory: Some(Memory {
            total_mb: 8126,
            details: MemoryDetails {
                kind: Some("DDR3".to_string()),
                speed_mhz: Some(1600),
            },
        }),
        gpu: Some(Gpu {
            name: "Intel Corporation 4th Gen Core Processor Integrated Graphics Controller"
                .to_string(),
            pci: Some(Pci {
                id: "8086:0a26".to_string(),
                slot: "00:02.0".to_string(),
            }),
        }),
        vram_mb: None,
        model: Some("MacBook Air (13-inch, Mid 2013)".to_string()),
    }
}

pub fn collect(mock_data: bool) -> SystemInfo {
    if mock_data { mock() } else { this_computer() }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sh(script: &str, timeout: Duration) -> Option<String> {
        run_command(Command::new("sh").args(["-c", script]), timeout)
    }

    #[test]
    fn runs_commands() {
        assert_eq!(sh("echo hi", TIMEOUT).as_deref(), Some("hi\n"));
        assert_eq!(sh("echo hi; exit 1", TIMEOUT), None);
        assert_eq!(run("no-such-program-here", &[]), None);
    }

    #[test]
    fn the_deadline_holds_when_a_child_keeps_the_pipe_open() {
        let start = Instant::now();
        assert_eq!(sh("sleep 5 & echo hi", Duration::from_millis(200)), None);
        assert!(start.elapsed() < Duration::from_secs(2));
    }
}
