use crate::error::{LoofError, Result};
use crate::model::*;
use super::{PlatformProvider, ProviderConfig};

use std::collections::HashMap;
use std::fs;
use std::net::SocketAddr;
use std::os::unix::fs::MetadataExt;
use std::path::Path;

// ---------------------------------------------------------------------------
// Helper types
// ---------------------------------------------------------------------------

/// Intermediate representation for a socket found in /proc/net/*.
#[derive(Debug, Clone)]
struct SocketNetInfo {
    protocol: Protocol,
    local_addr: String,
    local_port: u16,
    remote_addr: String,
    remote_port: u16,
    state: TcpState,
    tx_queue: Option<u64>,
    rx_queue: Option<u64>,
}

// ---------------------------------------------------------------------------
// Helper functions
// ---------------------------------------------------------------------------

/// Classify a `std::fs::Metadata` into our `FileType`.
fn classify_file_type(meta: &fs::Metadata) -> FileType {
    let ft = meta.file_type();
    if ft.is_file() {
        FileType::Reg
    } else if ft.is_dir() {
        FileType::Dir
    } else if ft.is_symlink() {
        FileType::Link
    } else {
        // On Unix, use the mode bits to distinguish special files.
        use std::os::unix::fs::FileTypeExt;
        if ft.is_block_device() {
            FileType::Blk
        } else if ft.is_char_device() {
            FileType::Chr
        } else if ft.is_fifo() {
            FileType::Fifo
        } else if ft.is_socket() {
            FileType::Sock
        } else {
            FileType::Unknown("unknown".to_string())
        }
    }
}

/// Determine `FdMode` from the POSIX open-flags value read from fdinfo.
/// O_RDONLY = 0, O_WRONLY = 1, O_RDWR = 2.
fn fd_mode_from_flags(flags: u32) -> FdMode {
    match flags & 0o3 {
        0 => FdMode::Read,
        1 => FdMode::Write,
        2 => FdMode::ReadWrite,
        _ => FdMode::Unknown,
    }
}

/// Read the `flags:` field from `/proc/[pid]/fdinfo/[fd]`.
fn read_fd_flags(pid: u32, fd: i32) -> FdMode {
    let path = format!("/proc/{}/fdinfo/{}", pid, fd);
    match fs::read_to_string(&path) {
        Ok(content) => {
            for line in content.lines() {
                if let Some(rest) = line.strip_prefix("flags:") {
                    let trimmed = rest.trim();
                    // The flags field is an octal number (e.g. "0100002").
                    // Parse it as octal.
                    let digits = trimmed.trim_start_matches('0');
                    let octal_str = if digits.is_empty() { "0" } else { digits };
                    if let Ok(val) = u32::from_str_radix(octal_str, 8) {
                        return fd_mode_from_flags(val);
                    }
                    // Fallback: try decimal parse
                    if let Ok(val) = trimmed.parse::<u32>() {
                        return fd_mode_from_flags(val);
                    }
                }
            }
            FdMode::Unknown
        }
        Err(_) => FdMode::Unknown,
    }
}

/// Format a device number as "major,minor" using the Linux encoding.
fn format_device(dev: u64) -> String {
    let major = ((dev >> 8) & 0xfff) | ((dev >> 32) & !0xfff);
    let minor = (dev & 0xff) | ((dev >> 12) & !0xff);
    format!("{},{}", major, minor)
}

/// Map a `procfs::net::TcpState` to our `TcpState`.
fn map_tcp_state(state: &procfs::net::TcpState) -> TcpState {
    match state {
        procfs::net::TcpState::Established => TcpState::Established,
        procfs::net::TcpState::SynSent => TcpState::SynSent,
        procfs::net::TcpState::SynRecv => TcpState::SynRecv,
        procfs::net::TcpState::FinWait1 => TcpState::FinWait1,
        procfs::net::TcpState::FinWait2 => TcpState::FinWait2,
        procfs::net::TcpState::TimeWait => TcpState::TimeWait,
        procfs::net::TcpState::Close => TcpState::Closed,
        procfs::net::TcpState::CloseWait => TcpState::CloseWait,
        procfs::net::TcpState::LastAck => TcpState::LastAck,
        procfs::net::TcpState::Listen => TcpState::Listen,
        procfs::net::TcpState::Closing => TcpState::Closing,
        _ => TcpState::Unknown(format!("{:?}", state)),
    }
}

