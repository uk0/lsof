use clap::Parser;

#[derive(Parser, Debug)]
#[command(name = "loof", version, about = "A modern lsof replacement with interactive TUI")]
pub struct CliArgs {
    /// Select by PID (comma-separated, prefix ^ to exclude)
    #[arg(short = 'p', value_name = "PID")]
    pub pid: Option<String>,

    /// Select IPv[46] files
    #[arg(short = 'i', value_name = "SPEC", num_args = 0..=1, default_missing_value = "")]
    pub inet: Option<String>,

    /// Select by user (comma-separated, prefix ^ to exclude)
    #[arg(short = 'u', value_name = "USER")]
    pub user: Option<String>,

    /// Select by command name (prefix match, prefix ^ to exclude)
    #[arg(short = 'c', value_name = "CMD")]
    pub command: Option<String>,

    /// Terse output: PIDs only
    #[arg(short = 't')]
    pub terse: bool,

    /// No hostname resolution
    #[arg(short = 'n')]
    pub no_hostname: bool,

    /// No port name resolution
    #[arg(short = 'P')]
    pub no_portname: bool,

    /// AND selections (default is OR)
    #[arg(short = 'a')]
    pub and_mode: bool,

    /// Enter interactive TUI mode
    #[arg(short = 'I', long = "interactive")]
    pub interactive: bool,

    /// List UID numbers instead of login names
    #[arg(short = 'l')]
    pub list_uid: bool,

    /// Show parent PID (PPID) column
    #[arg(short = 'R')]
    pub show_ppid: bool,

    /// Field output mode (specify field characters)
    #[arg(short = 'F', value_name = "FIELDS")]
    pub field_output: Option<String>,

    /// Repeat mode interval in seconds
    #[arg(short = 'r', value_name = "SECONDS")]
    pub repeat: Option<u64>,

    /// FD set filter
    #[arg(short = 'd', value_name = "FD")]
    pub fd_filter: Option<String>,

    /// Search directory tree recursively (+D)
    #[arg(long = "dir-tree", value_name = "DIR")]
    pub dir_tree: Option<String>,

    /// Search directory non-recursively (+d)
    #[arg(long = "dir", value_name = "DIR")]
    pub dir: Option<String>,

    /// Command name width (+c)
    #[arg(long = "cmd-width", value_name = "WIDTH")]
    pub cmd_width: Option<usize>,

    /// Suppress warnings
    #[arg(short = 'w')]
    pub suppress_warnings: bool,

    /// Select by process group ID (comma-separated, prefix ^ to exclude)
    #[arg(short = 'g', value_name = "PGID")]
    pub pgid: Option<String>,

    /// File size filter (prefix: +=greater, -=less, exact match)
    #[arg(short = 's', value_name = "SIZE")]
    pub size_filter: Option<String>,

    /// Avoid kernel blocks (compatibility, no-op)
    #[arg(short = 'b')]
    pub avoid_blocking: bool,

    /// Cross filesystem/mountpoint (compatibility, no-op)
    #[arg(short = 'x')]
    pub cross_fs: bool,

    /// Avoid stat() calls on files
    #[arg(short = 'S')]
    pub avoid_stat: bool,

    /// Follow symbolic links
    #[arg(short = 'L')]
    pub follow_symlinks: bool,

    /// TCP/TPI info (s=state, q=queue sizes)
    #[arg(short = 'T', value_name = "INFO", num_args = 0..=1, default_missing_value = "s")]
    pub tcp_info: Option<String>,

    /// Positional: file names to search for
    pub names: Vec<String>,
}

/// Preprocess command-line arguments to convert lsof-style `+` prefix flags
/// into clap-compatible `--long` flags before parsing.
///
/// Conversions:
///   +D DIR   -> --dir-tree DIR
///   +d DIR   -> --dir DIR
///   +c WIDTH -> --cmd-width WIDTH
pub fn preprocess_args(args: Vec<String>) -> Vec<String> {
    let mut result = Vec::with_capacity(args.len());
    let mut iter = args.into_iter();

    // Always keep the program name as-is
    if let Some(prog) = iter.next() {
        result.push(prog);
    }

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "+D" => {
                result.push("--dir-tree".to_string());
                if let Some(val) = iter.next() {
                    result.push(val);
                }
            }
            "+d" => {
                result.push("--dir".to_string());
                if let Some(val) = iter.next() {
                    result.push(val);
                }
            }
            "+c" => {
                result.push("--cmd-width".to_string());
                if let Some(val) = iter.next() {
                    result.push(val);
                }
            }
            _ => {
                result.push(arg);
            }
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_preprocess_plus_d_uppercase() {
        let args = vec![
            "loof".to_string(),
            "+D".to_string(),
            "/tmp".to_string(),
        ];
        let result = preprocess_args(args);
        assert_eq!(result, vec!["loof", "--dir-tree", "/tmp"]);
    }

    #[test]
    fn test_preprocess_plus_d_lowercase() {
        let args = vec![
            "loof".to_string(),
            "+d".to_string(),
            "/var".to_string(),
        ];
        let result = preprocess_args(args);
        assert_eq!(result, vec!["loof", "--dir", "/var"]);
    }

    #[test]
    fn test_preprocess_plus_c() {
        let args = vec![
            "loof".to_string(),
            "+c".to_string(),
            "15".to_string(),
        ];
        let result = preprocess_args(args);
        assert_eq!(result, vec!["loof", "--cmd-width", "15"]);
    }

    #[test]
    fn test_preprocess_mixed_args() {
        let args = vec![
            "loof".to_string(),
            "-p".to_string(),
            "1234".to_string(),
            "+D".to_string(),
            "/tmp".to_string(),
            "-t".to_string(),
        ];
        let result = preprocess_args(args);
        assert_eq!(result, vec!["loof", "-p", "1234", "--dir-tree", "/tmp", "-t"]);
    }

    #[test]
    fn test_preprocess_no_plus_flags() {
        let args = vec![
            "loof".to_string(),
            "-p".to_string(),
            "1234".to_string(),
            "-n".to_string(),
        ];
        let result = preprocess_args(args.clone());
        assert_eq!(result, args);
    }
}
