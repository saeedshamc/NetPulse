//! Linux network monitor: /proc for interfaces, inet_diag netlink for per-process TCP bytes.
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::monitor::types::{AppTraffic, InterfaceTraffic, InterfaceType, Protocol};
use crate::monitor::NetworkMonitor;

#[derive(Clone, Default)]
struct Counters {
    sent: u64,
    recv: u64,
}

pub struct LinuxMonitor {
    prev_apps: HashMap<u32, Counters>,
    session_apps: HashMap<u32, Counters>,
    prev_ifaces: HashMap<u32, Counters>,
    last_poll: Option<Instant>,
    privilege_note: Option<String>,
}

impl LinuxMonitor {
    pub fn new() -> Self {
        Self {
            prev_apps: HashMap::new(),
            session_apps: HashMap::new(),
            prev_ifaces: HashMap::new(),
            last_poll: None,
            privilege_note: Some(
                "Linux per-app totals use inet_diag (SOCK_DIAG) TCP byte counters. \
                 Full visibility for other users' sockets may require elevated privileges. \
                 eBPF accounting is planned as a later upgrade."
                    .into(),
            ),
        }
    }
}

impl NetworkMonitor for LinuxMonitor {
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
        let privilege_note = self.privilege_note.clone();
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

impl LinuxMonitor {
    fn poll_apps(&mut self, elapsed_secs: f64, first: bool) -> Result<Vec<AppTraffic>, String> {
        let current = collect_app_bytes()?;
        let mut apps = Vec::new();

        for (pid, counters) in &current {
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

            let (name, path) = process_info(*pid);
            apps.push(AppTraffic {
                pid: *pid,
                name,
                executable_path: path,
                bytes_sent: session.sent,
                bytes_received: session.recv,
                bytes_sent_rate: (sent_delta as f64 / elapsed_secs) as u64,
                bytes_received_rate: (recv_delta as f64 / elapsed_secs) as u64,
                protocol: Protocol::Tcp,
            });
        }

        self.prev_apps = current;
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

fn collect_interface_bytes() -> Result<HashMap<u32, (String, InterfaceType, u64, u64)>, String> {
    let data = fs::read_to_string("/proc/net/dev").map_err(|e| format!("/proc/net/dev: {e}"))?;
    let mut map = HashMap::new();
    let mut index = 1u32;

    for line in data.lines().skip(2) {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Some((name_part, rest)) = line.split_once(':') else {
            continue;
        };
        let name = name_part.trim().to_string();
        if name == "lo" {
            index += 1;
            continue;
        }
        let cols: Vec<&str> = rest.split_whitespace().collect();
        if cols.len() < 9 {
            continue;
        }
        let recv: u64 = cols[0].parse().unwrap_or(0);
        let sent: u64 = cols[8].parse().unwrap_or(0);
        let iface_type = classify_linux_iface(&name);
        map.insert(index, (name, iface_type, sent, recv));
        index += 1;
    }
    Ok(map)
}

fn classify_linux_iface(name: &str) -> InterfaceType {
    let n = name.to_lowercase();
    if n.starts_with("wl") || n.starts_with("wlan") || n.starts_with("wifi") {
        InterfaceType::Wifi
    } else if n.starts_with("eth")
        || n.starts_with("en")
        || n.starts_with("em")
        || n.starts_with("eno")
        || n.starts_with("ens")
        || n.starts_with("enp")
    {
        InterfaceType::Ethernet
    } else if n.starts_with("tun")
        || n.starts_with("tap")
        || n.starts_with("wg")
        || n.starts_with("vpn")
        || n.contains("wireguard")
    {
        InterfaceType::Vpn
    } else if n.starts_with("bnep") || n.starts_with("bt") {
        InterfaceType::Bluetooth
    } else if n.starts_with("ap") || n.contains("hotspot") {
        InterfaceType::Hotspot
    } else {
        InterfaceType::Other
    }
}

fn collect_app_bytes() -> Result<HashMap<u32, Counters>, String> {
    // inode -> counters from inet_diag (TCP). UDP endpoints listed without bytes.
    let mut inode_bytes = inet_diag::dump_tcp_bytes().unwrap_or_default();
    let udp_inodes = parse_proc_socket_inodes("/proc/net/udp");
    let udp6_inodes = parse_proc_socket_inodes("/proc/net/udp6");
    for inode in udp_inodes.into_iter().chain(udp6_inodes) {
        inode_bytes.entry(inode).or_default();
    }

    let mut by_pid: HashMap<u32, Counters> = HashMap::new();
    let proc = fs::read_dir("/proc").map_err(|e| format!("/proc: {e}"))?;
    for entry in proc.flatten() {
        let name = entry.file_name();
        let name = name.to_string_lossy();
        let Ok(pid) = name.parse::<u32>() else {
            continue;
        };
        let fd_dir = entry.path().join("fd");
        let Ok(fds) = fs::read_dir(&fd_dir) else {
            continue;
        };
        for fd in fds.flatten() {
            let Ok(link) = fs::read_link(fd.path()) else {
                continue;
            };
            let link = link.to_string_lossy();
            if let Some(inode) = parse_socket_inode(&link) {
                if let Some(c) = inode_bytes.get(&inode) {
                    let entry = by_pid.entry(pid).or_default();
                    entry.sent = entry.sent.saturating_add(c.sent);
                    entry.recv = entry.recv.saturating_add(c.recv);
                }
            }
        }
    }
    Ok(by_pid)
}

fn parse_socket_inode(link: &str) -> Option<u64> {
    // socket:[12345]
    let start = link.find('[')? + 1;
    let end = link.find(']')?;
    link[start..end].parse().ok()
}

fn parse_proc_socket_inodes(path: &str) -> Vec<u64> {
    let Ok(data) = fs::read_to_string(path) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for line in data.lines().skip(1) {
        let cols: Vec<&str> = line.split_whitespace().collect();
        // inode is typically column 9 (0-based index 9)
        if cols.len() > 9 {
            if let Ok(inode) = cols[9].parse::<u64>() {
                out.push(inode);
            }
        }
    }
    out
}

fn process_info(pid: u32) -> (String, Option<String>) {
    let cmdline = fs::read_to_string(format!("/proc/{pid}/cmdline")).unwrap_or_default();
    let exe = fs::read_link(format!("/proc/{pid}/exe"))
        .ok()
        .map(|p| p.to_string_lossy().to_string());
    let name = if !cmdline.is_empty() {
        cmdline
            .split('\0')
            .next()
            .and_then(|s| Path::new(s).file_name().map(|f| f.to_string_lossy().to_string()))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| format!("pid:{pid}"))
    } else {
        exe.as_ref()
            .and_then(|p| Path::new(p).file_name().map(|f| f.to_string_lossy().to_string()))
            .unwrap_or_else(|| format!("pid:{pid}"))
    };
    (name, exe)
}

fn read_current_ssid() -> Result<Option<String>, String> {
    if let Ok(out) = std::process::Command::new("iwgetid").arg("-r").output() {
        if out.status.success() {
            let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !s.is_empty() {
                return Ok(Some(s));
            }
        }
    }
    if let Ok(out) = std::process::Command::new("nmcli")
        .args(["-t", "-f", "ACTIVE,SSID", "dev", "wifi"])
        .output()
    {
        if out.status.success() {
            for line in String::from_utf8_lossy(&out.stdout).lines() {
                if let Some(ssid) = line.strip_prefix("yes:") {
                    let ssid = ssid.trim();
                    if !ssid.is_empty() {
                        return Ok(Some(ssid.to_string()));
                    }
                }
            }
        }
    }
    Ok(None)
}

mod inet_diag {
    use super::Counters;
    use std::collections::HashMap;
    use std::io::ErrorKind;
    use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};