/// Format a `SocketAddr` into its IP string.
fn addr_ip_string(addr: &SocketAddr) -> String {
    addr.ip().to_string()
}

// ---------------------------------------------------------------------------
// Socket inode map builder
// ---------------------------------------------------------------------------

/// Build a map from socket inode -> SocketNetInfo by reading
/// /proc/net/tcp, tcp6, udp, udp6, and unix.
fn build_socket_inode_map() -> HashMap<u64, SocketNetInfo> {
    let mut map = HashMap::new();

    // TCP (IPv4)
    if let Ok(entries) = procfs::net::tcp() {
        for entry in entries {
            map.insert(entry.inode, SocketNetInfo {
                protocol: Protocol::Tcp,
                local_addr: addr_ip_string(&entry.local_address),
                local_port: entry.local_address.port(),
                remote_addr: addr_ip_string(&entry.remote_address),
                remote_port: entry.remote_address.port(),
                state: map_tcp_state(&entry.state),
                tx_queue: Some(entry.tx_queue as u64),
                rx_queue: Some(entry.rx_queue as u64),
            });
        }
    }

    // TCP6 (IPv6)
    if let Ok(entries) = procfs::net::tcp6() {
        for entry in entries {
            map.insert(entry.inode, SocketNetInfo {
                protocol: Protocol::Tcp6,
                local_addr: addr_ip_string(&entry.local_address),
                local_port: entry.local_address.port(),
                remote_addr: addr_ip_string(&entry.remote_address),
                remote_port: entry.remote_address.port(),
                state: map_tcp_state(&entry.state),
                tx_queue: Some(entry.tx_queue as u64),
                rx_queue: Some(entry.rx_queue as u64),
            });
        }
    }

    // UDP (IPv4)
    if let Ok(entries) = procfs::net::udp() {
        for entry in entries {
            let state = match entry.state {
                procfs::net::UdpState::Established => TcpState::Established,
                _ => TcpState::Unknown("NONE".to_string()),
            };
            map.insert(entry.inode, SocketNetInfo {
                protocol: Protocol::Udp,
                local_addr: addr_ip_string(&entry.local_address),
                local_port: entry.local_address.port(),
                remote_addr: addr_ip_string(&entry.remote_address),
                remote_port: entry.remote_address.port(),
                state,
                tx_queue: Some(entry.tx_queue as u64),
                rx_queue: Some(entry.rx_queue as u64),
            });
        }
    }

    // UDP6 (IPv6)
    if let Ok(entries) = procfs::net::udp6() {
        for entry in entries {
            let state = match entry.state {
                procfs::net::UdpState::Established => TcpState::Established,
                _ => TcpState::Unknown("NONE".to_string()),
            };
            map.insert(entry.inode, SocketNetInfo {
                protocol: Protocol::Udp6,
                local_addr: addr_ip_string(&entry.local_address),
                local_port: entry.local_address.port(),
                remote_addr: addr_ip_string(&entry.remote_address),
                remote_port: entry.remote_address.port(),
                state,
                tx_queue: Some(entry.tx_queue as u64),
                rx_queue: Some(entry.rx_queue as u64),
            });
        }
    }

    // Unix domain sockets
    if let Ok(entries) = procfs::net::unix() {
        for entry in entries {
            let path_str = entry.path
                .as_ref()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_default();
            map.insert(entry.inode, SocketNetInfo {
                protocol: Protocol::Unix,
                local_addr: path_str,
                local_port: 0,
                remote_addr: String::new(),
                remote_port: 0,
                state: TcpState::Unknown("NONE".to_string()),
                tx_queue: None,
                rx_queue: None,
            });
        }
    }

    map
}

