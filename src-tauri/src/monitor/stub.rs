use crate::monitor::types::{AppTraffic, InterfaceTraffic};
use crate::monitor::NetworkMonitor;

pub struct StubMonitor;

impl StubMonitor {
    pub fn new() -> Self {
        Self
    }
}

impl NetworkMonitor for StubMonitor {
    fn poll_per_app_traffic(&mut self) -> Result<Vec<AppTraffic>, String> {
        Err("Network monitoring is not implemented for this platform yet.".into())
    }

    fn poll_per_interface_traffic(&mut self) -> Result<Vec<InterfaceTraffic>, String> {
        Err("Network monitoring is not implemented for this platform yet.".into())
    }

    fn current_ssid(&mut self) -> Result<Option<String>, String> {
        Ok(None)
    }

    fn privilege_note(&self) -> Option<String> {
        Some("This platform build is a stub. Use Windows for the current MVP.".into())
    }
}
