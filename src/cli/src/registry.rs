use crate::session::{EXIT_OK, EXIT_SOURCE};
use crate::util::home_dir;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const EXIT_DIFF: i32 = 1;

/// 已知 Agent 的本地配置目录；未列出的 agent 判为「无法比较」
fn agent_local_dir(name: &str) -> Option<PathBuf> {
    let home = home_dir()?;
    match name {
        "zed" => Some(home.join(".config/zed")),
        "opencode" => Some(home.join(".config/opencode")),
        "hermes" => Some(home.join(".hermes")),
        _ => None,
    }
}

fn skip_file(name: &str) -> bool {
    matches!(name, "README.md" | "LICENSE" | ".gitkeep")
}

/// 配置文件扩展名族：`.json` / `.jsonc` 视为一族，`.yaml` / `.yml` 一族
fn family(ext: &str) -> &str {
    match ext {
        "json" | "jsonc" => "json",
        "yaml" | "yml" => "yaml",
        other => other,
    }
}

fn is_config_ext(ext: &str) -> bool {
    matches!(ext, "json" | "jsonc" | "yaml" | "yml" | "toml" | "ini")
}

fn ext_of(name: &str) -> &str {
    name.rsplit_once('.').map(|(_, e)| e).unwrap_or("")
}

fn stem_of(name: &str) -> &str {
    name.rsplit_once('.').map(|(s, _)| s).unwrap_or(name)
}

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
                flatten_json(v, &format!("{prefix}[{i}]"), out);
            }
        }
        other => {
            out.insert(prefix.to_string(), other.to_string());
        }
    }
}

/// 去 JSONC 的行注释与尾逗号（字符串内的不动）
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
                    Some(c) if c.is_whitespace() => continue,
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

fn parse_json(path: &Path) -> Result<BTreeMap<String, String>, String> {
    let raw =
        std::fs::read_to_string(path).map_err(|e| format!("无法读取 {}: {e}", path.display()))?;
    let value: serde_json::Value = serde_json::from_str(&strip_jsonc(&raw))
        .map_err(|e| format!("JSON 解析失败 {}: {e}", path.display()))?;
    let mut flat = BTreeMap::new();
    flatten_json(&value, "", &mut flat);
    Ok(flat)
}

struct ConfigDiff {
    ok: usize,
    diff: usize,
    missing: usize,
    skipped: bool,
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
    let Ok(local) = parse_json(local_path) else {
        return ConfigDiff {
            ok: 0,
            diff: 0,
            missing: reg.len(),
            skipped: false,
        };
    };