// ---------------------------------------------------------------------------
// Build an OpenFileInfo from a path (used for cwd, root, exe, and Path FDs)
// ---------------------------------------------------------------------------

/// Create an `OpenFileInfo` for a special entry (cwd, rtd, txt) from a path.
fn open_file_from_path(path: &Path, fd_type: FdType, config: &ProviderConfig) -> OpenFileInfo {
    let name = path.to_string_lossy().to_string();

    // When avoid_stat is true, skip all stat calls and return minimal info.
    if config.avoid_stat {
        return OpenFileInfo {
            fd: fd_type,
            file_type: FileType::Unknown("".into()),
            device: String::new(),
            size_off: None,
            node: String::new(),
            name,
            mode: None,
            link_target: None,
            send_queue: None,
            recv_queue: None,
        };
    }

    // When follow_symlinks is true, use metadata() (follows symlinks) instead
    // of symlink_metadata().
    let (file_type, device, size_off, node, link_target) = if config.follow_symlinks {
        match fs::metadata(path) {
            Ok(meta) => {
                let ft = classify_file_type(&meta);
                let dev = format_device(meta.dev());
                let size = Some(meta.size());
                let ino = meta.ino().to_string();
                (ft, dev, size, ino, None)
            }
            Err(_) => (
                FileType::Unknown("?".to_string()),
                String::new(),
                None,
                String::new(),
                None,
            ),
        }
    } else {
        // Try symlink_metadata first (does not follow symlinks), then metadata.
        match fs::symlink_metadata(path) {
            Ok(meta) => {
                let ft = classify_file_type(&meta);
                let dev = format_device(meta.dev());
                let size = Some(meta.size());
                let ino = meta.ino().to_string();
                let lt = if ft == FileType::Link {
                    fs::read_link(path)
                        .ok()
                        .map(|p| p.to_string_lossy().to_string())
                } else {
                    None
                };
                (ft, dev, size, ino, lt)
            }
            Err(_) => {
                // Cannot stat -- still record the entry with what we know.
                (
                    FileType::Unknown("?".to_string()),
                    String::new(),
                    None,
                    String::new(),
                    None,
                )
            }
        }
    };

    OpenFileInfo {
        fd: fd_type,
        file_type,
        device,
        size_off,
        node,
        name,
        mode: None,
        link_target,
        send_queue: None,
        recv_queue: None,
    }
}

