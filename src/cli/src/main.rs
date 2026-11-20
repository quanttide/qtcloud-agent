use clap::{Parser, Subcommand, ValueEnum};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// 退出码：0 有结果 / 一致，1 无匹配 / 有差异，2 用法错（clap），3 数据源读不到
const EXIT_OK: i32 = 0;
const EXIT_DIFF: i32 = 1;
const EXIT_SOURCE: i32 = 3;

/// 注册中心默认路径（可用 --path 或 QTCLOUD_AGENT_REGISTRY 覆盖）
const DEFAULT_REGISTRY: &str = "data/profile/default/agents";

/// 量潮智能体云 CLI：会话取用与注册中心对照
#[derive(Parser)]
#[command(name = "qtcloud-agent", version, about = "会话取用与注册中心对照")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// 注册中心：与本地 agent 配置对照
    #[command(subcommand)]
    Registry(RegistryCommand),
    /// 会话：找出来、看清楚、按归属筛、脱敏交出去
    #[command(subcommand)]
    Session(SessionCommand),
}

#[derive(Subcommand)]
enum RegistryCommand {
    /// 对照注册中心与本地 agent 配置
    Status {
        /// 注册中心路径（默认取 QTCLOUD_AGENT_REGISTRY，回退 data/profile/default/agents）
        #[arg(long)]
        path: Option<PathBuf>,
    },
}

#[derive(Subcommand)]
enum SessionCommand {
    /// 列出会话（按记录时的 cwd 判定归属）
    List {
        /// 只列某个 harness 的会话
        #[arg(long, value_enum)]
        agent: Option<Harness>,
        /// 按目录归属筛选（默认当前目录）
        #[arg(long, conflicts_with = "all")]
        cwd: Option<PathBuf>,
        /// 不按目录筛选
        #[arg(long)]
        all: bool,
        /// 输出 JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
enum Harness {
    /// DeepSeek Harness（$DSH_HOME，默认 ~/.dsh）
    Dsh,
}

fn main() {
    let code = match Cli::parse().command {
        Command::Registry(cmd) => match cmd {
            RegistryCommand::Status { path } => registry_status(&registry_path(path)),
        },
        Command::Session(cmd) => match cmd {
            SessionCommand::List {
                agent,
                cwd,
                all,
                json,
            } => session_list(agent, cwd, all, json),
        },
    };
    std::process::exit(code);
}

fn registry_path(cli: Option<PathBuf>) -> PathBuf {
    cli.or_else(|| std::env::var_os("QTCLOUD_AGENT_REGISTRY").map(PathBuf::from))
        .unwrap_or_else(|| PathBuf::from(DEFAULT_REGISTRY))
}

// ─── 会话记录 ───

struct SessionRecord {
    agent: &'static str,
    id: String,
    workspace: String,
    cwd: Option<String>,
    size: u64,
    modified: Option<u64>,
}

fn session_list(agent: Option<Harness>, cwd: Option<PathBuf>, all: bool, as_json: bool) -> i32 {
    let target =
        cwd.unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let harnesses: Vec<Harness> = match agent {
        Some(h) => vec![h],
        None => vec![Harness::Dsh],
    };

    let mut records: Vec<SessionRecord> = Vec::new();
    for harness in harnesses {
        let mut found = match harness {
            Harness::Dsh => read_dsh_sessions(),
        };
        records.append(&mut found);
    }

    if !all {
        records.retain(|r| match r.cwd.as_deref() {
            Some(c) => inside(c, &target),
            None => false,
        });
    }
    records.sort_by(|a, b| b.modified.cmp(&a.modified).then_with(|| a.id.cmp(&b.id)));

    if as_json {
        let items: Vec<serde_json::Value> = records
            .iter()
            .map(|r| {
                serde_json::json!({
                    "agent": r.agent,
                    "id": r.id,
                    "workspace": r.workspace,
                    "cwd": r.cwd,
                    "bytes": r.size,
                    "modified_unix": r.modified,
                })
            })
            .collect();
        let out = serde_json::json!({
            "target": if all { serde_json::Value::Null } else { serde_json::Value::String(target.display().to_string()) },
            "count": items.len(),
            "sessions": items,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        if all {
            println!("── 会话 ──  （全部）");
        } else {
            println!("── 会话 ──  {}", target.display());
        }
        for r in &records {
            println!(
                "  {:<5} {:<46} {:>8}  {:>10}  {}",
                r.agent,
                r.id,
                human_size(r.size),
                r.modified
                    .map(|s| rel_time(UNIX_EPOCH + Duration::from_secs(s)))
                    .unwrap_or_else(|| "-".into()),
                r.cwd.as_deref().unwrap_or("(无 cwd)"),
            );
        }
        println!("\n共 {} 个会话", records.len());
    }

    if records.is_empty() {
        if all {
            eprintln!("0 个会话：本机没有可读的会话存储");
        } else {
            eprintln!(
                "0 个匹配：{} 下没有可证归属的会话（用 --cwd <path> 换目录，或 --all 不过滤）",
                target.display()
            );
        }
        return EXIT_DIFF;
    }
    EXIT_OK
}

/// 归属判定：候选目录是否落在目标目录内（物理路径优先，缺失时退回原样比较；按路径段比对）
fn inside(candidate: &str, target: &Path) -> bool {
    let cand = Path::new(candidate);
    let cand_real = cand.canonicalize().unwrap_or_else(|_| cand.to_path_buf());
    let target_real = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());
    cand_real.starts_with(&target_real)
}

// ─── DSH 会话读取 ───

/// DSH 数据目录（$DSH_HOME 或 ~/.dsh）
fn dsh_home() -> PathBuf {
    if let Some(h) = std::env::var_os("DSH_HOME")
        && !h.is_empty()
    {
        return PathBuf::from(h);
    }
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".dsh")
}

fn read_dsh_sessions() -> Vec<SessionRecord> {
    let root = dsh_home().join("sessions");
    let cwds = proj_cache_cwds();
    let mut out = Vec::new();
    let Ok(workspaces) = std::fs::read_dir(&root) else {
        return out;
    };
    for ws in workspaces.flatten() {
        if !ws.path().is_dir() {
            continue;
        }
        let workspace = ws.file_name().to_string_lossy().into_owned();
        let decoded = decode_workspace(&workspace);
        let Ok(sessions) = std::fs::read_dir(ws.path()) else {
            continue;
        };
        for s in sessions.flatten() {
            if !s.path().is_dir() {
                continue;
            }
            let id = s.file_name().to_string_lossy().into_owned();
            let modified = s
                .metadata()
                .and_then(|m| m.modified())
                .ok()
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs());
            out.push(SessionRecord {
                agent: "dsh",
                cwd: cwds.get(&id).cloned(),
                workspace: decoded.clone(),
                size: dir_size(&s.path()),
                id,
                modified,
            });
        }
    }
    out
}

/// 解码工作区目录名（`--home-iguo-repos-qt--` → `/home/iguo/repos/qt`）
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
    // 目录名把 `/` 换成了 `-`，解码必然失真（`quanttide-human` 会变成 `quanttide/human`）；
    // cwd 以 session_projcache.json 为准，这里只作无 cwd 时的兜底显示。
    format!("/{}", inner.replace('-', "/"))
}

