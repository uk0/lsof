use super::open_file::OpenFileInfo;

#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub pid: u32,
    pub ppid: Option<u32>,
    pub pgid: Option<u32>,
    pub command: String,
    pub comm: String,
    pub user: String,
    pub uid: u32,
    pub open_files: Vec<OpenFileInfo>,
}

impl ProcessInfo {
    pub fn fd_count(&self) -> usize {
        self.open_files.len()
    }

    pub fn display_line(&self) -> String {
        format!("{:<8} {:<20} {:<12}", self.pid, self.comm, self.user)
    }
}
