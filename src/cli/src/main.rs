use clap::Parser;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

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
}

fn main() {
    let cli = Cli::parse();
    match cli {
        Cli::Status { registry } => status(&registry),
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

// ─── 状态展示 ───

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