    let (mut ok, mut diff, mut missing) = (0, 0, 0);
    for (key, reg_val) in &reg {
        match local.get(key) {
            Some(v) if v == reg_val => ok += 1,
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

pub fn status(registry: &Path, verbose: bool) -> i32 {
    if !registry.is_dir() {
        eprintln!("错误: 注册中心路径不存在: {}", registry.display());
        return EXIT_SOURCE;
    }

    let mut agents: Vec<_> = std::fs::read_dir(registry)
        .into_iter()
        .flatten()
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .collect();
    agents.sort_by_key(|e| e.file_name());

    if agents.is_empty() {
        eprintln!("错误: 注册中心为空: {}", registry.display());
        return EXIT_SOURCE;
    }

    let (mut ok_count, mut issue_count, mut comparable) = (0usize, 0usize, 0usize);
    let mut uncomparable: Vec<String> = Vec::new();

    for entry in &agents {
        let name = entry.file_name().to_string_lossy().into_owned();
        println!("\n── {name} ──");

        let Some(local_dir) = agent_local_dir(&name) else {
            println!("  无法比较：本地路径未定义（agent_local_dir 未收录）");
            uncomparable.push(name.clone());
            continue;
        };
        if !local_dir.is_dir() {
            println!("  本地目录不存在: {}", local_dir.display());
        }
        comparable += 1;

        let mut files: Vec<_> = std::fs::read_dir(entry.path())
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            .collect();
        files.sort_by_key(|e| e.file_name());

        // 本地已用掉的配置文件名（用于算「本地自定义」）
        let mut used_local: BTreeSet<String> = BTreeSet::new();
        let mut local_entries: Vec<(String, String, String)> = Vec::new(); // name, stem, ext
        if local_dir.is_dir()
            && let Ok(rd) = std::fs::read_dir(&local_dir)
        {
            for e in rd
                .flatten()
                .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
            {
                let n = e.file_name().to_string_lossy().into_owned();
                let ext = ext_of(&n).to_string();
                if !is_config_ext(&ext) {
                    continue;
                }
                local_entries.push((n.clone(), stem_of(&n).to_string(), ext));
            }
        }

        for file in &files {
            let fname = file.file_name().to_string_lossy().into_owned();
            if skip_file(&fname) {
                println!("  · {fname:<30}   (文档，跳过)");
                continue;
            }

            // 文件名对齐：同名 → 同 stem 同族
            let ext = ext_of(&fname);
            let stem = stem_of(&fname);
            let matched = if local_dir.join(&fname).is_file() {
                Some(local_dir.join(&fname))
            } else {
                local_entries
                    .iter()
                    .find(|(_, s, e)| s == stem && family(e) == family(ext))
                    .map(|(n, _, _)| local_dir.join(n))
            };

            let Some(local_path) = matched else {
                println!("  ✗ {fname:<30}   缺失");
                issue_count += 1;
                continue;
            };
            used_local.insert(
                local_path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            );

            if family(ext) == "json" {
                let d = compare_config(&file.path(), &local_path);
                if d.skipped {
                    continue;
                }
                ok_count += d.ok;
                issue_count += d.diff + d.missing;
                if d.diff + d.missing == 0 {
                    println!("  ✓ {fname:<30}   {} 项配置一致", d.ok);
                } else {
                    println!(
                        "  ⚠ {fname:<30}   {} 一致, {} 差异, {} 缺失",
                        d.ok, d.diff, d.missing
                    );
                }
            } else {
                println!("  ✓ {fname:<30}   存在");
                ok_count += 1;
            }
        }

        // 本地自定义：注册中心没登记的配置文件（默认汇总一行）
        let extras: Vec<&str> = local_entries
            .iter()
            .filter(|(n, _, _)| !used_local.contains(n))
            .filter(|(n, _, _)| {
                let stem = stem_of(n);
                !files
                    .iter()
                    .any(|f| stem_of(&f.file_name().to_string_lossy()) == stem)
            })
            .map(|(n, _, _)| n.as_str())
            .collect();
        if !extras.is_empty() {
            if verbose {
                for n in &extras {
                    println!("  + {n:<30}   本地自定义");
                }
            } else {
                println!(
                    "  + {:<30}   本地自定义（--verbose 展开）",
                    format!("{} 个文件", extras.len())
                );
            }
        }
    }

    println!("\n── 总览 ──");
    println!(
        "  Agent:     {} 个已注册（可比 {}，无法比较 {}）",
        agents.len(),
        comparable,
        uncomparable.len()
    );
    println!("  一致:      {ok_count} 项配置");
    println!("  不一致:    {issue_count} 项");
    if !uncomparable.is_empty() {
        println!("  无法比较:  {}", uncomparable.join(", "));
    }

    if issue_count == 0 && uncomparable.is_empty() {
        println!("\n  状态: ✓ 注册中心与本地完全一致");
        EXIT_OK
    } else if issue_count == 0 {
        println!(
            "\n  状态: ⚠ {} 个 agent 无法比较，其余一致",
            uncomparable.len()
        );
        EXIT_DIFF
    } else {
        println!("\n  状态: ⚠ {issue_count} 项不一致，需要同步");
        EXIT_DIFF
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strip_jsonc_drops_comments_and_trailing_commas() {
        let raw = "{\n  // 注释\n  \"a\": 1,\n  \"b\": [1, 2,],\n}";
        let parsed: serde_json::Value = serde_json::from_str(&strip_jsonc(raw)).unwrap();
        assert_eq!(parsed["a"], 1);
        assert_eq!(parsed["b"][1], 2);
    }

    #[test]
    fn file_name_family_pairs() {
        assert_eq!(family(ext_of("opencode.jsonc")), "json");
        assert_eq!(family(ext_of("opencode.json")), "json");
        assert_eq!(family(ext_of("config.yml")), "yaml");
        assert_eq!(stem_of("opencode.jsonc"), "opencode");
    }

    #[test]
    fn config_ext_whitelist() {
        assert!(is_config_ext("jsonc"));
        assert!(is_config_ext("toml"));
        assert!(!is_config_ext("db"));
        assert!(!is_config_ext("md"));
    }
}
