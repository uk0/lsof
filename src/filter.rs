use std::path::PathBuf;

use crate::cli::CliArgs;
use crate::error::{LoofError, Result};
use crate::model::{FileType, OpenFileInfo, ProcessInfo};

/// Top-level filter configuration built from CLI arguments.
#[derive(Debug, Default)]
pub struct FilterConfig {
    pub pids: Option<PidFilter>,
    pub pgids: Option<PgidFilter>,
    pub users: Option<UserFilter>,
    pub commands: Option<CommandFilter>,
    pub inet: Option<InetFilter>,
    pub dir_tree: Option<PathBuf>,
    pub dir: Option<PathBuf>,
    pub names: Vec<PathBuf>,
    pub and_mode: bool,
    pub size_filter: Option<SizeFilter>,
}

/// PID-based filter with include/exclude lists.
#[derive(Debug, Default)]
pub struct PidFilter {
    pub include: Vec<u32>,
    pub exclude: Vec<u32>,
}

/// PGID-based filter with include/exclude lists.
#[derive(Debug, Default)]
pub struct PgidFilter {
    pub include: Vec<u32>,
    pub exclude: Vec<u32>,
}

/// Size comparison operator.
#[derive(Debug)]
pub enum SizeOp {
    GreaterThan,
    LessThan,
    Exact,
}

/// File size filter.
#[derive(Debug)]
pub struct SizeFilter {
    pub op: SizeOp,
    pub bytes: u64,
}

/// User-based filter with include/exclude lists.
#[derive(Debug, Default)]
pub struct UserFilter {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

/// Command-name filter with include/exclude lists (prefix match).
#[derive(Debug, Default)]
pub struct CommandFilter {
    pub include: Vec<String>,
    pub exclude: Vec<String>,
}

/// Network/inet filter parsed from `-i` spec.
#[derive(Debug, Default)]
pub struct InetFilter {
    /// TCP, UDP, or None for all protocols
    pub protocol: Option<String>,
    /// Host to match
    pub host: Option<String>,
    /// Port to match
    pub port: Option<u16>,
    /// IP version: 4 or 6, or None for both
    pub ip_version: Option<u8>,
}

// ---------------------------------------------------------------------------
// Parsing helpers
// ---------------------------------------------------------------------------

/// Parse a PID filter string.
///
/// Format: comma-separated PIDs, prefix `^` to exclude.
/// Examples: "1234,5678", "^1234", "1234,^5678"
fn parse_pid_filter(s: &str) -> Result<PidFilter> {
    let mut filter = PidFilter::default();
    for token in s.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if let Some(rest) = token.strip_prefix('^') {
            let pid: u32 = rest
                .parse()
                .map_err(|_| LoofError::Parse(format!("invalid PID: {}", rest)))?;
            filter.exclude.push(pid);
        } else {
            let pid: u32 = token
                .parse()
                .map_err(|_| LoofError::Parse(format!("invalid PID: {}", token)))?;
            filter.include.push(pid);
        }
    }
    Ok(filter)
}

/// Parse a PGID filter string.
///
/// Format: comma-separated PGIDs, prefix `^` to exclude.
/// Examples: "1234,5678", "^1234", "1234,^5678"
fn parse_pgid_filter(s: &str) -> Result<PgidFilter> {
    let mut filter = PgidFilter::default();
    for token in s.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if let Some(rest) = token.strip_prefix('^') {
            let pgid: u32 = rest
                .parse()
                .map_err(|_| LoofError::Parse(format!("invalid PGID: {}", rest)))?;
            filter.exclude.push(pgid);
        } else {
            let pgid: u32 = token
                .parse()
                .map_err(|_| LoofError::Parse(format!("invalid PGID: {}", token)))?;
            filter.include.push(pgid);
        }
    }
    Ok(filter)
}

