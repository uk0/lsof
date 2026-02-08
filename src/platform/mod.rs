use crate::error::Result;
use crate::model::{ProcessInfo, OpenFileInfo, NetworkInfo};

pub trait PlatformProvider: Send + Sync {
    fn list_processes(&self) -> Result<Vec<ProcessInfo>>;
    fn list_open_files(&self, pid: u32) -> Result<Vec<OpenFileInfo>>;
    fn list_network_connections(&self, pid: Option<u32>) -> Result<Vec<NetworkInfo>>;
    fn get_process_detail(&self, pid: u32) -> Result<ProcessInfo>;
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

pub fn create_provider() -> Box<dyn PlatformProvider> {
    #[cfg(target_os = "linux")]
    { Box::new(linux::LinuxProvider::new()) }
    #[cfg(target_os = "macos")]
    { Box::new(macos::MacosProvider::new()) }
}
