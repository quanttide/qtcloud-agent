use clap::{Parser, Subcommand};
use std::collections::BTreeMap;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

/// 量潮智能体注册中心 CLI
#[derive(Parser)]
#[command(name = "qtcloud-agent", version, about = "Agent 注册与契约管理")]
enum Cli {
    /// 查看注册中心与本地配置的状态对比
    Status {
        /// 注册中心路径（data/profile/）
        #[arg(long, default_value = "data/profile")]
        registry: PathBuf,
    },
    /// 本地 DSH（DeepSeek Harness）管理
    #[command(subcommand)]
    Dsh(DshCommand),
}

#[derive(Subcommand)]
enum DshCommand {
    /// 检查本地 DSH 安装、运行与数据目录状态
    Status {
        /// Web UI 端口
        #[arg(long, default_value_t = 3080)]
        port: u16,
    },
    /// 列出本地 DSH 会话（按工作区分组）
    Sessions,
    /// 显示 DSH 配置（settings.yaml 与 profile 插件清单）
    Config,
}

fn main() {
    let cli = Cli::parse();
    match cli {
        Cli::Status { registry } => status(&registry),
        Cli::Dsh(cmd) => match cmd {
            DshCommand::Status { port } => dsh_status(port),
            DshCommand::Sessions => dsh_sessions(),
            DshCommand::Config => dsh_config(),
        },
    }
}

// ─── 工具 ───

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

/// 已知 Agent 本地配置目录
fn agent_local_dir(name: &str) -> Option<PathBuf> {
    let home = home_dir()?;
    match name {
        "zed" => Some(home.join(".config/zed")),
        "opencode" => Some(home.join(".config/opencode")),
        "hermes" => Some(home.join(".config/hermes")),
        _ => None,
    }
}

/// 是否需要跳过（非配置类文件）
fn skip_file(name: &str) -> bool {
    matches!(name, "README.md" | "LICENSE" | ".gitkeep")
}

// ─── 配置对比 ───

/// 将 JSON Value 展平为 "key.path" → 字符串 的映射
fn flatten_json(value: &serde_json::Value, prefix: &str, out: &mut BTreeMap<String, String>) {
    match value {
        serde_json::Value::Object(map) => {
            for (k, v) in map {
                let key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{prefix}.{k}")
                };
                flatten_json(v, &key, out);
            }
        }
        serde_json::Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate() {
                let key = format!("{prefix}[{i}]");
                flatten_json(v, &key, out);
            }
        }
        other => {
            out.insert(prefix.to_string(), other.to_string());
        }
    }
}

/// 去除 JSONC 中的行注释（//）和尾逗号，返回标准 JSON
fn strip_jsonc(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut in_string = false;
    let mut chars = raw.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '"' {
            in_string = !in_string;
            out.push(c);
        } else if !in_string && c == '/' && chars.peek() == Some(&'/') {
            // 跳过行注释
            chars.next();
            while let Some(&c) = chars.peek() {
                if c == '\n' {
                    break;
                }
                chars.next();
            }
            out.push('\n');
        } else if !in_string && c == ',' {
            // 跳过尾逗号：后面跳过空白/换行后是 ] 或 }
            let mut lookahead = chars.clone();
            let skip = loop {
                match lookahead.next() {
                    Some(c) if c == '}' || c == ']' => break true,
                    Some(c) if c.is_whitespace() || c == '\n' || c == '\r' => continue,
                    _ => break false,
                }
            };
            if skip {
                continue;
            }
            out.push(c);
        } else {
            out.push(c);
        }
    }
    out
}

/// 解析 JSON/JSONC 文件，返回展平后的键值对
fn parse_json(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let raw =
        std::fs::read_to_string(path).map_err(|e| format!("无法读取 {}: {e}", path.display()))?;
    let cleaned = strip_jsonc(&raw);
    let value: serde_json::Value = serde_json::from_str(&cleaned)
        .map_err(|e| format!("JSON 解析失败 {}: {e}", path.display()))?;
    let mut flat = BTreeMap::new();
    flatten_json(&value, "", &mut flat);
    Ok(flat)
}