/// Create an `OpenFileInfo` for a numbered FD backed by a filesystem path.
fn open_file_from_fd_path(
    path: &Path,
    fd_num: u32,
    mode: FdMode,
    config: &ProviderConfig,
) -> OpenFileInfo {
    let name = path.to_string_lossy().to_string();

    // When avoid_stat is true, skip all stat calls and return minimal info.
    if config.avoid_stat {
        return OpenFileInfo {
            fd: FdType::Numbered(fd_num, mode),
            file_type: FileType::Unknown("".into()),
            device: String::new(),
            size_off: None,
            node: String::new(),
            name,
            mode: Some(mode),
            link_target: None,
            send_queue: None,
            recv_queue: None,
        };
    }

    // When follow_symlinks is true and the path is a symlink, use metadata()
    // to follow the link. Otherwise use the normal metadata -> symlink_metadata
    // fallback chain.
    let (file_type, device, size_off, node, link_target) = if config.follow_symlinks {
        // Check if it's a symlink first; if so, follow it with metadata().
        let is_symlink = fs::symlink_metadata(path)
            .map(|m| m.file_type().is_symlink())
            .unwrap_or(false);
        if is_symlink {
            match fs::metadata(path) {
                Ok(meta) => {
                    let ft = classify_file_type(&meta);
                    let dev = format_device(meta.dev());
                    let size = Some(meta.size());
                    let ino = meta.ino().to_string();
                    let lt = fs::read_link(path)
                        .ok()
                        .map(|p| p.to_string_lossy().to_string());
                    (ft, dev, size, ino, lt)
                }
                Err(_) => (
                    FileType::Unknown("?".to_string()),
                    String::new(),
                    None,
                    String::new(),
                    None,
                ),
            }
        } else {
            match fs::metadata(path) {
                Ok(meta) => {
                    let ft = classify_file_type(&meta);
                    let dev = format_device(meta.dev());
                    let size = Some(meta.size());
                    let ino = meta.ino().to_string();
                    (ft, dev, size, ino, None)
                }
                Err(_) => (
                    FileType::Unknown("?".to_string()),
                    String::new(),
                    None,
                    String::new(),
                    None,
                ),
            }
        }
    } else {
        match fs::metadata(path) {
            Ok(meta) => {
                let ft = classify_file_type(&meta);
                let dev = format_device(meta.dev());
                let size = Some(meta.size());
                let ino = meta.ino().to_string();
                let lt = if ft == FileType::Link {
                    fs::read_link(path)
                        .ok()
                        .map(|p| p.to_string_lossy().to_string())
                } else {
                    None
                };
                (ft, dev, size, ino, lt)
            }
            Err(_) => {
                // Fallback: try symlink_metadata (the fd link itself).
                match fs::symlink_metadata(path) {
                    Ok(meta) => {
                        let ft = classify_file_type(&meta);
                        let dev = format_device(meta.dev());
                        let size = Some(meta.size());
                        let ino = meta.ino().to_string();
                        (ft, dev, size, ino, None)
                    }
                    Err(_) => (
                        FileType::Unknown("?".to_string()),
                        String::new(),
                        None,
                        String::new(),
                        None,
                    ),
                }
            }
        }
    };

    OpenFileInfo {
        fd: FdType::Numbered(fd_num, mode),
        file_type,
        device,
        size_off,
        node,
        name,
        mode: Some(mode),
        link_target,
        send_queue: None,
        recv_queue: None,
    }
}

// ---------------------------------------------------------------------------
// LinuxProvider
// ---------------------------------------------------------------------------

pub struct LinuxProvider {
    config: ProviderConfig,
}

