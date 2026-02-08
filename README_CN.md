# loof

用 Rust 编写的现代化跨平台 `lsof` 替代工具，支持交互式 TUI 模式和模糊搜索。

## 特性

- **完整 CLI 兼容** — 支持常用 lsof 命令行参数，可直接替换使用
- **交互式 TUI** — fzf 风格的模糊搜索 + 详情面板（`-I` 参数）
- **彩色文件类型** — 不同文件类型（REG、DIR、SOCK、PIPE、IPv4 等）用不同颜色区分
- **跨平台** — macOS（完整实现）、Linux（开发中）
- **高性能** — 原生 Rust 性能，macOS 上使用零拷贝 FFI

## 快速开始

```bash
# 构建
cargo build --release

# CLI 模式（兼容 lsof）
loof -p 1234
loof -u root -c nginx -a
loof -i TCP -n -P
loof +D /var/log

# 交互式 TUI 模式
loof -I
```

## TUI 界面示例

### 搜索视图
```
  PID   COMMAND          USER        FDs
  1234  nginx            root         47
  5678  postgres         postgres     23
  9012  node             firshme      15
  > ngi_
  1/142 matches
```

### 详情视图（4 个标签页）
```
 PID: 1234  CMD: nginx  USER: root
 [打开文件] [网络连接] [文件树] [摘要]
 ─────────────────────────────────────────────
  FD      TYPE   DEVICE       SIZE/OFF  NODE       NAME
  0r      CHR    1,3          0         149        /dev/null
  1w      REG    1,17         4.2K      12345      /var/log/nginx/access.log
  3u      IPv4   0x1234       0t0       TCP        *:80 (LISTEN)
  5u      unix   0x5678       0t0                  /var/run/nginx.sock
 ─────────────────────────────────────────────
  ↑↓ 滚动  Tab 切换标签  Esc 返回  q 退出
```

## CLI 参数

### 进程筛选

| 参数 | 说明 | 示例 |
|------|------|------|
| `-p` | 按 PID 筛选（逗号分隔，`^` 排除） | `-p 1234,5678` 或 `-p ^1234` |
| `-u` | 按用户筛选 | `-u root,www` 或 `-u ^root` |
| `-c` | 按命令名筛选（前缀匹配） | `-c nginx` |
| `-a` | AND 模式（默认为 OR） | `-u root -c nginx -a` |

### 网络与文件筛选

| 参数 | 说明 | 示例 |
|------|------|------|
| `-i` | 选择网络文件（可选：TCP/UDP/4/6） | `-i` 或 `-i TCP` |
| `+D` | 递归搜索目录树 | `+D /var/log` |
| `+d` | 非递归搜索目录 | `+d /tmp` |
| 文件名 | 位置参数 | `loof /var/log/syslog` |

### 输出格式

| 参数 | 说明 | 示例 |
|------|------|------|
| `-t` | 精简输出（仅 PID） | `-t` |
| `-n` | 不解析主机名 | `-n` |
| `-P` | 不解析端口名 | `-P` |
| `-l` | 显示 UID 而非用户名 | `-l` |
| `-R` | 显示 PPID 列 | `-R` |
| `-F` | 字段输出模式 | `-F pcn` |
| `+c` | 命令名宽度 | `+c 15` |
| `-r` | 重复间隔（秒） | `-r 2` |

### 交互模式

| 参数 | 说明 |
|------|------|
| `-I` / `--interactive` | 进入 TUI 交互模式 |

## TUI 快捷键

| 按键 | 搜索视图 | 详情视图 |
|------|---------|---------|
| 字母/数字 | 输入搜索 | — |
| `↑`/`↓` 或 `k`/`j` | 移动选择 | 滚动内容 |
| `PgUp`/`PgDn` | 翻页 | 翻页 |
| `Enter` | 进入详情 | — |
| `Tab`/`Shift+Tab` | — | 切换标签页 |
| `Esc` | 清空搜索/退出 | 返回搜索 |
| `Ctrl+U` | 清空搜索 | — |
| `q` | 退出 | 退出 |
| `Ctrl+R` | 刷新进程列表 | — |

## 功能对比：loof vs lsof

### 核心功能

| 功能 | loof | lsof | 备注 |
|------|:----:|:----:|------|
| 进程列表 | ✅ | ✅ | PID、命令、用户、PPID |
| 打开文件枚举（macOS） | ✅ | ✅ | 通过 FFI 调用 `proc_pidfdinfo` |
| 打开文件枚举（Linux） | 🚧 | ✅ | 仅 stub，待实现 `/proc/[pid]/fd` |
| 文件类型检测 | ✅ | ✅ | 13 种类型：REG、DIR、CHR、BLK、FIFO、SOCK、LINK、PIPE、IPv4、IPv6、Unix、Kqueue、Systm |
| 网络连接检测 | ✅ | ✅ | 从 Socket FD 提取 TCP/UDP/Unix |
| 符号链接解析 | ✅ | ✅ | `link_target` 字段 |
| FD 访问模式（r/w/u） | ✅ | ✅ | 读/写/读写 |

### CLI 参数兼容性