/// 对比配置结果
struct ConfigDiff {
    ok: usize,      // 一致项数
    diff: usize,    // 值不同
    missing: usize, // 本地缺失
    skipped: bool,  // 注册中心文件无法解析，跳过对比
}

/// 对比两份 JSON 配置
fn compare_json(
    reg_json: &BTreeMap<String, String>,
    local_json: &BTreeMap<String, String>,
) -> ConfigDiff {
    let mut ok = 0;
    let mut diff = 0;
    let mut missing = 0;

    for (key, reg_val) in reg_json {
        match local_json.get(key) {
            Some(local_val) if local_val == reg_val => ok += 1,
            Some(_local_val) => diff += 1,
            None => missing += 1,
        }
    }

    ConfigDiff {
        ok,
        diff,
        missing,
        skipped: false,
    }
}

/// 对比配置：注册中心的是标准，本地是否包含
fn compare_config(registry_path: &Path, local_path: &Path) -> ConfigDiff {
    let reg = match parse_json(registry_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("   ⚠ 注册中心文件解析失败: {e}");
            return ConfigDiff {
                ok: 0,
                diff: 0,
                missing: 0,
                skipped: true,
            };
        }
    };

    let local = match parse_json(local_path) {
        Ok(m) => m,
        Err(_) => {
            return ConfigDiff {
                ok: 0,
                diff: 0,
                missing: reg.len(),
                skipped: false,
            };
        }
    };

    compare_json(&reg, &local)
}

// ─── 状态展示（注册中心对比） ───

