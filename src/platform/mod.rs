use crate::error::Result;
use crate::model::{NetworkInfo, OpenFileInfo, ProcessInfo};

pub trait PlatformProvider: Send + Sync {
    fn list_processes(&self) -> Result<Vec<ProcessInfo>>;
    fn list_open_files(&self, pid: u32) -> Result<Vec<OpenFileInfo>>;
    #[allow(dead_code)]
    fn list_network_connections(&self, pid: Option<u32>) -> Result<Vec<NetworkInfo>>;
    #[allow(dead_code)]
    fn get_process_detail(&self, pid: u32) -> Result<ProcessInfo>;
}

#[derive(Debug, Clone, Default)]
pub struct ProviderConfig {
    #[allow(dead_code)]
    pub avoid_stat: bool,
    pub follow_symlinks: bool,
}

#[cfg(target_os = "linux")]
mod linux;
#[cfg(target_os = "macos")]
mod macos;

pub fn create_provider(config: ProviderConfig) -> Box<dyn PlatformProvider> {
    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxProvider::new(config))
    }
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacosProvider::new(config))
    }
}
