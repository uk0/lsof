# loof

A modern, cross-platform replacement for `lsof` written in Rust, featuring an interactive TUI mode with fuzzy search.

## Features

- **Full CLI compatibility** — drop-in replacement for common `lsof` flags
- **Interactive TUI** — fzf-style fuzzy search with detail panels (`-I` flag)
- **Color-coded file types** — visual distinction between REG, DIR, SOCK, PIPE, IPv4, etc.
- **Cross-platform** — macOS (fully implemented), Linux (in progress)
- **Fast** — native Rust performance with zero-copy FFI on macOS

## Quick Start

```bash
# Build
cargo build --release

# CLI mode (lsof-compatible)
loof -p 1234
loof -u root -c nginx -a
loof -i TCP -n -P
loof +D /var/log

# Interactive TUI mode
loof -I
```

## TUI Screenshots

### Search View
```
  PID   COMMAND          USER        FDs
  1234  nginx            root         47
  5678  postgres         postgres     23
  9012  node             firshme      15
  > ngi_
  1/142 matches
```

### Detail View (4 tabs)
```
 PID: 1234  CMD: nginx  USER: root
 [Open Files] [Network] [File Tree] [Summary]
 ─────────────────────────────────────────────
  FD      TYPE   DEVICE       SIZE/OFF  NODE       NAME
  0r      CHR    1,3          0         149        /dev/null
  1w      REG    1,17         4.2K      12345      /var/log/nginx/access.log
  3u      IPv4   0x1234       0t0       TCP        *:80 (LISTEN)
  5u      unix   0x5678       0t0                  /var/run/nginx.sock
 ─────────────────────────────────────────────
  ↑↓ scroll  Tab switch  Esc back  q quit
```

## CLI Flags

### Process Selection

| Flag | Description | Example |
|------|-------------|---------|
| `-p` | Filter by PID (comma-separated, `^` to exclude) | `-p 1234,5678` or `-p ^1234` |
| `-u` | Filter by user | `-u root,www` or `-u ^root` |
| `-c` | Filter by command name (prefix match) | `-c nginx` |
| `-a` | AND mode (default is OR) | `-u root -c nginx -a` |

### Network & File Selection

| Flag | Description | Example |
|------|-------------|---------|
| `-i` | Select network files (optional: TCP/UDP/4/6) | `-i` or `-i TCP` |
| `+D` | Search directory tree (recursive) | `+D /var/log` |
| `+d` | Search directory (non-recursive) | `+d /tmp` |
| names | File names (positional) | `loof /var/log/syslog` |

### Output Formatting

| Flag | Description | Example |
|------|-------------|---------|
| `-t` | Terse output (PIDs only) | `-t` |
| `-n` | No hostname resolution | `-n` |
| `-P` | No port name resolution | `-P` |
| `-l` | Show UID instead of username | `-l` |
| `-R` | Show PPID column | `-R` |
| `-F` | Field output mode | `-F pcn` |
| `+c` | Command name width | `+c 15` |
| `-r` | Repeat interval (seconds) | `-r 2` |
| `-w` | Suppress warnings | `-w` |
| `-g` | Filter by process group ID | `-g 1234` or `-g ^1234` |
| `-s` | File size filter | `-s +10M` or `-s -1K` |
| `-b` | Avoid kernel blocks (no-op) | `-b` |
| `-x` | Cross filesystem (no-op) | `-x` |
| `-S` | Avoid stat() calls | `-S` |
| `-L` | Follow symbolic links | `-L` |
| `-T` | TCP/TPI info (queue sizes) | `-T` or `-Tq` |

### Interactive Mode

| Flag | Description |
|------|-------------|
| `-I` / `--interactive` | Enter TUI mode |

## TUI Keyboard Shortcuts

| Key | Search View | Detail View |
|-----|-------------|-------------|
| Letters/digits | Type search query | — |
| `↑`/`↓` or `k`/`j` | Move selection | Scroll content |
| `PgUp`/`PgDn` | Page scroll | Page scroll |
| `Enter` | Open detail view | — |
| `Tab`/`Shift+Tab` | — | Switch tabs |
| `Esc` | Clear search / Quit | Back to search |
| `Ctrl+U` | Clear search | — |
| `q` | Quit | Quit |
| `Ctrl+Y` | — | Yank selected line |
| `Ctrl+E` | — | Export process data |
| `Ctrl+R` | Refresh process list | — |

## Feature Comparison: loof vs lsof

### Core Functionality

| Feature | loof | lsof | Notes |
|---------|:----:|:----:|-------|
| Process listing | ✅ | ✅ | PID, command, user, PPID |
| Open file enumeration (macOS) | ✅ | ✅ | Via raw FFI `proc_pidfdinfo` |
| Open file enumeration (Linux) | ✅ | ✅ | Via `/proc/[pid]/fd` with procfs |
| File type detection | ✅ | ✅ | 13 types: REG, DIR, CHR, BLK, FIFO, SOCK, LINK, PIPE, IPv4, IPv6, Unix, Kqueue, Systm |
| Network connection detection | ✅ | ✅ | TCP/UDP/Unix socket from FDs |
| Symlink target resolution | ✅ | ✅ | `link_target` field |
| FD access mode (r/w/u) | ✅ | ✅ | Read/Write/ReadWrite |

