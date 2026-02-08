use crate::error::{LoofError, Result};
use crate::model::*;
use super::PlatformProvider;
use sysinfo::System;

use std::ffi::CStr;
use std::mem;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::os::raw::{c_int, c_void};

use libproc::libproc::file_info::{
    pidfdinfo, ListFDs, ProcFDInfo, ProcFDType,
};
use libproc::libproc::net_info::{SocketFDInfo, SocketInfoKind, TcpSIState};
use libproc::libproc::proc_pid::{listpidinfo, pidinfo, pidpath};
use libproc::libproc::task_info::TaskAllInfo;

// --- Raw FFI for vnode/pipe/kqueue fd info (not exposed by libproc crate) ---

// Constants matching PROC_PIDFD* from the Darwin kernel headers.
const PROC_PIDFDVNODEPATHINFO: i32 = 2;
const PROC_PIDFDPIPEINFO: i32 = 6;
const PROC_PIDFDKQUEUEINFO: i32 = 7;

// Vnode type constants from <sys/vnode.h>.
const VNON: i32 = 0;
const VREG: i32 = 1;
const VDIR: i32 = 2;
const VBLK: i32 = 3;
const VCHR: i32 = 4;
const VLNK: i32 = 5;
const VSOCK: i32 = 6;
const VFIFO: i32 = 7;

// MAXPATHLEN on macOS
const MAXPATHLEN: usize = 1024;

// INI_IPV6 flag for insi_vflag
const INI_IPV6: u8 = 0x2;

// FREAD / FWRITE from <sys/fcntl.h> (kernel-internal open-flag bits)
const FREAD: u32 = 0x0001;
const FWRITE: u32 = 0x0002;

// Minimal repr(C) structs mirroring the Darwin kernel structures we need.
// We only define the fields we actually read; padding is handled by the
// overall struct size being correct (verified by mem::size_of checks at
// compile time would be ideal, but we rely on matching the kernel layout).