impl LinuxProvider {
    pub fn new(config: ProviderConfig) -> Self {
        Self { config }
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
                pgid: Some(stat.pgrp as u32),
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
        let process = procfs::process::Process::new(pid as i32)
            .map_err(|e| LoofError::Platform(format!("Cannot open process {}: {}", pid, e)))?;

        let mut results = Vec::new();

        // --- Special entries: cwd, root, exe ---

        // cwd
        if let Ok(cwd_path) = process.cwd() {
            results.push(open_file_from_path(&cwd_path, FdType::Cwd, &self.config));
        }

        // root (rtd)
        if let Ok(root_path) = process.root() {
            results.push(open_file_from_path(&root_path, FdType::Rtd, &self.config));
        }

        // exe (txt)
        if let Ok(exe_path) = process.exe() {
            results.push(open_file_from_path(&exe_path, FdType::Txt, &self.config));
        }

        // --- Memory-mapped files (mem entries) ---
        if let Ok(maps) = process.maps() {
            let mut seen_paths = std::collections::HashSet::new();
            for map in maps.iter() {
                if let procfs::process::MMapPath::Path(ref p) = map.pathname {
                    let path_str = p.to_string_lossy().to_string();
                    if seen_paths.insert(path_str.clone()) {
                        let (file_type, device, size_off, node) =
                            match fs::metadata(p) {
                                Ok(meta) => (
                                    classify_file_type(&meta),
                                    format_device(meta.dev()),
                                    Some(meta.size()),
                                    meta.ino().to_string(),
                                ),
                                Err(_) => (
                                    FileType::Reg,
                                    String::new(),
                                    None,
                                    String::new(),
                                ),
                            };
                        results.push(OpenFileInfo {
                            fd: FdType::Mem,
                            file_type,
                            device,
                            size_off,
                            node,
                            name: path_str,
                            mode: Some(FdMode::Read),
                            link_target: None,
                            send_queue: None,
                            recv_queue: None,
                        });
                    }
                }
            }
        }

        // --- Build socket inode map for resolving socket FDs ---
        let socket_map = build_socket_inode_map();

        // --- Enumerate numbered FDs ---
        let fds = match process.fd() {
            Ok(fds) => fds,
            Err(_) => return Ok(results), // Permission denied or gone
        };

        for fd_info in fds {
            let fd_info = match fd_info {
                Ok(fi) => fi,
                Err(_) => continue,
            };

            let fd_num = fd_info.fd as u32;
            let mode = read_fd_flags(pid, fd_info.fd);

            match fd_info.target {
                procfs::process::FDTarget::Path(ref path) => {
                    results.push(open_file_from_fd_path(path, fd_num, mode, &self.config));
                }
                procfs::process::FDTarget::Socket(inode) => {
                    if let Some(sock_info) = socket_map.get(&inode) {
                        let (file_type, name) = match sock_info.protocol {
                            Protocol::Tcp => (
                                FileType::IPv4,
                                format!(
                                    "{}:{} -> {}:{} ({})",
                                    sock_info.local_addr,
                                    sock_info.local_port,
                                    sock_info.remote_addr,
                                    sock_info.remote_port,
                                    sock_info.state,
                                ),
                            ),
                            Protocol::Tcp6 => (
                                FileType::IPv6,
                                format!(
                                    "{}:{} -> {}:{} ({})",
                                    sock_info.local_addr,
                                    sock_info.local_port,
                                    sock_info.remote_addr,
                                    sock_info.remote_port,
                                    sock_info.state,
                                ),
                            ),
                            Protocol::Udp => (
                                FileType::IPv4,
                                format!(
                                    "{}:{} -> {}:{} (UDP)",
                                    sock_info.local_addr,
                                    sock_info.local_port,
                                    sock_info.remote_addr,
                                    sock_info.remote_port,
                                ),
                            ),
                            Protocol::Udp6 => (
                                FileType::IPv6,
                                format!(
                                    "{}:{} -> {}:{} (UDP6)",
                                    sock_info.local_addr,
                                    sock_info.local_port,
                                    sock_info.remote_addr,
                                    sock_info.remote_port,
                                ),
                            ),
                            Protocol::Unix => {
                                let path = &sock_info.local_addr;
                                let display = if path.is_empty() {
                                    format!("unix socket inode={}", inode)
                                } else {
                                    path.clone()
                                };
                                (FileType::Unix, display)
                            }
                        };
                        results.push(OpenFileInfo {
                            fd: FdType::Numbered(fd_num, mode),
                            file_type,
                            device: String::new(),
                            size_off: None,
                            node: inode.to_string(),
                            name,
                            mode: Some(mode),
                            link_target: None,
                            send_queue: sock_info.tx_queue,
                            recv_queue: sock_info.rx_queue,
                        });
                    } else {
                        // Socket inode not found in /proc/net tables.
                        results.push(OpenFileInfo {
                            fd: FdType::Numbered(fd_num, mode),
                            file_type: FileType::Sock,
                            device: String::new(),
                            size_off: None,
                            node: inode.to_string(),
                            name: format!("socket:[{}]", inode),
                            mode: Some(mode),
                            link_target: None,
                            send_queue: None,
                            recv_queue: None,
                        });
                    }
                }
                procfs::process::FDTarget::Net(inode) => {
                    // Same handling as Socket -- look up in the map.
                    if let Some(sock_info) = socket_map.get(&inode) {
                        let (file_type, name) = match sock_info.protocol {
                            Protocol::Tcp | Protocol::Tcp6 => {
                                let ft = if sock_info.protocol == Protocol::Tcp {
                                    FileType::IPv4
                                } else {
                                    FileType::IPv6
                                };
                                (
                                    ft,
                                    format!(
                                        "{}:{} -> {}:{} ({})",
                                        sock_info.local_addr,
                                        sock_info.local_port,
                                        sock_info.remote_addr,
                                        sock_info.remote_port,
                                        sock_info.state,
                                    ),
                                )
                            }
                            Protocol::Udp | Protocol::Udp6 => {
                                let ft = if sock_info.protocol == Protocol::Udp {
                                    FileType::IPv4
                                } else {
                                    FileType::IPv6
                                };
                                (
                                    ft,
                                    format!(
                                        "{}:{} -> {}:{} ({})",
                                        sock_info.local_addr,
                                        sock_info.local_port,
                                        sock_info.remote_addr,
                                        sock_info.remote_port,
                                        sock_info.protocol,
                                    ),
                                )
                            }
                            Protocol::Unix => {
                                let path = &sock_info.local_addr;
                                let display = if path.is_empty() {
                                    format!("unix socket inode={}", inode)
                                } else {
                                    path.clone()
                                };
                                (FileType::Unix, display)
                            }
                        };
                        results.push(OpenFileInfo {
                            fd: FdType::Numbered(fd_num, mode),
                            file_type,
                            device: String::new(),
                            size_off: None,
                            node: inode.to_string(),
                            name,
                            mode: Some(mode),
                            link_target: None,
                            send_queue: sock_info.tx_queue,
                            recv_queue: sock_info.rx_queue,
                        });
                    } else {
                        results.push(OpenFileInfo {
                            fd: FdType::Numbered(fd_num, mode),
                            file_type: FileType::Sock,
                            device: String::new(),
                            size_off: None,
                            node: inode.to_string(),
                            name: format!("net:[{}]", inode),
                            mode: Some(mode),
                            link_target: None,
                            send_queue: None,
                            recv_queue: None,
                        });
                    }
                }
                procfs::process::FDTarget::Pipe(inode) => {
                    results.push(OpenFileInfo {
                        fd: FdType::Numbered(fd_num, mode),
                        file_type: FileType::Pipe,
                        device: String::new(),
                        size_off: None,
                        node: inode.to_string(),
                        name: format!("pipe:[{}]", inode),
                        mode: Some(mode),
                        link_target: None,
                        send_queue: None,
                        recv_queue: None,
                    });
                }
                procfs::process::FDTarget::AnonInode(ref desc) => {
                    results.push(OpenFileInfo {
                        fd: FdType::Numbered(fd_num, mode),
                        file_type: FileType::Unknown(desc.clone()),
                        device: String::new(),
                        size_off: None,
                        node: String::new(),
                        name: format!("anon_inode:[{}]", desc),
                        mode: Some(mode),
                        link_target: None,
                        send_queue: None,
                        recv_queue: None,
                    });
                }
                procfs::process::FDTarget::MemFD(ref name_str) => {
                    results.push(OpenFileInfo {
                        fd: FdType::Numbered(fd_num, mode),
                        file_type: FileType::Reg,
                        device: String::new(),
                        size_off: None,
                        node: String::new(),
                        name: format!("memfd:{}", name_str),
                        mode: Some(mode),
                        link_target: None,
                        send_queue: None,
                        recv_queue: None,
                    });
                }
                procfs::process::FDTarget::Other(ref name_str, inode) => {
                    results.push(OpenFileInfo {
                        fd: FdType::Numbered(fd_num, mode),
                        file_type: FileType::Unknown(name_str.clone()),
                        device: String::new(),
                        size_off: None,
                        node: inode.to_string(),
                        name: format!("{}:[{}]", name_str, inode),
                        mode: Some(mode),
                        link_target: None,
                        send_queue: None,
                        recv_queue: None,
                    });
                }
            }
        }

        Ok(results)
    }