### CLI Flag Compatibility

| Flag | loof | lsof | Description |
|------|:----:|:----:|-------------|
| `-p` | ✅ | ✅ | PID filter (include/exclude) |
| `-u` | ✅ | ✅ | User filter (include/exclude) |
| `-c` | ✅ | ✅ | Command filter (prefix match) |
| `-i` | ✅ | ✅ | Network file selection |
| `-t` | ✅ | ✅ | Terse output (PIDs only) |
| `-n` | ✅ | ✅ | No hostname resolution |
| `-P` | ✅ | ✅ | No port name resolution |
| `-l` | ✅ | ✅ | List UID numbers |
| `-R` | ✅ | ✅ | Show PPID column |
| `-F` | ✅ | ✅ | Field output mode |
| `-a` | ✅ | ✅ | AND mode |
| `-r` | ✅ | ✅ | Repeat mode |
| `+D` | ✅ | ✅ | Recursive directory search |
| `+d` | ✅ | ✅ | Non-recursive directory search |
| `+c` | ✅ | ✅ | Command name width |
| `-w` | ✅ | ✅ | Suppress warnings |
| `-g` | ✅ | ✅ | Process group filter |
| `-s` | ✅ | ✅ | File size filter |
| `-L` | ✅ | ✅ | Follow symbolic links |
| `-T` | ✅ | ✅ | TCP/TPI info |
| `-b` | ✅ | ✅ | Avoid kernel blocks |
| `-S` | ✅ | ✅ | Avoid stat calls |
| `-x` | ✅ | ✅ | Cross filesystem/mountpoint |

### Unique to loof (not in lsof)

| Feature | Description |
|---------|-------------|
| Interactive TUI (`-I`) | fzf-style fuzzy search with real-time filtering |
| Detail view | 4 tabbed panels: Open Files, Network, File Tree, Summary |
| Color-coded types | Each file type has a distinct color |
| Fuzzy matching | SkimMatcherV2 for approximate search |
| File tree view | Hierarchical directory-grouped display |
| FD statistics | Per-type counts and disk usage summary |
| Selection export | `Ctrl+Y` yank line, `Ctrl+E` export process data |

### Platform Support

| Platform | Process Listing | Open FD Enumeration | Network Detection | Status |
|----------|:-:|:-:|:-:|--------|
| macOS | ✅ | ✅ | ✅ | **Fully implemented** |
| Linux | ✅ | ✅ | ✅ | **Fully implemented** |

## Architecture

```
src/
├── main.rs              # Entry point, CLI/TUI dispatch
├── cli.rs               # clap argument parsing + preprocessor
├── error.rs             # Error types (thiserror)
├── event.rs             # Crossterm event handler
├── filter.rs            # Filter engine (PID/user/cmd/inet/dir)
├── output.rs            # Output formatter (standard/terse/field)
├── model/
│   ├── process.rs       # ProcessInfo
│   ├── open_file.rs     # OpenFileInfo, FileType, FdType
│   └── network.rs       # NetworkInfo, Protocol, TcpState
├── platform/
│   ├── mod.rs           # PlatformProvider trait
│   ├── macos.rs         # macOS FFI implementation (748 lines)
│   └── linux.rs         # Linux procfs implementation
├── app/
│   ├── state.rs         # AppState, ViewMode, DetailTab
│   └── action.rs        # Action enum, key mapping
└── ui/
    ├── search_view.rs   # fzf-style search interface
    ├── detail_view.rs   # Tabbed detail panels
    ├── theme.rs         # Color scheme (per-FileType)
    └── widgets/
        ├── file_table.rs  # Open files table
        ├── net_table.rs   # Network connections table
        ├── file_tree.rs   # Hierarchical file tree
        └── summary.rs     # Process info + FD statistics
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| clap 4 | CLI argument parsing |
| ratatui 0.30 | TUI rendering |
| crossterm 0.28 | Terminal events |
| sysinfo 0.30 | Process discovery |
| fuzzy-matcher 0.3 | Fuzzy search |
| thiserror 2 | Error handling |
| users 0.11 | UID/username resolution |
| nix 0.29 | Unix syscalls |
| procfs 0.17 | Linux `/proc` (Linux only) |
| libproc 0.14 | macOS `libproc` (macOS only) |

## Test Coverage

- **54 unit tests** — filter parsing, matching logic, edge cases, TUI export
- **13 CLI integration tests** — output format, flags, headers
- **7 filter integration tests** — PID/user/command/inet/AND mode
- **2 platform tests** — process listing, PID filtering
- **Total: 76 tests**, all passing

## Build

```bash
# Debug build
cargo build

# Release build
cargo build --release

# Run tests
cargo test

# Run with TUI
cargo run -- -I
```

## Roadmap

- [x] Linux open file descriptor enumeration via `/proc/[pid]/fd`
- [x] Linux network connection parsing via `/proc/net/tcp|udp`
- [x] `-L` follow symbolic links
- [x] `-g` process group filtering
- [x] `-s` file size filtering
- [x] `-T` TCP/TPI detailed info
- [x] Export/pipe TUI selection results

## License

MIT