#[repr(C)]
#[derive(Copy, Clone)]
struct VInfoStat {
    vst_dev: u32,
    vst_mode: u16,
    vst_nlink: u16,
    vst_ino: u64,
    vst_uid: u32,
    vst_gid: u32,
    vst_atime: i64,
    vst_atimensec: i64,
    vst_mtime: i64,
    vst_mtimensec: i64,
    vst_ctime: i64,
    vst_ctimensec: i64,
    vst_birthtime: i64,
    vst_birthtimensec: i64,
    vst_size: i64,
    vst_blocks: i64,
    vst_blksize: i32,
    vst_flags: u32,
    vst_gen: u32,
    vst_rdev: u32,
    vst_qspare: [i64; 2],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct FsId {
    val: [i32; 2],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct VnodeInfo {
    vi_stat: VInfoStat,
    vi_type: i32,
    vi_pad: i32,
    vi_fsid: FsId,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct VnodeInfoPath {
    vip_vi: VnodeInfo,
    vip_path: [i8; MAXPATHLEN],
}

#[repr(C)]
#[derive(Copy, Clone)]
struct ProcFileInfo {
    fi_openflags: u32,
    fi_status: u32,
    fi_offset: i64,
    fi_type: i32,
    fi_guardflags: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct VnodeFdInfoWithPath {
    pfi: ProcFileInfo,
    pvip: VnodeInfoPath,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct PipeInfo {
    pipe_stat: VInfoStat,
    pipe_handle: u64,
    pipe_peerhandle: u64,
    pipe_status: i32,
    rfu_1: i32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct PipeFdInfo {
    pfi: ProcFileInfo,
    pipeinfo: PipeInfo,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct KqueueInfo {
    kq_stat: VInfoStat,
    kq_state: u32,
    rfu_1: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct KqueueFdInfo {
    pfi: ProcFileInfo,
    kqueueinfo: KqueueInfo,
}

// IPPROTO_UDP from <netinet/in.h>
const IPPROTO_UDP: c_int = 17;

extern "C" {
    fn proc_pidfdinfo(
        pid: c_int,
        fd: c_int,
        flavor: c_int,
        buffer: *mut c_void,
        buffersize: c_int,
    ) -> c_int;
}

/// Safe wrapper around the raw `proc_pidfdinfo` syscall for types not
/// covered by the libproc crate.
unsafe fn raw_pidfdinfo<T: Copy>(pid: i32, fd: i32, flavor: i32) -> Option<T> {
    let mut info: T = mem::zeroed();
    let size = mem::size_of::<T>() as c_int;
    let ret = proc_pidfdinfo(
        pid,
        fd,
        flavor,
        &mut info as *mut T as *mut c_void,
        size,
    );
    if ret <= 0 {
        None
    } else {
        Some(info)
    }
}

/// Extract a UTF-8 path from a fixed-size i8 buffer (C string).
fn path_from_c_buf(buf: &[i8]) -> String {
    let ptr = buf.as_ptr() as *const i8;
    if buf[0] == 0 {
        return String::new();
    }
    unsafe {
        CStr::from_ptr(ptr)
            .to_string_lossy()
            .into_owned()
    }
}

/// Determine the `FdMode` from the kernel `fi_openflags` field.
fn fd_mode_from_openflags(flags: u32) -> FdMode {
    let r = flags & FREAD != 0;
    let w = flags & FWRITE != 0;
    match (r, w) {
        (true, true) => FdMode::ReadWrite,
        (true, false) => FdMode::Read,
        (false, true) => FdMode::Write,
        _ => FdMode::Unknown,
    }
}

/// Map a Darwin vnode type integer to our `FileType`.
fn file_type_from_vtype(vtype: i32) -> FileType {
    match vtype {
        VREG => FileType::Reg,
        VDIR => FileType::Dir,
        VBLK => FileType::Blk,
        VCHR => FileType::Chr,
        VLNK => FileType::Link,
        VSOCK => FileType::Sock,
        VFIFO => FileType::Fifo,
        VNON => FileType::Unknown("VNON".to_string()),
        _ => FileType::Unknown(format!("vtype={}", vtype)),
    }
}

/// Format a device number as "major,minor".
fn format_device(dev: u32) -> String {
    let major = (dev >> 24) & 0xff;
    let minor = dev & 0xffffff;
    format!("0x{:x},{:x}", major, minor)
}

/// Convert a network-byte-order port (stored in an i32) to host-byte-order u16.
fn port_from_nbo(nbo: i32) -> u16 {
    let p = nbo as u16;
    u16::from_be(p)
}

/// Extract an IPv4 address from the `InSIAddr` union (network byte order).
fn ipv4_from_insi(addr: &libproc::libproc::net_info::InSIAddr) -> Ipv4Addr {
    let s_addr = unsafe { addr.ina_46.i46a_addr4.s_addr };
    Ipv4Addr::from(u32::from_be(s_addr))
}

/// Extract an IPv6 address from the `InSIAddr` union.
fn ipv6_from_insi(addr: &libproc::libproc::net_info::InSIAddr) -> Ipv6Addr {
    let octets = unsafe { addr.ina_6.s6_addr };
    Ipv6Addr::from(octets)
}

/// Map `TcpSIState` to our `TcpState`.
fn tcp_state_from_si(state: TcpSIState) -> TcpState {
    match state {
        TcpSIState::Closed => TcpState::Closed,
        TcpSIState::Listen => TcpState::Listen,
        TcpSIState::SynSent => TcpState::SynSent,
        TcpSIState::SynReceived => TcpState::SynRecv,
        TcpSIState::Established => TcpState::Established,
        TcpSIState::CloseWait => TcpState::CloseWait,
        TcpSIState::FinWait1 => TcpState::FinWait1,
        TcpSIState::Closing => TcpState::Closing,
        TcpSIState::LastAck => TcpState::LastAck,
        TcpSIState::FinWait2 => TcpState::FinWait2,
        TcpSIState::TimeWait => TcpState::TimeWait,
        _ => TcpState::Unknown("UNKNOWN".to_string()),
    }
}

/// Build an `OpenFileInfo` from a vnode FD.
fn open_file_from_vnode(fd_num: i32, pid: i32) -> Option<OpenFileInfo> {
    let info: VnodeFdInfoWithPath =
        unsafe { raw_pidfdinfo(pid, fd_num, PROC_PIDFDVNODEPATHINFO)? };

    let mode = fd_mode_from_openflags(info.pfi.fi_openflags);
    let file_type = file_type_from_vtype(info.pvip.vip_vi.vi_type);
    let path = path_from_c_buf(&info.pvip.vip_path);
    let stat = &info.pvip.vip_vi.vi_stat;

    // Resolve symlink target if the vnode is a symbolic link.
    let link_target = if file_type == FileType::Link {
        std::fs::read_link(&path).ok().map(|p| p.to_string_lossy().into_owned())
    } else {
        None
    };

    Some(OpenFileInfo {
        fd: FdType::Numbered(fd_num as u32, mode),
        file_type,
        device: format_device(stat.vst_dev),
        size_off: Some(stat.vst_size as u64),
        node: stat.vst_ino.to_string(),
        name: path,
        mode: Some(mode),
        link_target,
    })
}

/// Build an `OpenFileInfo` from a socket FD.
fn open_file_from_socket(fd_num: i32, pid: i32) -> Option<OpenFileInfo> {
    let sock: SocketFDInfo = pidfdinfo(pid, fd_num).ok()?;
    let si = &sock.psi;
    let kind: SocketInfoKind = si.soi_kind.into();

    match kind {
        SocketInfoKind::Tcp => {
            let tcp = unsafe { si.soi_proto.pri_tcp };
            let ini = &tcp.tcpsi_ini;
            let is_v6 = ini.insi_vflag & INI_IPV6 != 0;
            let state = tcp_state_from_si(TcpSIState::from(tcp.tcpsi_state));

            let (local_addr, remote_addr, file_type) = if is_v6 {
                (
                    ipv6_from_insi(&ini.insi_laddr).to_string(),
                    ipv6_from_insi(&ini.insi_faddr).to_string(),
                    FileType::IPv6,
                )
            } else {
                (
                    ipv4_from_insi(&ini.insi_laddr).to_string(),
                    ipv4_from_insi(&ini.insi_faddr).to_string(),
                    FileType::IPv4,
                )
            };

            let lport = port_from_nbo(ini.insi_lport);
            let fport = port_from_nbo(ini.insi_fport);

            let name = format!(
                "{}:{} -> {}:{} ({})",
                local_addr, lport, remote_addr, fport, state
            );

            Some(OpenFileInfo {
                fd: FdType::Numbered(fd_num as u32, FdMode::ReadWrite),
                file_type,
                device: String::new(),
                size_off: None,
                node: "TCP".to_string(),
                name,
                mode: Some(FdMode::ReadWrite),
                link_target: None,
            })
        }
        SocketInfoKind::In => {
            // UDP or raw IP socket
            let ini = unsafe { si.soi_proto.pri_in };
            let is_v6 = ini.insi_vflag & INI_IPV6 != 0;

            let (local_addr, remote_addr, file_type) = if is_v6 {
                (
                    ipv6_from_insi(&ini.insi_laddr).to_string(),
                    ipv6_from_insi(&ini.insi_faddr).to_string(),
                    FileType::IPv6,
                )
            } else {
                (
                    ipv4_from_insi(&ini.insi_laddr).to_string(),
                    ipv4_from_insi(&ini.insi_faddr).to_string(),
                    FileType::IPv4,
                )
            };

            let lport = port_from_nbo(ini.insi_lport);
            let fport = port_from_nbo(ini.insi_fport);

            let proto_label = if si.soi_protocol == IPPROTO_UDP {
                "UDP"
            } else {
                "IP"
            };

            let name = format!(
                "{}:{} -> {}:{} ({})",
                local_addr, lport, remote_addr, fport, proto_label
            );

            Some(OpenFileInfo {
                fd: FdType::Numbered(fd_num as u32, FdMode::ReadWrite),
                file_type,
                device: String::new(),
                size_off: None,
                node: proto_label.to_string(),
                name,
                mode: Some(FdMode::ReadWrite),
                link_target: None,
            })
        }
        SocketInfoKind::Un => {
            let un = unsafe { si.soi_proto.pri_un };
            let sun = unsafe { un.unsi_addr.ua_sun };
            let path = if sun.sun_len > 0 {
                let ptr = sun.sun_path.as_ptr();
                unsafe { CStr::from_ptr(ptr) }
                    .to_string_lossy()
                    .into_owned()
            } else {
                String::new()
            };

            let name = if path.is_empty() {
                format!("unix socket 0x{:x}", un.unsi_conn_so)
            } else {
                path
            };

            Some(OpenFileInfo {
                fd: FdType::Numbered(fd_num as u32, FdMode::ReadWrite),
                file_type: FileType::Unix,
                device: String::new(),
                size_off: None,
                node: "unix".to_string(),
                name,
                mode: Some(FdMode::ReadWrite),
                link_target: None,
            })
        }
        SocketInfoKind::KernCtl => {
            let kctl = unsafe { si.soi_proto.pri_kern_ctl };
            let name_ptr = kctl.kcsi_name.as_ptr();
            let ctl_name = unsafe { CStr::from_ptr(name_ptr) }
                .to_string_lossy()
                .into_owned();

            Some(OpenFileInfo {
                fd: FdType::Numbered(fd_num as u32, FdMode::ReadWrite),
                file_type: FileType::Systm,
                device: String::new(),
                size_off: None,
                node: "kctl".to_string(),
                name: ctl_name,
                mode: Some(FdMode::ReadWrite),
                link_target: None,
            })
        }
        _ => {
            // Generic / Ndrv / KernEvent / Unknown
            Some(OpenFileInfo {
                fd: FdType::Numbered(fd_num as u32, FdMode::ReadWrite),
                file_type: FileType::Sock,
                device: String::new(),
                size_off: None,
                node: format!("{:?}", kind),
                name: format!("socket (kind={:?})", kind),
                mode: Some(FdMode::ReadWrite),
                link_target: None,
            })
        }
    }
}

/// Build an `OpenFileInfo` from a pipe FD.
fn open_file_from_pipe(fd_num: i32, pid: i32) -> Option<OpenFileInfo> {
    let info: PipeFdInfo =
        unsafe { raw_pidfdinfo(pid, fd_num, PROC_PIDFDPIPEINFO)? };

    let mode = fd_mode_from_openflags(info.pfi.fi_openflags);
    let stat = &info.pipeinfo.pipe_stat;

    Some(OpenFileInfo {
        fd: FdType::Numbered(fd_num as u32, mode),
        file_type: FileType::Pipe,
        device: String::new(),
        size_off: Some(stat.vst_size as u64),
        node: stat.vst_ino.to_string(),
        name: format!(
            "pipe 0x{:x} -> 0x{:x}",
            info.pipeinfo.pipe_handle, info.pipeinfo.pipe_peerhandle
        ),
        mode: Some(mode),
        link_target: None,
    })
}

/// Build an `OpenFileInfo` from a kqueue FD.
fn open_file_from_kqueue(fd_num: i32, pid: i32) -> Option<OpenFileInfo> {
    let info: KqueueFdInfo =
        unsafe { raw_pidfdinfo(pid, fd_num, PROC_PIDFDKQUEUEINFO)? };

    let mode = fd_mode_from_openflags(info.pfi.fi_openflags);

    Some(OpenFileInfo {
        fd: FdType::Numbered(fd_num as u32, mode),
        file_type: FileType::Kqueue,
        device: String::new(),
        size_off: None,
        node: "kqueue".to_string(),
        name: format!("count={}, state=0x{:x}", info.kqueueinfo.kq_stat.vst_size, info.kqueueinfo.kq_state),
        mode: Some(mode),
        link_target: None,
    })
}

/// Collect `NetworkInfo` entries from a single socket FD.
fn network_info_from_socket(fd_num: i32, pid: i32, command: Option<&str>) -> Option<NetworkInfo> {
    let sock: SocketFDInfo = pidfdinfo(pid, fd_num).ok()?;
    let si = &sock.psi;
    let kind: SocketInfoKind = si.soi_kind.into();

    match kind {
        SocketInfoKind::Tcp => {
            let tcp = unsafe { si.soi_proto.pri_tcp };
            let ini = &tcp.tcpsi_ini;
            let is_v6 = ini.insi_vflag & INI_IPV6 != 0;
            let state = tcp_state_from_si(TcpSIState::from(tcp.tcpsi_state));

            let (local_addr, remote_addr, protocol) = if is_v6 {
                (
                    ipv6_from_insi(&ini.insi_laddr).to_string(),
                    ipv6_from_insi(&ini.insi_faddr).to_string(),
                    Protocol::Tcp6,
                )
            } else {
                (
                    ipv4_from_insi(&ini.insi_laddr).to_string(),
                    ipv4_from_insi(&ini.insi_faddr).to_string(),
                    Protocol::Tcp,
                )
            };

            Some(NetworkInfo {
                protocol,
                local_addr,
                local_port: port_from_nbo(ini.insi_lport),
                remote_addr,
                remote_port: port_from_nbo(ini.insi_fport),
                state,
                pid: Some(pid as u32),
                command: command.map(|s| s.to_string()),
            })
        }
        SocketInfoKind::In if si.soi_protocol == IPPROTO_UDP => {
            let ini = unsafe { si.soi_proto.pri_in };
            let is_v6 = ini.insi_vflag & INI_IPV6 != 0;

            let (local_addr, remote_addr, protocol) = if is_v6 {
                (
                    ipv6_from_insi(&ini.insi_laddr).to_string(),
                    ipv6_from_insi(&ini.insi_faddr).to_string(),
                    Protocol::Udp6,
                )
            } else {
                (
                    ipv4_from_insi(&ini.insi_laddr).to_string(),
                    ipv4_from_insi(&ini.insi_faddr).to_string(),
                    Protocol::Udp,
                )
            };

            Some(NetworkInfo {
                protocol,
                local_addr,
                local_port: port_from_nbo(ini.insi_lport),
                remote_addr,
                remote_port: port_from_nbo(ini.insi_fport),
                state: TcpState::Unknown("NONE".to_string()),
                pid: Some(pid as u32),
                command: command.map(|s| s.to_string()),
            })
        }
        SocketInfoKind::Un => {
            let un = unsafe { si.soi_proto.pri_un };
            let sun = unsafe { un.unsi_addr.ua_sun };
            let path = if sun.sun_len > 0 {
                let ptr = sun.sun_path.as_ptr();
                unsafe { CStr::from_ptr(ptr) }
                    .to_string_lossy()
                    .into_owned()
            } else {
                String::new()
            };

            Some(NetworkInfo {
                protocol: Protocol::Unix,
                local_addr: path,
                local_port: 0,
                remote_addr: String::new(),
                remote_port: 0,
                state: TcpState::Unknown("NONE".to_string()),
                pid: Some(pid as u32),
                command: command.map(|s| s.to_string()),
            })
        }
        _ => None,
    }
}

/// Get the list of FDs for a process. Returns an empty vec on error
/// (e.g. permission denied for system processes).
fn get_fd_list(pid: i32) -> Vec<ProcFDInfo> {
    // Try to get the number of open files from BSDInfo first for a good
    // capacity hint; fall back to a reasonable default.
    let max_fds = pidinfo::<TaskAllInfo>(pid, 0)
        .map(|info| info.pbsd.pbi_nfiles as usize)
        .unwrap_or(256);

    listpidinfo::<ListFDs>(pid, max_fds).unwrap_or_default()
}

pub struct MacosProvider;

impl MacosProvider {
    pub fn new() -> Self {
        Self
    }
}

impl PlatformProvider for MacosProvider {
    fn list_processes(&self) -> Result<Vec<ProcessInfo>> {
        let mut sys = System::new_all();
        sys.refresh_processes();

        let mut processes = Vec::new();

        for (pid, proc_info) in sys.processes() {
            let pid_val = pid.as_u32();
            let uid = proc_info.user_id()
                .map(|u| **u)
                .unwrap_or(0);
            let user = users::get_user_by_uid(uid)
                .map(|u| u.name().to_string_lossy().to_string())
                .unwrap_or_else(|| uid.to_string());

            let comm = proc_info.name().to_string();
            let cmd_parts: Vec<String> = proc_info.cmd().iter()
                .map(|s| s.to_string())
                .collect();
            let command = if cmd_parts.is_empty() {
                comm.clone()
            } else {
                cmd_parts.join(" ")
            };

            let ppid = proc_info.parent().map(|p| p.as_u32());

            processes.push(ProcessInfo {
                pid: pid_val,
                ppid,
                command,
                comm,
                user,
                uid,
                open_files: Vec::new(),
            });
        }

        Ok(processes)
    }

    fn list_open_files(&self, pid: u32) -> Result<Vec<OpenFileInfo>> {
        let pid_i32 = pid as i32;
        let fds = get_fd_list(pid_i32);
        let mut results = Vec::with_capacity(fds.len() + 1);

        // Add the process executable as a "txt" entry.
        if let Ok(exe_path) = pidpath(pid_i32) {
            results.push(OpenFileInfo {
                fd: FdType::Txt,
                file_type: FileType::Reg,
                device: String::new(),
                size_off: None,
                node: String::new(),
                name: exe_path,
                mode: Some(FdMode::Read),
                link_target: None,
            });
        }

        for fd in &fds {
            let fd_num = fd.proc_fd;
            let fd_type: ProcFDType = fd.proc_fdtype.into();

            let info = match fd_type {
                ProcFDType::VNode => open_file_from_vnode(fd_num, pid_i32),
                ProcFDType::Socket => open_file_from_socket(fd_num, pid_i32),
                ProcFDType::Pipe => open_file_from_pipe(fd_num, pid_i32),
                ProcFDType::KQueue => open_file_from_kqueue(fd_num, pid_i32),
                _ => {
                    // PSHM, PSEM, FSEvents, ATalk, etc. -- record as unknown
                    Some(OpenFileInfo {
                        fd: FdType::Numbered(fd_num as u32, FdMode::Unknown),
                        file_type: FileType::Unknown(format!("{:?}", fd_type)),
                        device: String::new(),
                        size_off: None,
                        node: String::new(),
                        name: format!("{:?} fd={}", fd_type, fd_num),
                        mode: Some(FdMode::Unknown),
                        link_target: None,
                    })
                }
            };

            if let Some(entry) = info {
                results.push(entry);
            }
            // If we failed to read a particular FD (permission denied, race
            // condition where the FD was closed), we silently skip it.
        }

        Ok(results)
    }

    fn list_network_connections(&self, pid: Option<u32>) -> Result<Vec<NetworkInfo>> {
        let mut connections = Vec::new();

        match pid {
            Some(target_pid) => {
                let pid_i32 = target_pid as i32;
                let comm = pidpath(pid_i32).ok();
                let comm_ref = comm.as_deref();
                let fds = get_fd_list(pid_i32);

                for fd in &fds {
                    let fd_type: ProcFDType = fd.proc_fdtype.into();
                    if let ProcFDType::Socket = fd_type {
                        if let Some(net) = network_info_from_socket(fd.proc_fd, pid_i32, comm_ref) {
                            connections.push(net);
                        }
                    }
                }
            }
            None => {
                // Iterate all processes and collect network connections.
                let procs = self.list_processes()?;
                for p in &procs {
                    let pid_i32 = p.pid as i32;
                    let fds = get_fd_list(pid_i32);
                    for fd in &fds {
                        let fd_type: ProcFDType = fd.proc_fdtype.into();
                        if let ProcFDType::Socket = fd_type {
                            if let Some(net) = network_info_from_socket(
                                fd.proc_fd,
                                pid_i32,
                                Some(&p.comm),
                            ) {
                                connections.push(net);
                            }
                        }
                    }
                }
            }
        }

        Ok(connections)
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