    const NETLINK_SOCK_DIAG: i32 = 4;
    const AF_INET: u8 = 2;
    const AF_INET6: u8 = 10;
    const IPPROTO_TCP: u8 = 6;
    const SOCK_DIAG_BY_FAMILY: u16 = 20;
    const INET_DIAG_INFO: u8 = 2;
    const NLM_F_REQUEST: u16 = 0x01;
    const NLM_F_DUMP: u16 = 0x300;
    const NLMSG_DONE: u16 = 0x3;
    const NLMSG_ERROR: u16 = 0x2;
    const TCPF_ALL: u32 = 0xFFF;

    #[repr(C)]
    struct NlMsgHdr {
        nlmsg_len: u32,
        nlmsg_type: u16,
        nlmsg_flags: u16,
        nlmsg_seq: u32,
        nlmsg_pid: u32,
    }

    #[repr(C)]
    struct InetDiagSockId {
        idiag_sport: u16,
        idiag_dport: u16,
        idiag_src: [u32; 4],
        idiag_dst: [u32; 4],
        idiag_if: u32,
        idiag_cookie: [u32; 2],
    }

    #[repr(C)]
    struct InetDiagReqV2 {
        sdiag_family: u8,
        sdiag_protocol: u8,
        idiag_ext: u8,
        pad: u8,
        idiag_states: u32,
        id: InetDiagSockId,
    }

