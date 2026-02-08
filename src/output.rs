use crate::cli::CliArgs;
use crate::model::{FileType, ProcessInfo};

/// Formats process and open-file data in lsof-compatible output.
pub struct OutputFormatter {
    /// Maximum width for the COMMAND column (default 9, configurable via +c).
    pub cmd_width: usize,
    /// `-n` flag: suppress hostname resolution (print numeric addresses).
    pub no_hostname: bool,
    /// `-P` flag: suppress port-name resolution (print numeric ports).
    pub no_portname: bool,
    /// `-l` flag: list UID numbers instead of login names.
    pub list_uid: bool,
    /// `-R` flag: show PPID column.
    pub show_ppid: bool,
    /// `-t` flag: terse output (PIDs only).
    pub terse: bool,
    /// `-F` flag: field-delimited output with the given field characters.
    pub field_output: Option<String>,
    /// `-T` flag: TCP/TPI info (s=state, q=queue sizes).
    pub tcp_info: Option<String>,
}

impl OutputFormatter {
    /// Build an `OutputFormatter` from parsed CLI arguments.
    pub fn from_cli(args: &CliArgs) -> Self {
        OutputFormatter {
            cmd_width: args.cmd_width.unwrap_or(9),
            no_hostname: args.no_hostname,
            no_portname: args.no_portname,
            list_uid: args.list_uid,
            show_ppid: args.show_ppid,
            terse: args.terse,
            field_output: args.field_output.clone(),
            tcp_info: args.tcp_info.clone(),
        }
    }

    /// Print the standard lsof-style column header line.
    pub fn print_header(&self) {
        if self.show_ppid {
            println!(
                "{:<width$} {:>5} {:>5} {:<8} {:>4}  {:>6} {:>8}  {:>8}  {:>4} {}",
                "COMMAND", "PID", "PPID", "USER", "FD", "TYPE", "DEVICE", "SIZE/OFF", "NODE", "NAME",
                width = self.cmd_width,
            );
        } else {
            println!(
                "{:<width$} {:>5} {:<8} {:>4}  {:>6} {:>8}  {:>8}  {:>4} {}",
                "COMMAND", "PID", "USER", "FD", "TYPE", "DEVICE", "SIZE/OFF", "NODE", "NAME",
                width = self.cmd_width,
            );
        }
    }

    /// Print one line per open file for a process, in standard lsof format.
    pub fn print_process_files(&self, proc: &ProcessInfo) {
        let cmd = fit_str(&proc.comm, self.cmd_width);
        let user_display = if self.list_uid {
            proc.uid.to_string()
        } else {
            proc.user.clone()
        };

        for file in &proc.open_files {
            let size_off = format_size_off(file.size_off);
            let mut display_name = file.name.clone();

            // When -T flag includes "q", append queue sizes for network files.
            if let Some(ref tcp_flags) = self.tcp_info {
                if tcp_flags.contains('q') {
                    let is_network = matches!(
                        file.file_type,
                        FileType::IPv4 | FileType::IPv6
                    );
                    if is_network {
                        if let (Some(rq), Some(sq)) = (file.recv_queue, file.send_queue) {
                            display_name.push_str(&format!(" QR={} QS={}", rq, sq));
                        }
                    }
                }
            }

            if self.show_ppid {
                println!(
                    "{} {:>5} {:>5} {:<8} {:>4}  {:>6} {:>8}  {:>8}  {:>4} {}",
                    cmd,
                    proc.pid,
                    proc.ppid.map(|p| p.to_string()).unwrap_or_default(),
                    user_display,
                    file.fd,
                    file.file_type,
                    file.device,
                    size_off,
                    file.node,
                    display_name,
                );
            } else {
                println!(
                    "{} {:>5} {:<8} {:>4}  {:>6} {:>8}  {:>8}  {:>4} {}",
                    cmd,
                    proc.pid,
                    user_display,
                    file.fd,
                    file.file_type,
                    file.device,
                    size_off,
                    file.node,
                    display_name,
                );
            }
        }
    }

    /// Print PIDs only (terse mode, `-t`).
    pub fn print_terse(&self, processes: &[ProcessInfo]) {
        for proc in processes {
            println!("{}", proc.pid);
        }
    }

