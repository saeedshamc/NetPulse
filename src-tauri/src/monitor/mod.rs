mod types;

#[cfg(not(windows))]
mod stub;

#[cfg(windows)]
mod windows_impl;

pub use types::*;

pub trait NetworkMonitor: Send {
    fn poll_per_app_traffic(&mut self) -> Result<Vec<AppTraffic>, String>;
    fn poll_per_interface_traffic(&mut self) -> Result<Vec<InterfaceTraffic>, String>;
    fn current_ssid(&mut self) -> Result<Option<String>, String>;
    fn privilege_note(&self) -> Option<String>;

    fn poll_snapshot(&mut self) -> Result<MonitorSnapshot, String> {
        let apps = self.poll_per_app_traffic()?;
        let interfaces = self.poll_per_interface_traffic()?;
        let current_ssid = self.current_ssid().unwrap_or(None);
        let privilege_note = self.privilege_note();
        let timestamp_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Ok(MonitorSnapshot {
            timestamp_ms,
            apps,
            interfaces,
            current_ssid,
            privilege_note,
        })
    }
}

pub fn create_monitor() -> Box<dyn NetworkMonitor> {
    #[cfg(windows)]
    {
        Box::new(windows_impl::WindowsMonitor::new())
    }
    #[cfg(not(windows))]
    {
        Box::new(stub::StubMonitor::new())
    }
}

#[cfg(all(test, windows))]
mod windows_smoke {
    use super::create_monitor;

    #[test]
    fn poll_snapshot_smoke() {
        let mut monitor = create_monitor();
        let snap = monitor
            .poll_snapshot()
            .expect("first poll should succeed on Windows");
        assert!(snap.timestamp_ms > 0);
        let _ = monitor.poll_snapshot().expect("second poll");
    }
}