| 参数 | loof | lsof | 说明 |
|------|:----:|:----:|------|
| `-p` | ✅ | ✅ | PID 筛选（包含/排除） |
| `-u` | ✅ | ✅ | 用户筛选（包含/排除） |
| `-c` | ✅ | ✅ | 命令筛选（前缀匹配） |
| `-i` | ✅ | ✅ | 网络文件选择 |
| `-t` | ✅ | ✅ | 精简输出（仅 PID） |
| `-n` | ✅ | ✅ | 不解析主机名 |
| `-P` | ✅ | ✅ | 不解析端口名 |
| `-l` | ✅ | ✅ | 显示 UID |
| `-R` | ✅ | ✅ | 显示 PPID 列 |
| `-F` | ✅ | ✅ | 字段输出模式 |
| `-a` | ✅ | ✅ | AND 模式 |
| `-r` | ✅ | ✅ | 重复模式 |
| `+D` | ✅ | ✅ | 递归目录搜索 |
| `+d` | ✅ | ✅ | 非递归目录搜索 |
| `+c` | ✅ | ✅ | 命令名宽度 |
| `-L` | ❌ | ✅ | 跟踪符号链接 |
| `-w` | ❌ | ✅ | 抑制警告 |
| `-g` | ❌ | ✅ | 进程组筛选 |
| `-s` | ❌ | ✅ | 文件大小筛选 |
| `-T` | ❌ | ✅ | TCP/TPI 详细信息 |
| `-b` | ❌ | ✅ | 避免内核阻塞 |
| `-S` | ❌ | ✅ | 避免 stat 调用 |
| `-x` | ❌ | ✅ | 跨文件系统/挂载点 |

### loof 独有功能（lsof 不具备）

| 功能 | 说明 |
|------|------|
| 交互式 TUI（`-I`） | fzf 风格模糊搜索，实时过滤 |
| 详情视图 | 4 个标签页：打开文件、网络连接、文件树、摘要 |
| 彩色文件类型 | 每种文件类型使用不同颜色显示 |
| 模糊匹配 | 基于 SkimMatcherV2 的近似搜索 |
| 文件树视图 | 按目录层级分组展示 |
| FD 统计 | 按类型计数 + 磁盘占用汇总 |

### 进度总览

```
loof 实现进度
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

CLI 参数兼容    ████████████████░░░░  15/22 flags (68%)
macOS 平台      ████████████████████  完整实现 ✓
Linux 平台      ████░░░░░░░░░░░░░░░░  进程列表完成，FD 枚举待实现
TUI 交互模式    ████████████████████  完整实现 ✓ (lsof 无此功能)
测试覆盖        ████████████████████  58 个测试全部通过 ✓
```

### 平台支持

| 平台 | 进程列表 | FD 枚举 | 网络检测 | 状态 |
|------|:-------:|:------:|:-------:|------|
| macOS | ✅ | ✅ | ✅ | **完整实现** |
| Linux | ✅ | 🚧 | 🚧 | 进程列表可用；FD 枚举待实现 |

## 项目结构

```
src/
├── main.rs              # 入口，CLI/TUI 模式分发
├── cli.rs               # clap 参数解析 + 预处理器
├── error.rs             # 错误类型（thiserror）
├── event.rs             # crossterm 事件处理
├── filter.rs            # 过滤引擎（PID/用户/命令/网络/目录）
├── output.rs            # 输出格式化（标准/精简/字段）
├── model/
│   ├── process.rs       # ProcessInfo 进程信息
│   ├── open_file.rs     # OpenFileInfo 打开文件信息、FileType、FdType
│   └── network.rs       # NetworkInfo 网络信息、Protocol、TcpState
├── platform/
│   ├── mod.rs           # PlatformProvider trait 平台抽象
│   ├── macos.rs         # macOS FFI 实现（748 行）
│   └── linux.rs         # Linux procfs 实现
├── app/
│   ├── state.rs         # AppState 应用状态、ViewMode、DetailTab
│   └── action.rs        # Action 枚举、按键映射
└── ui/
    ├── search_view.rs   # fzf 风格搜索界面
    ├── detail_view.rs   # 标签页详情面板
    ├── theme.rs         # 颜色方案（按 FileType 区分）
    └── widgets/
        ├── file_table.rs  # 打开文件表格
        ├── net_table.rs   # 网络连接表格
        ├── file_tree.rs   # 层级文件树
        └── summary.rs     # 进程信息 + FD 统计
```

## 依赖

| 库 | 用途 |
|----|------|
| clap 4 | CLI 参数解析 |
| ratatui 0.30 | TUI 渲染框架 |
| crossterm 0.28 | 终端事件处理 |
| sysinfo 0.30 | 进程发现 |
| fuzzy-matcher 0.3 | 模糊搜索 |
| thiserror 2 | 错误处理 |
| users 0.11 | UID/用户名解析 |
| nix 0.29 | Unix 系统调用 |
| procfs 0.17 | Linux `/proc`（仅 Linux） |
| libproc 0.14 | macOS `libproc`（仅 macOS） |

## 测试覆盖

- **41 个单元测试** — 过滤器解析、匹配逻辑、边界情况
- **8 个 CLI 集成测试** — 输出格式、参数、表头
- **7 个过滤器集成测试** — PID/用户/命令/网络/AND 模式
- **2 个平台测试** — 进程列表、PID 筛选
- **共计 58 个测试**，全部通过

## 构建

```bash
# 调试构建
cargo build

# 发布构建
cargo build --release

# 运行测试
cargo test

# 启动 TUI
cargo run -- -I
```

## 开发路线

- [ ] Linux 打开文件描述符枚举（`/proc/[pid]/fd`）
- [ ] Linux 网络连接解析（`/proc/net/tcp|udp`）
- [ ] `-L` 跟踪符号链接
- [ ] `-g` 进程组筛选
- [ ] `-s` 文件大小筛选
- [ ] `-T` TCP/TPI 详细信息
- [ ] TUI 选中结果导出/管道传递

## 许可证

MIT
