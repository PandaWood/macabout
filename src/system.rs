//! Reading this computer's facts: `/etc/os-release`, `/proc`, sysfs, `lspci` and udev, all
//! readable without root. Or, with `--mock`, a made-up computer.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::parse::{self, Gpu, MemoryDetails, OsRelease};

/// Everything the summary is made from. `None` where a source was missing or said nothing.
pub struct SystemInfo {
    pub os: OsRelease,
    /// The CPU model as `/proc/cpuinfo` names it.
    pub cpu: Option<String>,
    pub cpu_cores: Option<u32>,
    pub memory: Option<Memory>,
    pub gpu: Option<Gpu>,
    /// The graphics card's own memory, where the driver reports it in sysfs.
    pub vram_mb: Option<u64>,
    /// The product name, such as "MacBook Air (13-inch, Mid 2013)".
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

/// A command's stdout, `None` if it is missing, fails, or is still running after `timeout`.
pub fn run_command(command: &mut Command, timeout: Duration) -> Option<String> {
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdout = child.stdout.take()?;
    let reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.read_to_end(&mut out);
        out
    });
    let deadline = Instant::now() + timeout;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if Instant::now() < deadline => std::thread::sleep(Duration::from_millis(5)),
            _ => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };
    let out = reader.join().ok()?;
    status.success().then(|| text(&out))
}

fn run(program: &str, args: &[&str]) -> Option<String> {
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

/// `mem_info_vram_total` of each card, in `/sys/class/drm` directory order.
fn sysfs_vram_mb() -> Option<u64> {
    let contents: Vec<String> = std::fs::read_dir("/sys/class/drm")
        .ok()?
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().starts_with("card"))
        .filter_map(|e| read_text(e.path().join("device/mem_info_vram_total")))
        .collect();
    parse::vram_mb(&contents)
}

fn this_computer() -> SystemInfo {
    let cpuinfo = read_text("/proc/cpuinfo").unwrap_or_default();
    let gpu = run("lspci", &["-nn"]).and_then(|out| parse::lspci_gpu(&out));
    SystemInfo {
        os: read_text("/etc/os-release")
            .map(|text| parse::os_release(&text))
            .unwrap_or_default(),
        cpu: parse::cpu_model(&cpuinfo),
        cpu_cores: parse::cpu_cores(&cpuinfo),
        memory: memory(),
        vram_mb: gpu.as_ref().and_then(|_| sysfs_vram_mb()),
        gpu,
        model: read_text("/sys/class/dmi/id/product_name")
            .and_then(|name| parse::dmi_value(&name, "system product name")),
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
            pci_id: "8086:0a26".to_string(),
        }),
        vram_mb: None,
        model: Some("MacBook Air (13-inch, Mid 2013)".to_string()),
    }
}

pub fn collect(mock_data: bool) -> SystemInfo {
    if mock_data { mock() } else { this_computer() }
}
