//! macOS network monitor: nettop for per-process bytes, getifaddrs for interfaces.
use std::collections::HashMap;
use std::process::Command;
use std::time::{Duration, Instant};

use crate::monitor::types::{AppTraffic, InterfaceTraffic, InterfaceType, Protocol};
use crate::monitor::NetworkMonitor;

#[derive(Clone, Default)]
struct Counters {
    sent: u64,
    recv: u64,
}

pub struct MacMonitor {
    prev_apps: HashMap<u32, Counters>,
    session_apps: HashMap<u32, Counters>,
    prev_ifaces: HashMap<u32, Counters>,
    last_poll: Option<Instant>,
    privilege_note: Option<String>,
    nettop_ok: bool,
}

impl MacMonitor {
    pub fn new() -> Self {
        Self {
            prev_apps: HashMap::new(),
            session_apps: HashMap::new(),
            prev_ifaces: HashMap::new(),
            last_poll: None,
            privilege_note: Some(
                "macOS per-app traffic is collected via the nettop CLI. \
                 Elevated privileges (sudo / admin) are usually required for complete results. \
                 A Network Extension entitlement is the longer-term path for App Store builds."
                    .into(),
            ),
            nettop_ok: true,
        }
    }
}

impl NetworkMonitor for MacMonitor {
    fn poll_snapshot(&mut self) -> Result<crate::monitor::MonitorSnapshot, String> {
        let elapsed = self
            .last_poll
            .map(|t| t.elapsed())
            .unwrap_or(Duration::from_secs(1));
        let elapsed_secs = elapsed.as_secs_f64().max(0.5);
        let first = self.last_poll.is_none();

        let apps = self.poll_apps(elapsed_secs, first)?;
        let interfaces = self.poll_ifaces(elapsed_secs, first)?;
        let current_ssid = read_current_ssid().unwrap_or(None);
        let mut privilege_note = self.privilege_note.clone();
        if !self.nettop_ok {
            privilege_note = Some(
                "nettop failed or returned no data. Run NetPulse with elevated privileges \
                 to enable per-application traffic on macOS."
                    .into(),
            );
        }
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        self.last_poll = Some(Instant::now());

        Ok(crate::monitor::MonitorSnapshot {
            timestamp_ms,
            apps,
            interfaces,
            current_ssid,
            privilege_note,
        })
    }

    fn poll_per_app_traffic(&mut self) -> Result<Vec<AppTraffic>, String> {
        Ok(self.poll_snapshot()?.apps)
    }

    fn poll_per_interface_traffic(&mut self) -> Result<Vec<InterfaceTraffic>, String> {
        Ok(self.poll_snapshot()?.interfaces)
    }

    fn current_ssid(&mut self) -> Result<Option<String>, String> {
        read_current_ssid()
    }

    fn privilege_note(&self) -> Option<String> {
        self.privilege_note.clone()
    }
}

impl MacMonitor {
    fn poll_apps(&mut self, elapsed_secs: f64, first: bool) -> Result<Vec<AppTraffic>, String> {
        let current = match collect_app_bytes_nettop() {
            Ok(map) => {
                self.nettop_ok = true;
                map
            }
            Err(_) => {
                self.nettop_ok = false;
                HashMap::new()
            }
        };
        let mut apps = Vec::new();

        for (pid, (name, counters)) in &current {
            let prev = self.prev_apps.get(pid).cloned().unwrap_or_default();
            let sent_delta = if first {
                0
            } else {
                counters.sent.saturating_sub(prev.sent)
            };
            let recv_delta = if first {
                0
            } else {
                counters.recv.saturating_sub(prev.recv)
            };

            let session = self.session_apps.entry(*pid).or_default();
            session.sent = session.sent.saturating_add(sent_delta);
            session.recv = session.recv.saturating_add(recv_delta);

            apps.push(AppTraffic {
                pid: *pid,
                name: name.clone(),
                executable_path: None,
                bytes_sent: session.sent,
                bytes_received: session.recv,
                bytes_sent_rate: (sent_delta as f64 / elapsed_secs) as u64,
                bytes_received_rate: (recv_delta as f64 / elapsed_secs) as u64,
                protocol: Protocol::Mixed,
            });
        }

        self.prev_apps = current
            .into_iter()
            .map(|(pid, (_n, c))| (pid, c))
            .collect();

        apps.sort_by(|a, b| {
            let ta = a.bytes_sent_rate + a.bytes_received_rate;
            let tb = b.bytes_sent_rate + b.bytes_received_rate;
            tb.cmp(&ta).then_with(|| {
                (b.bytes_sent + b.bytes_received).cmp(&(a.bytes_sent + a.bytes_received))
            })
        });
        Ok(apps)
    }