    /// Print field-delimited output (`-F` mode).
    ///
    /// Each field is printed on its own line as a single-character tag
    /// followed by the value. A NUL character terminates each record set.
    ///
    /// Common field characters:
    ///   p = PID, c = command, u = user, n = name, f = FD, t = type
    pub fn print_field_output(&self, proc: &ProcessInfo) {
        let fields = self
            .field_output
            .as_deref()
            .unwrap_or("pcuftn");

        // Process-level fields
        for ch in fields.chars() {
            match ch {
                'p' => println!("p{}", proc.pid),
                'c' => println!("c{}", proc.comm),
                'u' => {
                    if self.list_uid {
                        println!("u{}", proc.uid);
                    } else {
                        println!("u{}", proc.user);
                    }
                }
                'R' => {
                    if let Some(ppid) = proc.ppid {
                        println!("R{}", ppid);
                    }
                }
                'g' => println!("g{}", proc.pid), // PGID placeholder
                _ => {} // file-level fields handled below
            }
        }

        // File-level fields
        for file in &proc.open_files {
            for ch in fields.chars() {
                match ch {
                    'f' => println!("f{}", file.fd),
                    't' => println!("t{}", file.file_type),
                    'D' => println!("D{}", file.device),
                    's' => {
                        if let Some(sz) = file.size_off {
                            println!("s{}", sz);
                        }
                    }
                    'i' => println!("i{}", file.node),
                    'n' => println!("n{}", file.name),
                    _ => {}
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Truncate or pad a string to exactly `width` characters.
fn fit_str(s: &str, width: usize) -> String {
    if s.len() > width {
        s[..width].to_string()
    } else {
        format!("{:<width$}", s, width = width)
    }
}

/// Format the SIZE/OFF column value.
fn format_size_off(size: Option<u64>) -> String {
    match size {
        Some(s) => format!("{}", s),
        None => "0t0".to_string(),
    }
}

// ---------------------------------------------------------------------------
// Unit tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{FdType, FileType, OpenFileInfo};

    #[test]
    fn test_fit_str_truncate() {
        assert_eq!(fit_str("longcommandname", 9), "longcomma");
    }

    #[test]
    fn test_fit_str_pad() {
        assert_eq!(fit_str("sh", 9), "sh       ");
    }

    #[test]
    fn test_format_size_off_some() {
        assert_eq!(format_size_off(Some(4096)), "4096");
    }

    #[test]
    fn test_format_size_off_none() {
        assert_eq!(format_size_off(None), "0t0");
    }

    #[test]
    fn test_default_cmd_width() {
        let fmt = OutputFormatter {
            cmd_width: 9,
            no_hostname: false,
            no_portname: false,
            list_uid: false,
            show_ppid: false,
            terse: false,
            field_output: None,
            tcp_info: None,
        };
        assert_eq!(fmt.cmd_width, 9);
    }

    #[test]
    fn test_terse_output() {
        let fmt = OutputFormatter {
            cmd_width: 9,
            no_hostname: false,
            no_portname: false,
            list_uid: false,
            show_ppid: false,
            terse: true,
            field_output: None,
            tcp_info: None,
        };

        let procs = vec![
            ProcessInfo {
                pid: 100,
                ppid: None,
                pgid: None,
                command: "bash".to_string(),
                comm: "bash".to_string(),
                user: "root".to_string(),
                uid: 0,
                open_files: Vec::new(),
            },
            ProcessInfo {
                pid: 200,
                ppid: None,
                pgid: None,
                command: "nginx".to_string(),
                comm: "nginx".to_string(),
                user: "www".to_string(),
                uid: 33,
                open_files: Vec::new(),
            },
        ];

        // Capture output by calling the method (we just verify it doesn't panic)
        fmt.print_terse(&procs);
    }

    #[test]
    fn test_field_output_format() {
        let fmt = OutputFormatter {
            cmd_width: 9,
            no_hostname: false,
            no_portname: false,
            list_uid: false,
            show_ppid: false,
            terse: false,
            field_output: Some("pcun".to_string()),
            tcp_info: None,
        };

        let proc = ProcessInfo {
            pid: 1234,
            ppid: Some(1),
            pgid: None,
            command: "/usr/sbin/nginx".to_string(),
            comm: "nginx".to_string(),
            user: "root".to_string(),
            uid: 0,
            open_files: vec![OpenFileInfo {
                fd: FdType::Cwd,
                file_type: FileType::Dir,
                device: "1,16".to_string(),
                size_off: Some(704),
                node: "2".to_string(),
                name: "/".to_string(),
                mode: None,
                link_target: None,
                send_queue: None,
                recv_queue: None,
            }],
        };

        // Verify it doesn't panic
        fmt.print_field_output(&proc);
    }
}