/// Parse a size filter string.
///
/// Format: `[+|-]SIZE[K|KB|M|MB|G|GB]`
///
/// Prefix `+` means greater-than, `-` means less-than, no prefix means exact.
/// Suffixes: K/KB = 1024, M/MB = 1048576, G/GB = 1073741824.
fn parse_size_filter(s: &str) -> Option<SizeFilter> {
    let s = s.trim();
    if s.is_empty() {
        return None;
    }

    let (op, rest) = if let Some(r) = s.strip_prefix('+') {
        (SizeOp::GreaterThan, r)
    } else if let Some(r) = s.strip_prefix('-') {
        (SizeOp::LessThan, r)
    } else {
        (SizeOp::Exact, s)
    };

    // Split numeric part from suffix
    let rest = rest.trim();
    let num_end = rest
        .find(|c: char| !c.is_ascii_digit())
        .unwrap_or(rest.len());
    let (num_str, suffix) = rest.split_at(num_end);
    let base: u64 = num_str.parse().ok()?;

    let multiplier: u64 = match suffix.to_uppercase().as_str() {
        "" => 1,
        "K" | "KB" => 1024,
        "M" | "MB" => 1_048_576,
        "G" | "GB" => 1_073_741_824,
        _ => return None,
    };

    Some(SizeFilter {
        op,
        bytes: base * multiplier,
    })
}

/// Parse a user filter string.
///
/// Format: comma-separated user names, prefix `^` to exclude.
/// Examples: "root,www", "^root"
fn parse_user_filter(s: &str) -> UserFilter {
    let mut filter = UserFilter::default();
    for token in s.split(',') {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }
        if let Some(rest) = token.strip_prefix('^') {
            filter.exclude.push(rest.to_string());
        } else {
            filter.include.push(token.to_string());
        }
    }
    filter
}

/// Parse a command filter string.
///
/// Format: command name prefix, prefix `^` to exclude.
/// Examples: "nginx", "^nginx"
fn parse_command_filter(s: &str) -> CommandFilter {
    let mut filter = CommandFilter::default();
    let token = s.trim();
    if token.is_empty() {
        return filter;
    }
    if let Some(rest) = token.strip_prefix('^') {
        filter.exclude.push(rest.to_string());
    } else {
        filter.include.push(token.to_string());
    }
    filter
}

impl FilterConfig {
    /// Build a `FilterConfig` from parsed CLI arguments.
    pub fn from_cli(args: &CliArgs) -> Result<Self> {
        let pids = match &args.pid {
            Some(s) => Some(parse_pid_filter(s)?),
            None => None,
        };

        let pgids = match &args.pgid {
            Some(s) => Some(parse_pgid_filter(s)?),
            None => None,
        };

        let users = args.user.as_ref().map(|s| parse_user_filter(s));
        let commands = args.command.as_ref().map(|s| parse_command_filter(s));
        let inet = args.inet.as_ref().map(|s| parse_inet_filter(s));
        let size_filter = args.size_filter.as_ref().and_then(|s| parse_size_filter(s));

        let dir_tree = args.dir_tree.as_ref().map(PathBuf::from);
        let dir = args.dir.as_ref().map(PathBuf::from);
        let names = args.names.iter().map(PathBuf::from).collect();

        Ok(FilterConfig {
            pids,
            pgids,
            users,
            commands,
            inet,
            dir_tree,
            dir,
            names,
            and_mode: args.and_mode,
            size_filter,
        })
    }

