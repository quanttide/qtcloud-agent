use crate::session::{EXIT_NONE, EXIT_OK, EXIT_SOURCE};
use regex::Regex;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// 量潮专用清单：新增域名/桶名/人名改这里
const BUCKETS: &str = "qtclass-lite|qtclass-site|qtclass-studio|qtcloud-org-lite|qtcloud-org-studio|qtconsult-private|qtconsult-studio|qtgame-site|qtstore-site|qtstrategy-site|quanttide-terraform-state";

/// (类别, 正则, 需要排除的精确命中)——Rust 的 regex 不支持前后瞻，占位符靠精确排除
fn rules() -> Vec<(&'static str, Regex, Option<&'static str>)> {
    let mut out: Vec<(&'static str, Regex, Option<&'static str>)> = Vec::new();
    {
        let mut add = |cat: &'static str, pat: &str| {
            out.push((cat, Regex::new(pat).expect("内置正则应合法"), None));
        };
        add("secrets", r"sk-[A-Za-z0-9_\-]{8,}");
        add("secrets", r"ghp_[A-Za-z0-9]{16,}");
        add("secrets", r"github_pat_[A-Za-z0-9_]{20,}");
        add("secrets", r"AKIA[0-9A-Z]{12,}");
        add("secrets", r"LTAI[A-Za-z0-9]{12,}");
        add("secrets", r"AKID[A-Za-z0-9]{12,}");
        add("secrets", r"xox[baprs]-[A-Za-z0-9\-]{10,}");
        add(
            "secrets",
            r"open\.feishu\.cn/open-apis/bot/v2/hook/[A-Za-z0-9\-]+",
        );
        add("secrets", r"-----BEGIN [A-Z ]*PRIVATE KEY-----");
        add("secrets", r"Bearer\s+[A-Za-z0-9._\-]{12,}");
        add("emails", r"[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}");
        add("names", r"Guo Zhang");
        add("names", r"zhangguo");
        add("hosts", r"(?:[A-Za-z0-9\-]+\.)*quanttide\.(?:com|top)");
        add(
            "hosts",
            r"(?:[A-Za-z0-9\-]+\.)*oss-cn-[a-z0-9\-]+\.aliyuncs\.com",
        );
        add("hosts", r"(?:[A-Za-z0-9\-]+\.)*[a-z0-9\-]+\.amazonaws\.com");
        add("buckets", &format!(r"(?:{BUCKETS})"));
        add("buckets", r"oss://[a-z0-9][a-z0-9\-]*");
    }
    // 审计自己的清洗产物时，`oss://bucket` 是本工具写下的占位符，不算命中
    if let Some(last) = out.last_mut() {
        last.2 = Some("oss://bucket");
    }
    out.push((
        "paths",
        Regex::new(r"/home/iguo").expect("内置正则应合法"),
        None,
    ));
    out
}

fn is_scan_ext(name: &str) -> bool {
    matches!(
        name.rsplit_once('.').map(|(_, e)| e).unwrap_or(""),
        "json" | "jsonl" | "md" | "txt" | "log" | "yaml" | "yml"
    )
}

fn collect(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        out.push(path.to_path_buf());
        return;
    }
    let Ok(rd) = std::fs::read_dir(path) else {
        return;
    };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect(&p, out);
        } else if p
            .file_name()
            .map(|n| is_scan_ext(&n.to_string_lossy()))
            .unwrap_or(false)
        {
            out.push(p);
        }
    }
}

pub fn run(path: &Path, as_json: bool) -> i32 {
    if !path.exists() {
        eprintln!("错误: 路径不存在: {}", path.display());
        return EXIT_SOURCE;
    }

    let mut files = Vec::new();
    collect(path, &mut files);
    files.sort();

    let rules = rules();
    let mut totals: BTreeMap<&str, usize> = BTreeMap::new();
    let mut per_file: Vec<(String, BTreeMap<&str, usize>)> = Vec::new();

    for f in &files {
        let Ok(bytes) = std::fs::read(f) else {
            continue;
        };
        let text = String::from_utf8_lossy(&bytes);
        let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
        for (cat, re, exclude) in &rules {
            let n = re
                .find_iter(&text)
                .filter(|m| Some(m.as_str()) != *exclude)
                .count();
            if n > 0 {
                *counts.entry(cat).or_insert(0) += n;
                *totals.entry(cat).or_insert(0) += n;
            }
        }
        if !counts.is_empty() {
            per_file.push((f.display().to_string(), counts));
        }
    }

    let total: usize = totals.values().sum();

    if as_json {
        let hits: Vec<serde_json::Value> = per_file
            .iter()
            .map(|(f, c)| {
                let mut m = serde_json::Map::new();
                for (k, v) in c {
                    m.insert((*k).to_string(), serde_json::json!(v));
                }
                serde_json::json!({ "file": f, "counts": m })
            })
            .collect();
        let out = serde_json::json!({
            "path": path.display().to_string(),
            "files": files.len(),
            "total": total,
            "totals": totals,
            "hits": hits,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        println!("── 审计 ──  {}（{} 个文件）", path.display(), files.len());
        for (f, c) in &per_file {
            let detail: Vec<String> = c.iter().map(|(k, v)| format!("{k} {v}")).collect();
            println!("  {}  {}", f, detail.join(", "));
        }
        if total == 0 {
            println!("  0 命中");
        } else {
            println!("\n合计 {total} 处，涉及 {} 个文件", per_file.len());
        }
    }

    if total == 0 { EXIT_OK } else { EXIT_NONE }
}
