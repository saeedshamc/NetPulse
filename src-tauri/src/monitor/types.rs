use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Protocol {
    Tcp,
    Udp,
    Mixed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum InterfaceType {
    Wifi,
    Ethernet,
    Hotspot,
    Vpn,
    Bluetooth,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppTraffic {
    pub pid: u32,
    pub name: String,
    pub executable_path: Option<String>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub bytes_sent_rate: u64,
    pub bytes_received_rate: u64,
    pub protocol: Protocol,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceTraffic {
    pub name: String,
    pub interface_type: InterfaceType,
    pub index: u32,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub bytes_sent_rate: u64,
    pub bytes_received_rate: u64,
    pub ssid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitorSnapshot {
    pub timestamp_ms: u64,
    pub apps: Vec<AppTraffic>,
    pub interfaces: Vec<InterfaceTraffic>,
    pub current_ssid: Option<String>,
    pub privilege_note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HourlyAggregate {
    pub hour_start_ms: i64,
    pub interface_name: Option<String>,
    pub application_name: Option<String>,
    pub network_ssid: Option<String>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailyAggregate {
    pub day_start_ms: i64,
    pub interface_name: Option<String>,
    pub application_name: Option<String>,
    pub network_ssid: Option<String>,
    pub bytes_sent: u64,
    pub bytes_received: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn app_traffic_serializes() {
        let app = AppTraffic {
            pid: 1,
            name: "test.exe".into(),
            executable_path: None,
            bytes_sent: 10,
            bytes_received: 20,
            bytes_sent_rate: 1,
            bytes_received_rate: 2,
            protocol: Protocol::Tcp,
        };
        let json = serde_json::to_string(&app).unwrap();
        assert!(json.contains("test.exe"));
        assert!(json.contains("tcp"));
    }
}