    fn poll_ifaces(
        &mut self,
        elapsed_secs: f64,
        first: bool,
    ) -> Result<Vec<InterfaceTraffic>, String> {
        let ssid = read_current_ssid().unwrap_or(None);
        let current = collect_interface_bytes()?;
        let mut list = Vec::new();

        for (index, (name, iface_type, sent, recv)) in &current {
            let prev = self.prev_ifaces.get(index).cloned().unwrap_or_default();
            let sent_delta = if first {
                0
            } else {
                sent.saturating_sub(prev.sent)
            };
            let recv_delta = if first {
                0
            } else {
                recv.saturating_sub(prev.recv)
            };

            list.push(InterfaceTraffic {
                name: name.clone(),
                interface_type: *iface_type,
                index: *index,
                bytes_sent: *sent,
                bytes_received: *recv,
                bytes_sent_rate: (sent_delta as f64 / elapsed_secs) as u64,
                bytes_received_rate: (recv_delta as f64 / elapsed_secs) as u64,
                ssid: if *iface_type == InterfaceType::Wifi {
                    ssid.clone()
                } else {
                    None
                },
            });
        }

        self.prev_ifaces = current
            .into_iter()
            .map(|(idx, (_n, _t, s, r))| (idx, Counters { sent: s, recv: r }))
            .collect();

        list.sort_by(|a, b| {
            (b.bytes_sent_rate + b.bytes_received_rate)
                .cmp(&(a.bytes_sent_rate + a.bytes_received_rate))
        });
        Ok(list)
    }
}

fn collect_app_bytes_nettop() -> Result<HashMap<u32, (String, Counters)>, String> {
    // One sample line per process; -L 1 prints a single sample then exits on recent macOS.
    let output = Command::new("nettop")
        .args([
            "-P", "-x", "-L", "1", "-J", "bytes_in,bytes_out",
        ])
        .output()
        .map_err(|e| format!("failed to spawn nettop: {e}"))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(format!("nettop failed: {err}"));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut map: HashMap<u32, (String, Counters)> = HashMap::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with(',') || line.contains("time,") {
            continue;
        }
        // Typical: process.pid,bytes_in,bytes_out  OR  name.12345,in,out
        let cols: Vec<&str> = line.split(',').map(|s| s.trim()).collect();
        if cols.len() < 3 {
            continue;
        }
        let proc_field = cols[0];
        let (name, pid) = parse_nettop_process(proc_field);
        let Some(pid) = pid else { continue };
        let recv: u64 = cols[cols.len() - 2].replace('"', "").parse().unwrap_or(0);
        let sent: u64 = cols[cols.len() - 1].replace('"', "").parse().unwrap_or(0);
        let entry = map.entry(pid).or_insert_with(|| (name.clone(), Counters::default()));
        entry.0 = name;
        entry.1.sent = entry.1.sent.saturating_add(sent);
        entry.1.recv = entry.1.recv.saturating_add(recv);
    }

    if map.is_empty() {
        // Fallback format: whitespace columns from `nettop -P -L 1`
        let output = Command::new("nettop")
            .args(["-P", "-L", "1"])
            .output()
            .map_err(|e| format!("failed to spawn nettop: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(1) {
            let cols: Vec<&str> = line.split_whitespace().collect();
            if cols.len() < 3 {
                continue;
            }
            let (name, pid) = parse_nettop_process(cols[0]);
            let Some(pid) = pid else { continue };
            // Heuristic: last two numeric columns often bytes out/in depending on version
            let nums: Vec<u64> = cols
                .iter()
                .filter_map(|c| c.replace(',', "").parse::<u64>().ok())
                .collect();
            if nums.len() < 2 {
                continue;
            }
            let recv = nums[nums.len() - 2];
            let sent = nums[nums.len() - 1];
            map.insert(pid, (name, Counters { sent, recv }));
        }
    }

    Ok(map)
}