    #[repr(C)]
    struct InetDiagMsg {
        idiag_family: u8,
        idiag_state: u8,
        idiag_timer: u8,
        idiag_retrans: u8,
        id: InetDiagSockId,
        idiag_expires: u32,
        idiag_rqueue: u32,
        idiag_wqueue: u32,
        idiag_uid: u32,
        idiag_inode: u32,
    }

    #[repr(C)]
    struct NlAttr {
        nla_len: u16,
        nla_type: u16,
    }

    pub fn dump_tcp_bytes() -> Result<HashMap<u64, Counters>, String> {
        let mut map = HashMap::new();
        dump_family(AF_INET, &mut map)?;
        let _ = dump_family(AF_INET6, &mut map);
        Ok(map)
    }

    fn dump_family(family: u8, map: &mut HashMap<u64, Counters>) -> Result<(), String> {
        let fd = create_netlink_socket()?;
        let req = InetDiagReqV2 {
            sdiag_family: family,
            sdiag_protocol: IPPROTO_TCP,
            idiag_ext: 1 << (INET_DIAG_INFO - 1),
            pad: 0,
            idiag_states: TCPF_ALL,
            id: unsafe { std::mem::zeroed() },
        };

        let hdr = NlMsgHdr {
            nlmsg_len: (std::mem::size_of::<NlMsgHdr>() + std::mem::size_of::<InetDiagReqV2>())
                as u32,
            nlmsg_type: SOCK_DIAG_BY_FAMILY,
            nlmsg_flags: NLM_F_REQUEST | NLM_F_DUMP,
            nlmsg_seq: 1,
            nlmsg_pid: 0,
        };

        let mut buf = vec![0u8; hdr.nlmsg_len as usize];
        unsafe {
            std::ptr::copy_nonoverlapping(
                &hdr as *const _ as *const u8,
                buf.as_mut_ptr(),
                std::mem::size_of::<NlMsgHdr>(),
            );
            std::ptr::copy_nonoverlapping(
                &req as *const _ as *const u8,
                buf.as_mut_ptr().add(std::mem::size_of::<NlMsgHdr>()),
                std::mem::size_of::<InetDiagReqV2>(),
            );
        }

        let raw = fd.as_raw_fd();
        let written = unsafe { libc::send(raw, buf.as_ptr() as *const _, buf.len(), 0) };
        if written < 0 {
            return Err(format!(
                "netlink send failed: {}",
                std::io::Error::last_os_error()
            ));
        }

        let mut recv_buf = vec![0u8; 65536];
        loop {
            let n =
                unsafe { libc::recv(raw, recv_buf.as_mut_ptr() as *mut _, recv_buf.len(), 0) };
            if n < 0 {
                let err = std::io::Error::last_os_error();
                if err.kind() == ErrorKind::WouldBlock || err.kind() == ErrorKind::TimedOut {
                    break;
                }
                return Err(format!("netlink recv failed: {err}"));
            }
            if n == 0 {
                break;
            }
            let n = n as usize;
            let mut offset = 0usize;
            while offset + std::mem::size_of::<NlMsgHdr>() <= n {
                let nh = unsafe { &*(recv_buf.as_ptr().add(offset) as *const NlMsgHdr) };
                let len = nh.nlmsg_len as usize;
                if len < std::mem::size_of::<NlMsgHdr>() || offset + len > n {
                    break;
                }
                if nh.nlmsg_type == NLMSG_DONE {
                    return Ok(());
                }
                if nh.nlmsg_type == NLMSG_ERROR {
                    return Err("netlink error response".into());
                }
                if nh.nlmsg_type == SOCK_DIAG_BY_FAMILY {
                    parse_diag_msg(
                        &recv_buf[offset + std::mem::size_of::<NlMsgHdr>()..offset + len],
                        map,
                    );
                }
                offset += (len + 3) & !3;
            }
        }
        Ok(())
    }

