use std::fmt;

#[derive(Debug, Clone)]
pub struct OpenFileInfo {
    pub fd: FdType,
    pub file_type: FileType,
    pub device: String,
    pub size_off: Option<u64>,
    pub node: String,
    pub name: String,
    #[allow(dead_code)]
    pub mode: Option<FdMode>,
    pub link_target: Option<String>,
    pub send_queue: Option<u64>,
    pub recv_queue: Option<u64>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FileType {
    Reg,
    Dir,
    Chr,
    Blk,
    Fifo,
    Sock,
    Link,
    Pipe,
    IPv4,
    IPv6,
    Unix,
    #[allow(dead_code)]
    Kqueue,
    #[allow(dead_code)]
    Systm,
    Unknown(String),
}

impl fmt::Display for FileType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileType::Reg => write!(f, "REG"),
            FileType::Dir => write!(f, "DIR"),
            FileType::Chr => write!(f, "CHR"),
            FileType::Blk => write!(f, "BLK"),
            FileType::Fifo => write!(f, "FIFO"),
            FileType::Sock => write!(f, "SOCK"),
            FileType::Link => write!(f, "LINK"),
            FileType::Pipe => write!(f, "PIPE"),
            FileType::IPv4 => write!(f, "IPv4"),
            FileType::IPv6 => write!(f, "IPv6"),
            FileType::Unix => write!(f, "unix"),
            FileType::Kqueue => write!(f, "KQUEUE"),
            FileType::Systm => write!(f, "SYSTM"),
            FileType::Unknown(s) => write!(f, "{}", s),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FdType {
    #[allow(dead_code)]
    Cwd,
    Txt,
    #[allow(dead_code)]
    Mem,
    #[allow(dead_code)]
    Rtd,
    #[allow(dead_code)]
    Mmap,
    Numbered(u32, FdMode),
}

impl fmt::Display for FdType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FdType::Cwd => write!(f, "cwd"),
            FdType::Txt => write!(f, "txt"),
            FdType::Mem => write!(f, "mem"),
            FdType::Rtd => write!(f, "rtd"),
            FdType::Mmap => write!(f, "mmap"),
            FdType::Numbered(n, mode) => write!(f, "{}{}", n, mode),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FdMode {
    Read,
    Write,
    ReadWrite,
    Unknown,
}

impl fmt::Display for FdMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FdMode::Read => write!(f, "r"),
            FdMode::Write => write!(f, "w"),
            FdMode::ReadWrite => write!(f, "u"),
            FdMode::Unknown => write!(f, " "),
        }
    }
}
