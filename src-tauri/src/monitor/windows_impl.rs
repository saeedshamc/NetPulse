//! Windows network monitor using minimal FFI to IP Helper / WLAN APIs.
use std::collections::HashMap;
use std::time::{Duration, Instant};

use crate::monitor::types::{AppTraffic, InterfaceTraffic, InterfaceType, Protocol};
use crate::monitor::NetworkMonitor;

#[derive(Clone, Default)]
struct Counters {
    sent: u64,
    recv: u64,
}

pub struct WindowsMonitor {
    prev_apps: HashMap<u32, Counters>,
    session_apps: HashMap<u32, Counters>,
    prev_ifaces: HashMap<u32, Counters>,
    last_poll: Option<Instant>,
    privilege_note: Option<String>,
}

impl WindowsMonitor {
    pub fn new() -> Self {
        Self {
            prev_apps: HashMap::new(),
            session_apps: HashMap::new(),
            prev_ifaces: HashMap::new(),
            last_poll: None,
            privilege_note: Some(
                "Per-app byte counts use TCP connection statistics (ESTATS). \
                 Some processes may require Administrator rights for complete visibility. \
                 UDP endpoints are listed when present but byte counts are TCP-based."
                    .into(),
            ),
        }
    }

    fn poll_apps(&mut self, elapsed_secs: f64, first: bool) -> Result<Vec<AppTraffic>, String> {
        let current = ffi::collect_app_bytes()?;
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

            let (name, path) = ffi::process_info(*pid);
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
        let ssid = ffi::read_current_ssid().unwrap_or(None);
        let current = ffi::collect_interface_bytes()?;
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

impl NetworkMonitor for WindowsMonitor {
    fn poll_snapshot(&mut self) -> Result<crate::monitor::MonitorSnapshot, String> {
        let elapsed = self
            .last_poll
            .map(|t| t.elapsed())
            .unwrap_or(Duration::from_secs(1));
        let elapsed_secs = elapsed.as_secs_f64().max(0.5);
        let first = self.last_poll.is_none();

        let apps = self.poll_apps(elapsed_secs, first)?;
        let interfaces = self.poll_ifaces(elapsed_secs, first)?;
        let current_ssid = ffi::read_current_ssid().unwrap_or(None);
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
        ffi::read_current_ssid()
    }

    fn privilege_note(&self) -> Option<String> {
        self.privilege_note.clone()
    }
}

mod ffi {
    use super::Counters;
    use crate::monitor::InterfaceType;
    use std::collections::HashMap;
    use std::ffi::c_void;

    const AF_INET: u32 = 2;
    const TCP_TABLE_OWNER_PID_CONNECTIONS: u32 = 5;
    const UDP_TABLE_OWNER_PID: u32 = 1;
    const ERROR_INSUFFICIENT_BUFFER: u32 = 122;
    const TCP_CONNECTION_ESTATS_DATA: u32 = 1;
    const MAX_INTERFACE_NAME_LEN: usize = 256;
    const MAXLEN_IFDESCR: usize = 256;
    const MAXLEN_PHYSADDR: usize = 8;
    const IF_TYPE_SOFTWARE_LOOPBACK: u32 = 24;
    const IF_TYPE_ETHERNET_CSMACD: u32 = 6;
    const IF_TYPE_IEEE80211: u32 = 71;
    const IF_TYPE_PPP: u32 = 23;
    const IF_TYPE_TUNNEL: u32 = 131;
    const IF_TYPE_PROP_VIRTUAL: u32 = 53;
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const WLAN_INTF_OPCODE_CURRENT_CONNECTION: u32 = 7;

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct MibTcpRowOwnerPid {
        state: u32,
        local_addr: u32,
        local_port: u32,
        remote_addr: u32,
        remote_port: u32,
        owning_pid: u32,
    }

    #[repr(C)]
    struct MibTcpTableOwnerPid {
        num_entries: u32,
        table: [MibTcpRowOwnerPid; 1],
    }

    #[repr(C)]
    #[derive(Clone, Copy)]
    struct MibUdpRowOwnerPid {
        local_addr: u32,
        local_port: u32,
        owning_pid: u32,
    }

    #[repr(C)]
    struct MibUdpTableOwnerPid {
        num_entries: u32,
        table: [MibUdpRowOwnerPid; 1],
    }

    #[repr(C)]
    struct TcpEstatsDataRw {
        enable_collection: u8,
    }

    #[repr(C)]
    #[derive(Default)]
    struct TcpEstatsDataRod {
        data_bytes_out: u64,
        data_bytes_in: u64,
        data_segs_out: u64,
        data_segs_in: u64,
        soft_errors: u32,
        data_in_cwin: u32,
        _pad: [u64; 6],
    }

    #[repr(C)]
    struct MibIfRow {
        wsz_name: [u16; MAX_INTERFACE_NAME_LEN],
        dw_index: u32,
        dw_type: u32,
        dw_mtu: u32,
        dw_speed: u32,
        dw_phys_addr_len: u32,
        b_phys_addr: [u8; MAXLEN_PHYSADDR],
        dw_admin_status: u32,
        dw_oper_status: u32,
        dw_last_change: u32,
        dw_in_octets: u32,
        dw_in_ucast_pkts: u32,
        dw_in_nucast_pkts: u32,
        dw_in_discards: u32,
        dw_in_errors: u32,
        dw_in_unknown_protos: u32,
        dw_out_octets: u32,
        dw_out_ucast_pkts: u32,
        dw_out_nucast_pkts: u32,
        dw_out_discards: u32,
        dw_out_errors: u32,
        dw_out_qlen: u32,
        dw_descr_len: u32,
        b_descr: [u8; MAXLEN_IFDESCR],
    }

    #[repr(C)]
    struct MibIfTable {
        dw_num_entries: u32,
        table: [MibIfRow; 1],
    }

    #[link(name = "iphlpapi")]
    unsafe extern "system" {
        fn GetExtendedTcpTable(
            table: *mut c_void,
            size: *mut u32,
            order: i32,
            af: u32,
            table_class: u32,
            reserved: u32,
        ) -> u32;
        fn GetExtendedUdpTable(
            table: *mut c_void,
            size: *mut u32,
            order: i32,
            af: u32,
            table_class: u32,
            reserved: u32,
        ) -> u32;
        fn SetPerTcpConnectionEStats(
            row: *mut MibTcpRowOwnerPid,
            estats_type: u32,
            rw: *const u8,
            rw_version: u32,
            rw_size: u32,
            offset: u32,
        ) -> u32;
        fn GetPerTcpConnectionEStats(
            row: *mut MibTcpRowOwnerPid,
            estats_type: u32,
            rw: *mut u8,
            rw_version: u32,
            rw_size: u32,
            ros: *mut u8,
            ros_version: u32,
            ros_size: u32,
            rod: *mut u8,
            rod_version: u32,
            rod_size: u32,
        ) -> u32;
        fn GetIfTable(table: *mut c_void, size: *mut u32, order: i32) -> u32;
    }

    #[link(name = "wlanapi")]
    unsafe extern "system" {
        fn WlanOpenHandle(
            client_version: u32,
            reserved: *mut c_void,
            negotiated_version: *mut u32,
            client_handle: *mut *mut c_void,
        ) -> u32;
        fn WlanCloseHandle(client_handle: *mut c_void, reserved: *mut c_void) -> u32;
        fn WlanEnumInterfaces(
            client_handle: *mut c_void,
            reserved: *mut c_void,
            interface_list: *mut *mut WlanInterfaceInfoList,
        ) -> u32;
        fn WlanQueryInterface(
            client_handle: *mut c_void,
            interface_guid: *const Guid,
            op_code: u32,
            reserved: *mut c_void,
            data_size: *mut u32,
            data: *mut *mut c_void,
            wlan_opcode_value_type: *mut u32,
        ) -> u32;
        fn WlanFreeMemory(memory: *mut c_void);
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
        fn CloseHandle(handle: *mut c_void) -> i32;
        fn QueryFullProcessImageNameW(
            process: *mut c_void,
            flags: u32,
            exe_name: *mut u16,
            size: *mut u32,
        ) -> i32;
    }

    #[repr(C)]
    struct Guid {
        data1: u32,
        data2: u16,
        data3: u16,
        data4: [u8; 8],
    }

    #[repr(C)]
    struct WlanInterfaceInfo {
        interface_guid: Guid,
        str_interface_description: [u16; 256],
        is_state: u32,
    }

    #[repr(C)]
    struct WlanInterfaceInfoList {
        number_of_items: u32,
        index: u32,
        interface_info: [WlanInterfaceInfo; 1],
    }

    #[repr(C)]
    struct Dot11Ssid {
        ssid_length: u32,
        ssid: [u8; 32],
    }

    #[repr(C)]
    struct WlanAssociationAttributes {
        dot11_ssid: Dot11Ssid,
        _pad: [u8; 80],
    }

    #[repr(C)]
    struct WlanConnectionAttributes {
        is_state: u32,
        wlan_connection_mode: u32,
        profile_name: [u16; 256],
        wlan_association_attributes: WlanAssociationAttributes,
    }

    pub fn collect_app_bytes() -> Result<HashMap<u32, Counters>, String> {
        unsafe {
            let mut size: u32 = 0;
            let status = GetExtendedTcpTable(
                std::ptr::null_mut(),
                &mut size,
                0,
                AF_INET,
                TCP_TABLE_OWNER_PID_CONNECTIONS,
                0,
            );
            if status != ERROR_INSUFFICIENT_BUFFER {
                return Err(format!("GetExtendedTcpTable size query failed: {status}"));
            }

            let mut buf = vec![0u8; size as usize];
            let status = GetExtendedTcpTable(
                buf.as_mut_ptr().cast(),
                &mut size,
                0,
                AF_INET,
                TCP_TABLE_OWNER_PID_CONNECTIONS,
                0,
            );
            if status != 0 {
                return Err(format!("GetExtendedTcpTable failed: {status}"));
            }

            let table = &*(buf.as_ptr() as *const MibTcpTableOwnerPid);
            let rows = std::slice::from_raw_parts(
                buf.as_ptr()
                    .add(std::mem::offset_of!(MibTcpTableOwnerPid, table))
                    as *const MibTcpRowOwnerPid,
                table.num_entries as usize,
            );

            let mut map: HashMap<u32, Counters> = HashMap::new();
            for row in rows {
                let pid = row.owning_pid;
                if pid == 0 {
                    continue;
                }
                let mut row_mut = *row;
                let rw = TcpEstatsDataRw {
                    enable_collection: 1,
                };
                let _ = SetPerTcpConnectionEStats(
                    &mut row_mut,
                    TCP_CONNECTION_ESTATS_DATA,
                    std::ptr::from_ref(&rw) as *const u8,
                    0,
                    std::mem::size_of::<TcpEstatsDataRw>() as u32,
                    0,
                );

                let mut rod = TcpEstatsDataRod::default();
                let get_status = GetPerTcpConnectionEStats(
                    &mut row_mut,
                    TCP_CONNECTION_ESTATS_DATA,
                    std::ptr::null_mut(),
                    0,
                    0,
                    std::ptr::null_mut(),
                    0,
                    0,
                    std::ptr::from_mut(&mut rod) as *mut u8,
                    0,
                    std::mem::size_of::<TcpEstatsDataRod>() as u32,
                );

                let entry = map.entry(pid).or_default();
                if get_status == 0 {
                    entry.sent = entry.sent.saturating_add(rod.data_bytes_out);
                    entry.recv = entry.recv.saturating_add(rod.data_bytes_in);
                }
            }

            collect_udp_pids(&mut map);
            Ok(map)
        }
    }

    fn collect_udp_pids(map: &mut HashMap<u32, Counters>) {
        unsafe {
            let mut size: u32 = 0;
            let status = GetExtendedUdpTable(
                std::ptr::null_mut(),
                &mut size,
                0,
                AF_INET,
                UDP_TABLE_OWNER_PID,
                0,
            );
            if status != ERROR_INSUFFICIENT_BUFFER {
                return;
            }
            let mut buf = vec![0u8; size as usize];
            let status = GetExtendedUdpTable(
                buf.as_mut_ptr().cast(),
                &mut size,
                0,
                AF_INET,
                UDP_TABLE_OWNER_PID,
                0,
            );
            if status != 0 {
                return;
            }
            let table = &*(buf.as_ptr() as *const MibUdpTableOwnerPid);
            let rows = std::slice::from_raw_parts(
                buf.as_ptr()
                    .add(std::mem::offset_of!(MibUdpTableOwnerPid, table))
                    as *const MibUdpRowOwnerPid,
                table.num_entries as usize,
            );
            for row in rows {
                if row.owning_pid != 0 {
                    map.entry(row.owning_pid).or_default();
                }
            }
        }
    }

    pub fn collect_interface_bytes() -> Result<HashMap<u32, (String, InterfaceType, u64, u64)>, String>
    {
        unsafe {
            let mut size: u32 = 0;
            let status = GetIfTable(std::ptr::null_mut(), &mut size, 0);
            if status != ERROR_INSUFFICIENT_BUFFER {
                return Err(format!("GetIfTable size query failed: {status}"));
            }
            let mut buf = vec![0u8; size as usize];
            let status = GetIfTable(buf.as_mut_ptr().cast(), &mut size, 0);
            if status != 0 {
                return Err(format!("GetIfTable failed: {status}"));
            }

            let table = &*(buf.as_ptr() as *const MibIfTable);
            let rows = std::slice::from_raw_parts(
                buf.as_ptr()
                    .add(std::mem::offset_of!(MibIfTable, table)) as *const MibIfRow,
                table.dw_num_entries as usize,
            );

            let mut map = HashMap::new();
            for row in rows {
                if row.dw_type == IF_TYPE_SOFTWARE_LOOPBACK {
                    continue;
                }
                let name = wide_to_string(&row.wsz_name);
                if name.is_empty() {
                    continue;
                }
                let descr_len = (row.dw_descr_len as usize).min(MAXLEN_IFDESCR);
                let description = String::from_utf8_lossy(&row.b_descr[..descr_len])
                    .trim_end_matches('\0')
                    .to_string();
                let iface_type = classify(&name, &description, row.dw_type);
                let sent = row.dw_out_octets as u64;
                let recv = row.dw_in_octets as u64;
                if sent == 0
                    && recv == 0
                    && !matches!(
                        iface_type,
                        InterfaceType::Wifi | InterfaceType::Ethernet | InterfaceType::Vpn
                    )
                {
                    continue;
                }
                map.insert(row.dw_index, (name, iface_type, sent, recv));
            }
            Ok(map)
        }
    }

    fn classify(alias: &str, description: &str, if_type: u32) -> InterfaceType {
        let desc = description.to_lowercase();
        let alias_l = alias.to_lowercase();
        if desc.contains("bluetooth") || alias_l.contains("bluetooth") {
            return InterfaceType::Bluetooth;
        }
        if desc.contains("vpn")
            || alias_l.contains("vpn")
            || desc.contains("wireguard")
            || desc.contains("tap-")
            || if_type == IF_TYPE_TUNNEL
            || if_type == IF_TYPE_PPP
            || if_type == IF_TYPE_PROP_VIRTUAL
        {
            return InterfaceType::Vpn;
        }
        if desc.contains("mobile hotspot") || desc.contains("hosted network") {
            return InterfaceType::Hotspot;
        }
        if if_type == IF_TYPE_IEEE80211 || desc.contains("wi-fi") || desc.contains("wireless") {
            return InterfaceType::Wifi;
        }
        if if_type == IF_TYPE_ETHERNET_CSMACD || desc.contains("ethernet") {
            return InterfaceType::Ethernet;
        }
        InterfaceType::Other
    }

    pub fn read_current_ssid() -> Result<Option<String>, String> {
        unsafe {
            let mut negotiated = 0u32;
            let mut client: *mut c_void = std::ptr::null_mut();
            if WlanOpenHandle(2, std::ptr::null_mut(), &mut negotiated, &mut client) != 0 {
                return Ok(None);
            }

            let mut list: *mut WlanInterfaceInfoList = std::ptr::null_mut();
            if WlanEnumInterfaces(client, std::ptr::null_mut(), &mut list) != 0 || list.is_null() {
                WlanCloseHandle(client, std::ptr::null_mut());
                return Ok(None);
            }

            let count = (*list).number_of_items as usize;
            let infos = std::slice::from_raw_parts(
                std::ptr::addr_of!((*list).interface_info) as *const WlanInterfaceInfo,
                count,
            );

            let mut ssid = None;
            for info in infos {
                let mut data_size = 0u32;
                let mut data: *mut c_void = std::ptr::null_mut();
                let mut op_type = 0u32;
                let query = WlanQueryInterface(
                    client,
                    &info.interface_guid,
                    WLAN_INTF_OPCODE_CURRENT_CONNECTION,
                    std::ptr::null_mut(),
                    &mut data_size,
                    &mut data,
                    &mut op_type,
                );
                if query == 0 && !data.is_null() {
                    let attrs = &*(data as *const WlanConnectionAttributes);
                    let len = attrs.wlan_association_attributes.dot11_ssid.ssid_length as usize;
                    if len > 0 && len <= 32 {
                        let bytes = &attrs.wlan_association_attributes.dot11_ssid.ssid[..len];
                        ssid = Some(String::from_utf8_lossy(bytes).to_string());
                    }
                    WlanFreeMemory(data);
                    if ssid.is_some() {
                        break;
                    }
                }
            }

            WlanFreeMemory(list as *mut c_void);
            WlanCloseHandle(client, std::ptr::null_mut());
            Ok(ssid)
        }
    }

    pub fn process_info(pid: u32) -> (String, Option<String>) {
        unsafe {
            let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if handle.is_null() {
                return (format!("pid:{pid}"), None);
            }
            let mut buf = vec![0u16; 1024];
            let mut size = buf.len() as u32;
            let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
            CloseHandle(handle);
            if ok == 0 {
                return (format!("pid:{pid}"), None);
            }
            let path = String::from_utf16_lossy(&buf[..size as usize]);
            let name = std::path::Path::new(&path)
                .file_name()
                .map(|s| s.to_string_lossy().to_string())
                .unwrap_or_else(|| format!("pid:{pid}"));
            (name, Some(path))
        }
    }

    fn wide_to_string(buf: &[u16]) -> String {
        let len = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
        String::from_utf16_lossy(&buf[..len])
    }
}