fn status(registry: &Path) {
    if !registry.is_dir() {
        eprintln!("错误: 注册中心路径不存在: {}", registry.display());
        std::process::exit(1);
    }

    let mut agents: Vec<_> = std::fs::read_dir(registry)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    agents.sort_by_key(|e| e.file_name());

    let mut total_ok = 0usize;
    let mut total_issues = 0usize;

    for entry in &agents {
        let name = entry.file_name();
        let agent_dir = entry.path();
        let local_dir = agent_local_dir(&name.to_string_lossy());

        println!("\n── {} ──", name.to_string_lossy());

        // 本地目录是否存在
        let local_ok = local_dir.as_ref().map(|d| d.is_dir()).unwrap_or(false);
        if !local_ok {
            match local_dir {
                Some(ref d) => println!("  本地目录不存在: {}", d.display()),
                None => println!("  未知 Agent，本地路径未定义"),
            }
        }

        // 遍历注册中心文件
        let mut files: Vec<_> = std::fs::read_dir(&agent_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .collect();
        files.sort_by_key(|e| e.file_name());

        for file in &files {
            let fname = file.file_name();
            let fname_str = fname.to_string_lossy();

            if skip_file(&fname_str) {
                println!("  · {:<30}   (文档，跳过)", fname_str);
                continue;
            }

            let Some(ref local_base) = local_dir else {
                println!("  ? {:<30}   未知本地路径", fname_str);
                continue;
            };
            let local_path = local_base.join(&*fname_str);

            if !local_path.is_file() {
                println!("  ✗ {:<30}   缺失", fname_str);
                total_issues += 1;
                continue;
            }

            // 根据扩展名选择对比方式
            let ext = fname_str.rsplit('.').next().unwrap_or("");
            match ext {
                "json" => {
                    let result = compare_config(&file.path(), &local_path);
                    total_ok += result.ok;
                    let issues = result.diff + result.missing;
                    total_issues += issues;
                    if result.skipped {
                        println!("  ? {:<30}   注册中心文件无法解析", fname_str);
                    } else if issues == 0 {
                        println!("  ✓ {:<30}   {} 项配置一致", fname_str, result.ok);
                    } else {
                        println!(
                            "  ⚠ {:<30}   {} 一致, {} 差异, {} 缺失",
                            fname_str, result.ok, result.diff, result.missing,
                        );
                    }
                }
                _ => {
                    // 非 JSON 文件只检查存在
                    println!("  ✓ {:<30}   存在", fname_str);
                    total_ok += 1;
                }
            }
        }

        // 检查本地是否有多余的配置目录（不比对内容）
        if let Some(ref base) = local_dir {
            if base.is_dir() {
                let local_files: Vec<_> = std::fs::read_dir(base)
                    .unwrap()
                    .filter_map(|e| e.ok())
                    .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                    .map(|e| e.file_name())
                    .filter(|n| !skip_file(&n.to_string_lossy()))
                    .collect();

                let reg_files: std::collections::HashSet<_> = files
                    .iter()
                    .map(|f| f.file_name())
                    .filter(|n| !skip_file(&n.to_string_lossy()))
                    .collect();

                for f in &local_files {
                    if !reg_files.contains(f) {
                        println!("  + {:<30}   本地自定义", f.to_string_lossy());
                    }
                }
            }
        }
    }

    // 总览
    println!("\n── 总览 ──");
    println!("  Agent:   {} 个已注册", agents.len());
    println!("  一致:    {} 项配置", total_ok);

    if total_issues == 0 {
        println!("\n  状态: ✓ 注册中心与本地完全一致");
    } else {
        println!("\n  状态: ⚠ {total_issues} 项不一致，需要同步");
    }
}

// ─── DSH 管理 ───

/// DSH 数据目录（$DSH_HOME 或 ~/.dsh）
fn dsh_home() -> PathBuf {
    if let Ok(h) = std::env::var("DSH_HOME") {
        if !h.is_empty() {
            return PathBuf::from(h);
        }
    }
    home_dir().unwrap_or_else(|| PathBuf::from(".")).join(".dsh")
}

/// 执行外部命令并捕获 stdout（去除首尾空白）
fn run_cmd(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

/// 从 package.json 读取 version 字段
fn pkg_version(dir: &Path) -> String {
    let raw = std::fs::read_to_string(dir.join("package.json")).unwrap_or_default();
    serde_json::from_str::<serde_json::Value>(&raw)
        .ok()
        .and_then(|v| v.get("version").and_then(|x| x.as_str()).map(String::from))
        .unwrap_or_else(|| "?".into())
}

/// 探测 DSH 安装目录与版本：`which dsh` → 解析 symlink → 包目录
fn dsh_install() -> Option<(PathBuf, String)> {
    let bin = run_cmd("which", &["dsh"])?;
    let resolved = std::fs::canonicalize(PathBuf::from(bin)).ok()?;
    let pkg = resolved.parent()?.parent()?; // .../@deepseek-ai/dsh/lib/bin.js → 包目录
    let ver = pkg_version(pkg);
    Some((pkg.to_path_buf(), ver))
}

/// 读取进程 VmRSS（/proc/<pid>/status）
fn vm_rss(pid: u32) -> String {
    let Ok(status) = std::fs::read_to_string(format!("/proc/{pid}/status")) else {
        return "-".into();
    };
    for line in status.lines() {
        if let Some(rest) = line.strip_prefix("VmRSS:") {
            return rest.trim().to_string();
        }
    }
    "-".into()
}

/// 扫描 /proc 查找 DSH 相关进程（dsh web / headless / --profile）
fn dsh_processes() -> Vec<(u32, String, String)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return out;
    };
    for e in entries.flatten() {
        let Ok(pid) = e.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let raw = std::fs::read(e.path().join("cmdline")).unwrap_or_default();
        let cmd = raw
            .split(|&b| b == 0)
            .filter(|s| !s.is_empty())
            .map(|s| String::from_utf8_lossy(s).into_owned())
            .collect::<Vec<_>>()
            .join(" ");
        if cmd.contains("dsh") && cmd.contains("web") {
            out.push((pid, cmd, vm_rss(pid)));
        }
    }
    out.sort();
    out
}

/// 探测 Web UI 是否响应，返回 HTTP 状态行
fn http_status(port: u16) -> Option<String> {
    let addr: SocketAddr = format!("127.0.0.1:{port}").parse().ok()?;
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_secs(2)).ok()?;
    s.set_read_timeout(Some(Duration::from_secs(2))).ok()?;
    s.write_all(b"GET / HTTP/1.0\r\n\r\n").ok()?;
    let mut buf = [0u8; 64];
    let n = s.read(&mut buf).ok()?;
    let head = String::from_utf8_lossy(&buf[..n]);
    head.lines().next().map(|l| l.to_string())
}