fn parse_nettop_process(field: &str) -> (String, Option<u32>) {
    let field = field.trim().trim_matches('"');
    if let Some((name, pid_str)) = field.rsplit_once('.') {
        if let Ok(pid) = pid_str.parse::<u32>() {
            return (name.to_string(), Some(pid));
        }
    }
    if let Some((name, pid_str)) = field.rsplit_once(':') {
        if let Ok(pid) = pid_str.parse::<u32>() {
            return (name.to_string(), Some(pid));
        }
    }
    (field.to_string(), None)
}

fn collect_interface_bytes() -> Result<HashMap<u32, (String, InterfaceType, u64, u64)>, String> {
    unsafe {
        let mut ifap: *mut libc::ifaddrs = std::ptr::null_mut();
        if libc::getifaddrs(&mut ifap) != 0 {
            return Err(format!(
                "getifaddrs failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        let mut map: HashMap<u32, (String, InterfaceType, u64, u64)> = HashMap::new();
        let mut idx = 1u32;
        let mut cursor = ifap;
        while !cursor.is_null() {
            let ifa = &*cursor;
            if !ifa.ifa_addr.is_null() && (*ifa.ifa_addr).sa_family as i32 == libc::AF_LINK {
                let name = std::ffi::CStr::from_ptr(ifa.ifa_name)
                    .to_string_lossy()
                    .to_string();
                if name == "lo0" || name.starts_with("lo") {
                    cursor = ifa.ifa_next;
                    continue;
                }
                // On macOS, AF_LINK ifa_data points to if_data
                let mut sent = 0u64;
                let mut recv = 0u64;
                if !ifa.ifa_data.is_null() {
                    let data = &*(ifa.ifa_data as *const libc::if_data);
                    sent = data.ifi_obytes as u64;
                    recv = data.ifi_ibytes as u64;
                }
                let iface_type = classify_macos_iface(&name);
                map.insert(idx, (name, iface_type, sent, recv));
                idx += 1;
            }
            cursor = ifa.ifa_next;
        }
        libc::freeifaddrs(ifap);
        Ok(map)
    }
}

fn classify_macos_iface(name: &str) -> InterfaceType {
    let n = name.to_lowercase();
    if n.starts_with("en") {
        // en0 is often Wi-Fi on laptops; refined via SSID presence elsewhere
        InterfaceType::Wifi
    } else if n.starts_with("eth") {
        InterfaceType::Ethernet
    } else if n.starts_with("utun") || n.starts_with("ipsec") || n.starts_with("ppp") {
        InterfaceType::Vpn
    } else if n.starts_with("awdl") || n.starts_with("llw") {
        InterfaceType::Wifi
    } else if n.starts_with("bridge") {
        InterfaceType::Hotspot
    } else {
        InterfaceType::Other
    }
}

fn read_current_ssid() -> Result<Option<String>, String> {
    // Prefer networksetup / ipconfig summary; airport binary path varies by macOS version.
    if let Ok(out) = Command::new("ipconfig").args(["getsummary", "en0"]).output() {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("SSID :") {
                    let ssid = rest.trim();
                    if !ssid.is_empty() {
                        return Ok(Some(ssid.to_string()));
                    }
                }
                if let Some(rest) = line
                    .strip_prefix("SSID:")
                    .or_else(|| line.strip_prefix("  SSID :"))
                {
                    let ssid = rest.trim();
                    if !ssid.is_empty() {
                        return Ok(Some(ssid.to_string()));
                    }
                }
            }
        }
    }

    let airport = "/System/Library/PrivateFrameworks/Apple80211.framework/Versions/Current/Resources/airport";
    if let Ok(out) = Command::new(airport).arg("-I").output() {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                let line = line.trim();
                if let Some(rest) = line.strip_prefix("SSID:") {
                    let ssid = rest.trim();
                    if !ssid.is_empty() {
                        return Ok(Some(ssid.to_string()));
                    }
                }
            }
        }
    }
    Ok(None)
}