    fn list_network_connections(&self, pid: Option<u32>) -> Result<Vec<NetworkInfo>> {
        let socket_map = build_socket_inode_map();

        match pid {
            Some(target_pid) => {
                // Get the process's FDs and find which are sockets.
                let process = procfs::process::Process::new(target_pid as i32)
                    .map_err(|e| LoofError::Platform(
                        format!("Cannot open process {}: {}", target_pid, e),
                    ))?;

                let stat = process.stat()
                    .map_err(|e| LoofError::Platform(e.to_string()))?;
                let command = Some(stat.comm.clone());

                let fds = match process.fd() {
                    Ok(fds) => fds,
                    Err(_) => return Ok(Vec::new()),
                };

                let mut connections = Vec::new();
                for fd_info in fds {
                    let fd_info = match fd_info {
                        Ok(fi) => fi,
                        Err(_) => continue,
                    };

                    let inode = match &fd_info.target {
                        procfs::process::FDTarget::Socket(i) => Some(*i),
                        procfs::process::FDTarget::Net(i) => Some(*i),
                        _ => None,
                    };

                    if let Some(inode) = inode {
                        if let Some(sock_info) = socket_map.get(&inode) {
                            connections.push(NetworkInfo {
                                protocol: sock_info.protocol.clone(),
                                local_addr: sock_info.local_addr.clone(),
                                local_port: sock_info.local_port,
                                remote_addr: sock_info.remote_addr.clone(),
                                remote_port: sock_info.remote_port,
                                state: sock_info.state.clone(),
                                pid: Some(target_pid),
                                command: command.clone(),
                            });
                        }
                    }
                }

                Ok(connections)
            }
            None => {
                // No specific PID -- build a reverse map from inode -> (pid, comm)
                // by scanning all processes, then emit NetworkInfo for every
                // socket in the global tables.
                let mut inode_to_proc: HashMap<u64, (u32, String)> = HashMap::new();

                let all_procs = procfs::process::all_processes()
                    .map_err(|e| LoofError::Platform(e.to_string()))?;

                for proc_result in all_procs {
                    let proc = match proc_result {
                        Ok(p) => p,
                        Err(_) => continue,
                    };

                    let proc_pid = proc.pid as u32;
                    let comm = proc.stat()
                        .map(|s| s.comm.clone())
                        .unwrap_or_default();

                    let fds = match proc.fd() {
                        Ok(fds) => fds,
                        Err(_) => continue,
                    };

                    for fd_info in fds {
                        let fd_info = match fd_info {
                            Ok(fi) => fi,
                            Err(_) => continue,
                        };

                        let inode = match &fd_info.target {
                            procfs::process::FDTarget::Socket(i) => Some(*i),
                            procfs::process::FDTarget::Net(i) => Some(*i),
                            _ => None,
                        };

                        if let Some(inode) = inode {
                            inode_to_proc
                                .entry(inode)
                                .or_insert((proc_pid, comm.clone()));
                        }
                    }
                }

                let mut connections = Vec::new();
                for (inode, sock_info) in &socket_map {
                    let (pid_val, cmd) = inode_to_proc
                        .get(inode)
                        .cloned()
                        .unwrap_or((0, String::new()));

                    connections.push(NetworkInfo {
                        protocol: sock_info.protocol.clone(),
                        local_addr: sock_info.local_addr.clone(),
                        local_port: sock_info.local_port,
                        remote_addr: sock_info.remote_addr.clone(),
                        remote_port: sock_info.remote_port,
                        state: sock_info.state.clone(),
                        pid: if pid_val > 0 { Some(pid_val) } else { None },
                        command: if cmd.is_empty() { None } else { Some(cmd) },
                    });
                }

                Ok(connections)
            }
        }
    }

    fn get_process_detail(&self, pid: u32) -> Result<ProcessInfo> {
        let processes = self.list_processes()?;
        let mut proc_info = processes.into_iter()
            .find(|p| p.pid == pid)
            .ok_or(LoofError::ProcessNotFound(pid))?;

        // Populate open files for the detailed view.
        proc_info.open_files = self.list_open_files(pid)?;
        Ok(proc_info)
    }
}