/// 递归计算目录大小
fn dir_size(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(rd) = std::fs::read_dir(path) {
        for e in rd.flatten() {
            let p = e.path();
            if p.is_dir() {
                total += dir_size(&p);
            } else if let Ok(m) = e.metadata() {
                total += m.len();
            }
        }
    }
    total
}

/// 人类可读的文件大小
fn human_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[u])
    }
}

/// 相对时间描述（基于 SystemTime 年龄）
fn rel_time(t: SystemTime) -> String {
    let Ok(age) = t.elapsed() else {
        return "-".into();
    };
    let s = age.as_secs();
    if s < 60 {
        format!("{s} 秒前")
    } else if s < 3600 {
        format!("{} 分钟前", s / 60)
    } else if s < 86400 {
        format!("{} 小时前", s / 3600)
    } else {
        format!("{} 天前", s / 86400)
    }
}

/// 解码会话目录名（`--home-iguo-repos-qt--` → `/home/iguo/repos/qt`）
fn decode_workspace(name: &str) -> String {
    if name == "--tmp--" {
        return "临时工作区".into();
    }
    let inner = name
        .strip_prefix("--")
        .and_then(|s| s.strip_suffix("--"))
        .unwrap_or(name);
    if inner.is_empty() {
        return name.into();
    }
    inner.replace('-', "/")
}

/// 从 session_projcache.json 读取 session id → 工作目录 映射
fn proj_cache_cwds(home: &Path) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let p = home.join("storages/session_projcache.json");
    let Ok(raw) = std::fs::read_to_string(p) else {
        return map;
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return map;
    };
    let Some(sessions) = v.pointer("/tables/sessions") else {
        return map;
    };
    if let Some(objs) = sessions.as_object() {
        for (sid, val) in objs {
            if let Some(cwd) = val.pointer("/identity/cwd").and_then(|c| c.as_str()) {
                map.insert(sid.clone(), cwd.to_string());
            }
        }
    }
    map
}