    fn parse_diag_msg(payload: &[u8], map: &mut HashMap<u64, Counters>) {
        if payload.len() < std::mem::size_of::<InetDiagMsg>() {
            return;
        }
        let msg = unsafe { &*(payload.as_ptr() as *const InetDiagMsg) };
        let inode = msg.idiag_inode as u64;
        let mut sent = 0u64;
        let mut recv = 0u64;

        let mut attr_off = std::mem::size_of::<InetDiagMsg>();
        while attr_off + std::mem::size_of::<NlAttr>() <= payload.len() {
            let attr = unsafe { &*(payload.as_ptr().add(attr_off) as *const NlAttr) };
            let alen = attr.nla_len as usize;
            if alen < std::mem::size_of::<NlAttr>() || attr_off + alen > payload.len() {
                break;
            }
            if (attr.nla_type & 0x3FFF) == INET_DIAG_INFO as u16 {
                let data_off = attr_off + std::mem::size_of::<NlAttr>();
                let data_len = alen - std::mem::size_of::<NlAttr>();
                if data_len >= 104 {
                    sent = u64::from_ne_bytes(
                        payload[data_off + 88..data_off + 96]
                            .try_into()
                            .unwrap_or([0; 8]),
                    );
                    recv = u64::from_ne_bytes(
                        payload[data_off + 96..data_off + 104]
                            .try_into()
                            .unwrap_or([0; 8]),
                    );
                }
            }
            attr_off += (alen + 3) & !3;
        }

        let entry = map.entry(inode).or_default();
        entry.sent = entry.sent.saturating_add(sent);
        entry.recv = entry.recv.saturating_add(recv);
    }

    fn create_netlink_socket() -> Result<OwnedFd, String> {
        let fd = unsafe { libc::socket(libc::AF_NETLINK, libc::SOCK_DGRAM, NETLINK_SOCK_DIAG) };
        if fd < 0 {
            return Err(format!(
                "socket(AF_NETLINK): {}",
                std::io::Error::last_os_error()
            ));
        }
        let tv = libc::timeval {
            tv_sec: 1,
            tv_usec: 0,
        };
        unsafe {
            libc::setsockopt(
                fd,
                libc::SOL_SOCKET,
                libc::SO_RCVTIMEO,
                &tv as *const _ as *const _,
                std::mem::size_of_val(&tv) as u32,
            );
        }
        Ok(unsafe { OwnedFd::from_raw_fd(fd) })
    }
}
