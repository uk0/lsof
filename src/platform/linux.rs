use crate::error::{LoofError, Result};
use crate::model::*;
use super::PlatformProvider;

pub struct LinuxProvider;

impl LinuxProvider {
    pub fn new() -> Self {
        Self
    }
}

impl PlatformProvider for LinuxProvider {
    fn list_processes(&self) -> Result<Vec<ProcessInfo>> {
        let mut processes = Vec::new();
        let all_procs = procfs::process::all_processes()
            .map_err(|e| LoofError::Platform(e.to_string()))?;

        for proc_result in all_procs {
            let proc = match proc_result {
                Ok(p) => p,
                Err(_) => continue,
            };

            let stat = match proc.stat() {
                Ok(s) => s,
                Err(_) => continue,
            };

            let uid = proc.uid().unwrap_or(0);
            let user = users::get_user_by_uid(uid)
                .map(|u| u.name().to_string_lossy().to_string())
                .unwrap_or_else(|| uid.to_string());

            let cmdline = proc.cmdline().unwrap_or_default().join(" ");
            let command = if cmdline.is_empty() {
                format!("[{}]", stat.comm)
            } else {
                cmdline
            };

            processes.push(ProcessInfo {
                pid: stat.pid as u32,
                ppid: Some(stat.ppid as u32),
                command,
                comm: stat.comm.clone(),
                user,
                uid,
                open_files: Vec::new(),
            });
        }

        Ok(processes)
    }

    fn list_open_files(&self, pid: u32) -> Result<Vec<OpenFileInfo>> {
        // Phase 2 implementation
        let _ = pid;
        Ok(Vec::new())
    }

    fn list_network_connections(&self, pid: Option<u32>) -> Result<Vec<NetworkInfo>> {
        // Phase 2 implementation
        let _ = pid;
        Ok(Vec::new())
    }

    fn get_process_detail(&self, pid: u32) -> Result<ProcessInfo> {
        let processes = self.list_processes()?;
        processes.into_iter()
            .find(|p| p.pid == pid)
            .ok_or(LoofError::ProcessNotFound(pid))
    }
}