/// 读取 profile 的插件 bundle 清单（package.json 的 dsh.profile.bundles）
fn profile_bundles(pj: &Path) -> Vec<String> {
    let Ok(raw) = std::fs::read_to_string(pj) else {
        return Vec::new();
    };
    let Ok(v) = serde_json::from_str::<serde_json::Value>(&raw) else {
        return Vec::new();
    };
    v.pointer("/dsh/profile/bundles")
        .and_then(|b| b.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|x| x.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

// ─── DSH 状态 ───

fn dsh_status(port: u16) {
    let home = dsh_home();
    println!("── DSH 安装 ──");
    println!("  DSH_HOME: {}", home.display());
    if !home.is_dir() {
        println!("    ⚠ 目录不存在");
    }

    match dsh_install() {
        Some((dir, ver)) => println!("  安装:     {} (v{})", dir.display(), ver),
        None => println!("  安装:     (未找到 @deepseek-ai/dsh 包目录)"),
    }
    let bin = run_cmd("which", &["dsh"]);
    let cli_ver = run_cmd("dsh", &["--version"]);
    match (bin, cli_ver) {
        (Some(b), Some(v)) => println!("  CLI:      {} → v{v}", b),
        (Some(b), None) => println!("  CLI:      {} (版本查询失败)", b),
        _ => println!("  CLI:      dsh 不在 PATH 中"),
    }

    println!("\n── 运行状态 ──");
    let procs = dsh_processes();
    if procs.is_empty() {
        println!("  服务:     ✗ 未运行");
    } else {
        for (pid, cmd, rss) in &procs {
            println!("  服务:     ✓ PID {pid}  {cmd}  (RSS {rss})");
        }
    }
    match http_status(port) {
        Some(line) => println!("  Web UI:   http://127.0.0.1:{port}  →  {line}"),
        None => println!("  Web UI:   http://127.0.0.1:{port}  ✗ 无响应"),
    }

    println!("\n── 数据目录 ──");
    if home.is_dir() {
        let profiles = home.join("profiles");
        let sessions = home.join("sessions");
        if let Ok(rd) = std::fs::read_dir(&profiles) {
            let names: Vec<_> = rd
                .flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .filter(|n| n != "node_modules")
                .collect();
            println!(
                "  Profiles: {}",
                if names.is_empty() {
                    "（无）".into()
                } else {
                    names.join(", ")
                }
            );
        }
        let ws_count = std::fs::read_dir(&sessions)
            .map(|rd| rd.flatten().filter(|e| e.path().is_dir()).count())
            .unwrap_or(0);
        println!(
            "  会话:     {ws_count} 个工作区, 共 {}",
            human_size(dir_size(&sessions))
        );
        let ws_json = home.join("storages/workspace.json");
        if ws_json.is_file() {
            println!("  Workspace: {}", ws_json.display());
        }
        println!("  总计:     {}", human_size(dir_size(&home)));
    }
}

// ─── DSH 会话 ───

fn dsh_sessions() {
    let home = dsh_home();
    let sessions_dir = home.join("sessions");
    if !sessions_dir.is_dir() {
        eprintln!("错误: DSH 会话目录不存在: {}", sessions_dir.display());
        std::process::exit(1);
    }

    let cwds = proj_cache_cwds(&home);
    let mut workspaces: Vec<_> = std::fs::read_dir(&sessions_dir)
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    workspaces.sort_by_key(|e| e.file_name());

    let mut total_sessions = 0usize;
    let mut total_size = 0u64;
    println!("── DSH 会话 ──  {}", sessions_dir.display());

    for ws in &workspaces {
        let name = ws.file_name().to_string_lossy().into_owned();
        let decoded = decode_workspace(&name);
        let mut items: Vec<_> = std::fs::read_dir(ws.path())
            .unwrap()
            .flatten()
            .filter(|e| e.path().is_dir())
            .map(|e| {
                let sz = dir_size(&e.path());
                let mtime = e.metadata().and_then(|m| m.modified()).ok();
                (
                    e.file_name().to_string_lossy().into_owned(),
                    sz,
                    mtime,
                )
            })
            .collect();
        items.sort();

        println!("\n── {} ──  {}", decoded, name);
        for (sid, sz, mtime) in &items {
            total_sessions += 1;
            total_size += sz;
            let cwd = cwds
                .get(sid)
                .map(|c| format!("  cwd: {c}"))
                .unwrap_or_default();
            let t = mtime.map(rel_time).unwrap_or_else(|| "-".into());
            println!("  {:<46} {:>8}  {}{}", sid, human_size(*sz), t, cwd);
        }
    }

    println!(
        "\n── 总览 ──  共 {} 个会话, 合计 {}",
        total_sessions,
        human_size(total_size)
    );
}

// ─── DSH 配置 ───

fn dsh_config() {
    let home = dsh_home();
    println!("── DSH 配置 ──  {}", home.display());

    let settings = home.join("settings.yaml");
    if settings.is_file() {
        println!("\n{}:", settings.display());
        if let Ok(raw) = std::fs::read_to_string(&settings) {
            for line in raw.lines() {
                println!("  {line}");
            }
        }
    } else {
        println!("\n  (无 settings.yaml)");
    }

    println!("\n── Profiles ──");
    let profiles = home.join("profiles");
    if let Ok(rd) = std::fs::read_dir(&profiles) {
        let mut dirs: Vec<_> = rd
            .flatten()
            .filter(|e| e.path().is_dir())
            .filter(|e| e.file_name() != "node_modules")
            .collect();
        dirs.sort_by_key(|e| e.file_name());
        if dirs.is_empty() {
            println!("  （无）");
        }
        for d in dirs {
            let name = d.file_name().to_string_lossy().into_owned();
            let bundles = profile_bundles(&d.path().join("package.json"));
            println!("  {name:<10} {}", bundles.join(", "));
        }
    }
}