/// 从 $DSH_HOME/storages/session_projcache.json 读取 session id → 工作目录 映射
fn proj_cache_cwds() -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let p = dsh_home().join("storages/session_projcache.json");
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

// ─── 注册中心对照 ───

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// 已知 Agent 本地配置目录
fn agent_local_dir(name: &str) -> Option<PathBuf> {
    let home = home_dir()?;
    match name {
        "zed" => Some(home.join(".config/zed")),
        "opencode" => Some(home.join(".config/opencode")),
        "hermes" => Some(home.join(".hermes")),
        _ => None,
    }
}

/// 是否需要跳过（非配置类文件）
fn skip_file(name: &str) -> bool {
    matches!(name, "README.md" | "LICENSE" | ".gitkeep")
}

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
            chars.next();
            while let Some(&c) = chars.peek() {
                if c == '\n' {
                    break;
                }
                chars.next();
            }
            out.push('\n');
        } else if !in_string && c == ',' {
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
    ok: usize,
    diff: usize,
    missing: usize,
    skipped: bool,
}

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
            Some(_) => diff += 1,
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

fn registry_status(registry: &Path) -> i32 {
    if !registry.is_dir() {
        eprintln!("错误: 注册中心路径不存在: {}", registry.display());
        return EXIT_SOURCE;
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

        let local_ok = local_dir.as_ref().map(|d| d.is_dir()).unwrap_or(false);
        if !local_ok {
            match local_dir {
                Some(ref d) => println!("  本地目录不存在: {}", d.display()),
                None => println!("  未知 Agent，本地路径未定义"),
            }
        }

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
                    println!("  ✓ {:<30}   存在", fname_str);
                    total_ok += 1;
                }
            }
        }

        let Some(base) = local_dir.as_ref().filter(|d| d.is_dir()) else {
            continue;
        };
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

    println!("\n── 总览 ──");
    println!("  Agent:   {} 个已注册", agents.len());
    println!("  一致:    {} 项配置", total_ok);

    if agents.is_empty() {
        eprintln!("\n  状态: ✗ 注册中心为空: {}", registry.display());
        return EXIT_SOURCE;
    }

    if total_issues == 0 {
        println!("\n  状态: ✓ 注册中心与本地完全一致");
        EXIT_OK
    } else {
        println!("\n  状态: ⚠ {total_issues} 项不一致，需要同步");
        EXIT_DIFF
    }
}

// ─── 通用 ───

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