    /// Returns `true` if no filters are configured at all.
    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.pids.is_none()
            && self.pgids.is_none()
            && self.users.is_none()
            && self.commands.is_none()
            && self.inet.is_none()
            && self.dir_tree.is_none()
            && self.dir.is_none()
            && self.names.is_empty()
            && self.size_filter.is_none()
    }

    /// Check whether a process matches the configured process-level filters
    /// (PID, user, command). In OR mode (default) any matching filter is
    /// sufficient; in AND mode all active filters must match.
    pub fn matches_process(&self, proc: &ProcessInfo) -> bool {
        // If no process-level filters are set, everything matches.
        if self.pids.is_none()
            && self.pgids.is_none()
            && self.users.is_none()
            && self.commands.is_none()
        {
            return true;
        }

        let pid_match = self.check_pid(proc);
        let pgid_match = self.check_pgid(proc);
        let user_match = self.check_user(proc);
        let cmd_match = self.check_command(proc);

        if self.and_mode {
            // AND: every *active* filter must match
            let mut pass = true;
            if self.pids.is_some() {
                pass = pass && pid_match;
            }
            if self.pgids.is_some() {
                pass = pass && pgid_match;
            }
            if self.users.is_some() {
                pass = pass && user_match;
            }
            if self.commands.is_some() {
                pass = pass && cmd_match;
            }
            pass
        } else {
            // OR: at least one active filter must match
            let mut any = false;
            if self.pids.is_some() {
                any = any || pid_match;
            }
            if self.pgids.is_some() {
                any = any || pgid_match;
            }
            if self.users.is_some() {
                any = any || user_match;
            }
            if self.commands.is_some() {
                any = any || cmd_match;
            }
            any
        }
    }

    /// Check whether an open file matches the configured file-level filters
    /// (inet, directory, names).
    pub fn matches_file(&self, file: &OpenFileInfo) -> bool {
        // If no file-level filters are set, everything matches.
        if self.inet.is_none()
            && self.dir_tree.is_none()
            && self.dir.is_none()
            && self.names.is_empty()
            && self.size_filter.is_none()
        {
            return true;
        }

        let mut results: Vec<bool> = Vec::new();

        if let Some(ref inet) = self.inet {
            results.push(inet.matches_file(file));
        }
        if let Some(ref dir_tree) = self.dir_tree {
            results.push(file_in_dir_tree(&file.name, dir_tree));
        }
        if let Some(ref dir) = self.dir {
            results.push(file_in_dir(&file.name, dir));
        }
        if !self.names.is_empty() {
            results.push(self.names.iter().any(|n| {
                let n_str = n.to_string_lossy();
                file.name == *n_str
            }));
        }
        if let Some(ref sf) = self.size_filter {
            if let Some(size) = file.size_off {
                let matches = match sf.op {
                    SizeOp::GreaterThan => size > sf.bytes,
                    SizeOp::LessThan => size < sf.bytes,
                    SizeOp::Exact => size == sf.bytes,
                };
                results.push(matches);
            } else {
                results.push(false);
            }
        }

        if results.is_empty() {
            return true;
        }

        if self.and_mode {
            results.iter().all(|&r| r)
        } else {
            results.iter().any(|&r| r)
        }
    }

    // -- private helpers --

    fn check_pid(&self, proc: &ProcessInfo) -> bool {
        match &self.pids {
            None => true,
            Some(f) => {
                if f.exclude.contains(&proc.pid) {
                    return false;
                }
                if f.include.is_empty() {
                    true
                } else {
                    f.include.contains(&proc.pid)
                }
            }
        }
    }

    fn check_pgid(&self, proc: &ProcessInfo) -> bool {
        match &self.pgids {
            None => true,
            Some(f) => {
                let pgid = match proc.pgid {
                    Some(p) => p,
                    None => return f.include.is_empty(),
                };
                if f.exclude.contains(&pgid) {
                    return false;
                }
                if f.include.is_empty() {
                    true
                } else {
                    f.include.contains(&pgid)
                }
            }
        }
    }

    fn check_user(&self, proc: &ProcessInfo) -> bool {
        match &self.users {
            None => true,
            Some(f) => {
                if f.exclude.contains(&proc.user) {
                    return false;
                }
                if f.include.is_empty() {
                    true
                } else {
                    f.include.contains(&proc.user)
                }
            }
        }
    }

    fn check_command(&self, proc: &ProcessInfo) -> bool {
        match &self.commands {
            None => true,
            Some(f) => {
                if f.exclude.iter().any(|c| proc.comm.starts_with(c.as_str())) {
                    return false;
                }
                if f.include.is_empty() {
                    true
                } else {
                    f.include.iter().any(|c| proc.comm.starts_with(c.as_str()))
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Inet filter matching
// ---------------------------------------------------------------------------

impl InetFilter {
    /// Check whether an open file matches this inet filter.
    pub fn matches_file(&self, file: &OpenFileInfo) -> bool {
        // Must be a network file type
        match file.file_type {
            FileType::IPv4 | FileType::IPv6 | FileType::Sock | FileType::Unix => {}
            _ => return false,
        }

        // Check IP version
        if let Some(ver) = self.ip_version {
            match ver {
                4 => {
                    if file.file_type != FileType::IPv4 {
                        return false;
                    }
                }
                6 => {
                    if file.file_type != FileType::IPv6 {
                        return false;
                    }
                }
                _ => {}
            }
        }

        // Check protocol
        if let Some(ref proto) = self.protocol {
            let proto_upper = proto.to_uppercase();
            let node_upper = file.node.to_uppercase();
            if !node_upper.contains(&proto_upper) {
                return false;
            }
        }

        // Check port (appears in the name as ":PORT")
        if let Some(port) = self.port {
            let port_str = format!(":{}", port);
            if !file.name.contains(&port_str) {
                return false;
            }
        }

        // Check host
        if let Some(ref host) = self.host {
            if !file.name.contains(host) {
                return false;
            }
        }

        true
    }
}

// ---------------------------------------------------------------------------
// Inet spec parser
// ---------------------------------------------------------------------------

/// Parse an inet filter spec string.
///
/// Format: `[46][protocol][@host][:port]`
///
/// Examples:
///   ""                -> match all network files
///   "TCP"             -> match TCP
///   "TCP:80"          -> match TCP port 80
///   "6TCP@localhost:443" -> match IPv6 TCP to localhost port 443
///   "UDP"             -> match UDP
///   ":8080"           -> match any protocol on port 8080
///   "@192.168.1.1"    -> match any protocol to host
fn parse_inet_filter(s: &str) -> InetFilter {
    let mut filter = InetFilter::default();
    if s.is_empty() {
        return filter;
    }

    let mut remaining = s;

    // Check for leading IP version digit (4 or 6)
    if remaining.starts_with('4') || remaining.starts_with('6') {
        let ch = remaining.as_bytes()[0];
        filter.ip_version = Some(ch - b'0');
        remaining = &remaining[1..];
    }

    // Extract protocol (letters before @ or :)
    let proto_end = remaining.find(['@', ':']).unwrap_or(remaining.len());
    if proto_end > 0 {
        let proto = &remaining[..proto_end];
        if !proto.is_empty() {
            filter.protocol = Some(proto.to_uppercase());
        }
        remaining = &remaining[proto_end..];
    }

    // Extract host (@host before :)
    if remaining.starts_with('@') {
        remaining = &remaining[1..]; // skip '@'
        let host_end = remaining.find(':').unwrap_or(remaining.len());
        if host_end > 0 {
            filter.host = Some(remaining[..host_end].to_string());
        }
        remaining = &remaining[host_end..];
    }

    // Extract port (:port)
    if remaining.starts_with(':') {
        remaining = &remaining[1..]; // skip ':'
        if let Ok(port) = remaining.parse::<u16>() {
            filter.port = Some(port);
        }
    }

    filter
}

// ---------------------------------------------------------------------------
// Directory matching helpers
// ---------------------------------------------------------------------------

/// Check if a file path is inside a directory tree (recursive).
fn file_in_dir_tree(file_name: &str, dir: &std::path::Path) -> bool {
    let dir_str = dir.to_string_lossy();
    let dir_prefix = if dir_str.ends_with('/') {
        dir_str.to_string()
    } else {
        format!("{}/", dir_str)
    };
    file_name.starts_with(&dir_prefix) || file_name == dir_str.as_ref()
}

/// Check if a file path is directly inside a directory (non-recursive).
fn file_in_dir(file_name: &str, dir: &std::path::Path) -> bool {
    let path = std::path::Path::new(file_name);
    if let Some(parent) = path.parent() {
        let dir_canonical = dir.to_string_lossy();
        let parent_str = parent.to_string_lossy();
        parent_str == dir_canonical
    } else {
        false
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FdMode, FdType};

    // -- PID filter parsing --

    #[test]
    fn test_parse_pid_include() {
        let f = parse_pid_filter("1234,5678").unwrap();
        assert_eq!(f.include, vec![1234, 5678]);
        assert!(f.exclude.is_empty());
    }

    #[test]
    fn test_parse_pid_exclude() {
        let f = parse_pid_filter("^1234").unwrap();
        assert!(f.include.is_empty());
        assert_eq!(f.exclude, vec![1234]);
    }

    #[test]
    fn test_parse_pid_mixed() {
        let f = parse_pid_filter("100,^200,300").unwrap();
        assert_eq!(f.include, vec![100, 300]);
        assert_eq!(f.exclude, vec![200]);
    }

    #[test]
    fn test_parse_pid_invalid() {
        assert!(parse_pid_filter("abc").is_err());
    }

    // -- User filter parsing --

    #[test]
    fn test_parse_user_include() {
        let f = parse_user_filter("root,www");
        assert_eq!(f.include, vec!["root", "www"]);
        assert!(f.exclude.is_empty());
    }

    #[test]
    fn test_parse_user_exclude() {
        let f = parse_user_filter("^root");
        assert!(f.include.is_empty());
        assert_eq!(f.exclude, vec!["root"]);
    }

    #[test]
    fn test_parse_user_mixed() {
        let f = parse_user_filter("admin,^nobody,www");
        assert_eq!(f.include, vec!["admin", "www"]);
        assert_eq!(f.exclude, vec!["nobody"]);
    }

    // -- Command filter parsing --

    #[test]
    fn test_parse_command_include() {
        let f = parse_command_filter("nginx");
        assert_eq!(f.include, vec!["nginx"]);
        assert!(f.exclude.is_empty());
    }

    #[test]
    fn test_parse_command_exclude() {
        let f = parse_command_filter("^nginx");
        assert!(f.include.is_empty());
        assert_eq!(f.exclude, vec!["nginx"]);
    }

    // -- Inet filter parsing --

    #[test]
    fn test_parse_inet_empty() {
        let f = parse_inet_filter("");
        assert!(f.protocol.is_none());
        assert!(f.host.is_none());
        assert!(f.port.is_none());
        assert!(f.ip_version.is_none());
    }

    #[test]
    fn test_parse_inet_tcp_port() {
        let f = parse_inet_filter("TCP:80");
        assert_eq!(f.protocol, Some("TCP".to_string()));
        assert_eq!(f.port, Some(80));
        assert!(f.host.is_none());
        assert!(f.ip_version.is_none());
    }

    #[test]
    fn test_parse_inet_ipv6_tcp_host_port() {
        let f = parse_inet_filter("6TCP@localhost:443");
        assert_eq!(f.ip_version, Some(6));
        assert_eq!(f.protocol, Some("TCP".to_string()));
        assert_eq!(f.host, Some("localhost".to_string()));
        assert_eq!(f.port, Some(443));
    }

    #[test]
    fn test_parse_inet_udp() {
        let f = parse_inet_filter("UDP");
        assert_eq!(f.protocol, Some("UDP".to_string()));
        assert!(f.host.is_none());
        assert!(f.port.is_none());
        assert!(f.ip_version.is_none());
    }

    #[test]
    fn test_parse_inet_port_only() {
        let f = parse_inet_filter(":8080");
        assert!(f.protocol.is_none());
        assert!(f.host.is_none());
        assert_eq!(f.port, Some(8080));
    }

    #[test]
    fn test_parse_inet_host_only() {
        let f = parse_inet_filter("@192.168.1.1");
        assert!(f.protocol.is_none());
        assert_eq!(f.host, Some("192.168.1.1".to_string()));
        assert!(f.port.is_none());
    }

    #[test]
    fn test_parse_inet_ipv4_only() {
        let f = parse_inet_filter("4");
        assert_eq!(f.ip_version, Some(4));
        assert!(f.protocol.is_none());
    }

    // -- Filter matching logic --

    fn make_proc(pid: u32, user: &str, comm: &str) -> ProcessInfo {
        ProcessInfo {
            pid,
            ppid: None,
            pgid: None,
            command: comm.to_string(),
            comm: comm.to_string(),
            user: user.to_string(),
            uid: 0,
            open_files: Vec::new(),
        }
    }

    fn make_file(name: &str, file_type: FileType) -> OpenFileInfo {
        OpenFileInfo {
            fd: FdType::Numbered(0, FdMode::Read),
            file_type,
            device: String::new(),
            size_off: None,
            node: String::new(),
            name: name.to_string(),
            mode: Some(FdMode::Read),
            link_target: None,
            send_queue: None,
            recv_queue: None,
        }
    }

    #[test]
    fn test_matches_process_pid_include() {
        let config = FilterConfig {
            pids: Some(PidFilter {
                include: vec![100, 200],
                exclude: vec![],
            }),
            ..Default::default()
        };
        assert!(config.matches_process(&make_proc(100, "root", "bash")));
        assert!(!config.matches_process(&make_proc(300, "root", "bash")));
    }

    #[test]
    fn test_matches_process_pid_exclude() {
        let config = FilterConfig {
            pids: Some(PidFilter {
                include: vec![],
                exclude: vec![100],
            }),
            ..Default::default()
        };
        assert!(!config.matches_process(&make_proc(100, "root", "bash")));
        assert!(config.matches_process(&make_proc(200, "root", "bash")));
    }

    #[test]
    fn test_matches_process_user() {
        let config = FilterConfig {
            users: Some(UserFilter {
                include: vec!["root".to_string()],
                exclude: vec![],
            }),
            ..Default::default()
        };
        assert!(config.matches_process(&make_proc(1, "root", "init")));
        assert!(!config.matches_process(&make_proc(2, "www", "nginx")));
    }

    #[test]
    fn test_matches_process_command_prefix() {
        let config = FilterConfig {
            commands: Some(CommandFilter {
                include: vec!["ngin".to_string()],
                exclude: vec![],
            }),
            ..Default::default()
        };
        assert!(config.matches_process(&make_proc(1, "root", "nginx")));
        assert!(!config.matches_process(&make_proc(2, "root", "bash")));
    }

    #[test]
    fn test_matches_process_and_mode() {
        let config = FilterConfig {
            pids: Some(PidFilter {
                include: vec![100],
                exclude: vec![],
            }),
            users: Some(UserFilter {
                include: vec!["root".to_string()],
                exclude: vec![],
            }),
            and_mode: true,
            ..Default::default()
        };
        // Both PID and user must match
        assert!(config.matches_process(&make_proc(100, "root", "bash")));
        assert!(!config.matches_process(&make_proc(100, "www", "bash")));
        assert!(!config.matches_process(&make_proc(200, "root", "bash")));
    }

    #[test]
    fn test_matches_process_or_mode() {
        let config = FilterConfig {
            pids: Some(PidFilter {
                include: vec![100],
                exclude: vec![],
            }),
            users: Some(UserFilter {
                include: vec!["www".to_string()],
                exclude: vec![],
            }),
            and_mode: false,
            ..Default::default()
        };
        // Either PID or user can match
        assert!(config.matches_process(&make_proc(100, "root", "bash")));
        assert!(config.matches_process(&make_proc(200, "www", "nginx")));
        assert!(!config.matches_process(&make_proc(200, "root", "bash")));
    }

    #[test]
    fn test_matches_file_name() {
        let config = FilterConfig {
            names: vec![PathBuf::from("/tmp/test.txt")],
            ..Default::default()
        };
        assert!(config.matches_file(&make_file("/tmp/test.txt", FileType::Reg)));
        assert!(!config.matches_file(&make_file("/tmp/other.txt", FileType::Reg)));
    }

    #[test]
    fn test_matches_file_dir_tree() {
        let config = FilterConfig {
            dir_tree: Some(PathBuf::from("/tmp")),
            ..Default::default()
        };
        assert!(config.matches_file(&make_file("/tmp/a/b/c.txt", FileType::Reg)));
        assert!(!config.matches_file(&make_file("/var/log/syslog", FileType::Reg)));
    }

    #[test]
    fn test_matches_file_dir_non_recursive() {
        let config = FilterConfig {
            dir: Some(PathBuf::from("/tmp")),
            ..Default::default()
        };
        assert!(config.matches_file(&make_file("/tmp/test.txt", FileType::Reg)));
        assert!(!config.matches_file(&make_file("/tmp/sub/test.txt", FileType::Reg)));
    }

    #[test]
    fn test_inet_filter_matches_ipv4_tcp() {
        let inet = InetFilter {
            protocol: Some("TCP".to_string()),
            port: Some(80),
            ..Default::default()
        };
        let mut file = make_file(
            "127.0.0.1:80 -> 10.0.0.1:12345 (ESTABLISHED)",
            FileType::IPv4,
        );
        file.node = "TCP".to_string();
        assert!(inet.matches_file(&file));
    }

    #[test]
    fn test_inet_filter_rejects_wrong_protocol() {
        let inet = InetFilter {
            protocol: Some("UDP".to_string()),
            ..Default::default()
        };
        let mut file = make_file("127.0.0.1:80 -> 10.0.0.1:12345", FileType::IPv4);
        file.node = "TCP".to_string();
        assert!(!inet.matches_file(&file));
    }

    #[test]
    fn test_inet_filter_rejects_non_network() {
        let inet = InetFilter::default();
        let file = make_file("/tmp/test.txt", FileType::Reg);
        assert!(!inet.matches_file(&file));
    }

    #[test]
    fn test_no_filters_matches_everything() {
        let config = FilterConfig::default();
        assert!(config.matches_process(&make_proc(1, "root", "init")));
        assert!(config.matches_file(&make_file("/any/path", FileType::Reg)));
    }

    // -- PGID filter parsing --

    #[test]
    fn test_parse_pgid_include() {
        let f = parse_pgid_filter("1234,5678").unwrap();
        assert_eq!(f.include, vec![1234, 5678]);
        assert!(f.exclude.is_empty());
    }

    #[test]
    fn test_parse_pgid_exclude() {
        let f = parse_pgid_filter("^1234").unwrap();
        assert!(f.include.is_empty());
        assert_eq!(f.exclude, vec![1234]);
    }

    // -- Size filter parsing --

    #[test]
    fn test_parse_size_filter_greater() {
        let f = parse_size_filter("+1024").unwrap();
        assert!(matches!(f.op, SizeOp::GreaterThan));
        assert_eq!(f.bytes, 1024);
    }

    #[test]
    fn test_parse_size_filter_less() {
        let f = parse_size_filter("-512").unwrap();
        assert!(matches!(f.op, SizeOp::LessThan));
        assert_eq!(f.bytes, 512);
    }

    #[test]
    fn test_parse_size_filter_exact() {
        let f = parse_size_filter("2048").unwrap();
        assert!(matches!(f.op, SizeOp::Exact));
        assert_eq!(f.bytes, 2048);
    }

    #[test]
    fn test_parse_size_filter_suffix() {
        let f = parse_size_filter("+10M").unwrap();
        assert!(matches!(f.op, SizeOp::GreaterThan));
        assert_eq!(f.bytes, 10 * 1_048_576);

        let f2 = parse_size_filter("5KB").unwrap();
        assert!(matches!(f2.op, SizeOp::Exact));
        assert_eq!(f2.bytes, 5 * 1024);

        let f3 = parse_size_filter("-2G").unwrap();
        assert!(matches!(f3.op, SizeOp::LessThan));
        assert_eq!(f3.bytes, 2 * 1_073_741_824);
    }

    // -- PGID filter matching --

    #[test]
    fn test_matches_process_pgid() {
        let config = FilterConfig {
            pgids: Some(PgidFilter {
                include: vec![42],
                exclude: vec![],
            }),
            ..Default::default()
        };
        let mut p = make_proc(1, "root", "bash");
        p.pgid = Some(42);
        assert!(config.matches_process(&p));

        let mut p2 = make_proc(2, "root", "bash");
        p2.pgid = Some(99);
        assert!(!config.matches_process(&p2));

        // Process with no pgid should not match when include list is non-empty
        let p3 = make_proc(3, "root", "bash");
        assert!(!config.matches_process(&p3));
    }
}
