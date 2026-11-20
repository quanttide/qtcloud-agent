use crate::util::{dir_size, home_dir, human_size, inside, rel_time, utc_time};
use rusqlite::{Connection, OpenFlags};
use std::collections::BTreeMap;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

pub const EXIT_OK: i32 = 0;
pub const EXIT_NONE: i32 = 1;
pub const EXIT_SOURCE: i32 = 3;

/// 一条会话记录。四家存储形态不同，读出来统一成这个形状。
pub struct SessionRecord {
    pub agent: &'static str,
    pub id: String,
    pub title: Option<String>,
    pub cwd: Option<String>,
    pub bytes: Option<u64>,
    pub messages: Option<u64>,
    pub modified: Option<u64>,
    /// 存储位置：目录 / 文件 / 数据库路径
    pub origin: String,
}

pub fn read(agent: &str) -> Vec<SessionRecord> {
    match agent {
        "dsh" => read_dsh(),
        "pi" => read_pi(),
        "opencode" => read_opencode(),
        "hermes" => read_hermes(),
        _ => Vec::new(),
    }
}

// ─── dsh ───

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

fn read_dsh() -> Vec<SessionRecord> {
    let root = dsh_home().join("sessions");
    let cwds = dsh_proj_cache();
    let mut out = Vec::new();
    let Ok(workspaces) = std::fs::read_dir(&root) else {
        return out;
    };
    for ws in workspaces.flatten() {
        if !ws.path().is_dir() {
            continue;
        }
        let Ok(sessions) = std::fs::read_dir(ws.path()) else {
            continue;
        };
        for s in sessions.flatten() {
            if !s.path().is_dir() {
                continue;
            }
            let id = s.file_name().to_string_lossy().into_owned();
            out.push(SessionRecord {
                agent: "dsh",
                cwd: cwds.get(&id).cloned(),
                title: None,
                bytes: Some(dir_size(&s.path())),
                messages: None,
                modified: mtime_secs(&s.path()),
                origin: s.path().display().to_string(),
                id,
            });
        }
    }
    out
}

/// $DSH_HOME/storages/session_projcache.json → session id → cwd
fn dsh_proj_cache() -> BTreeMap<String, String> {
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

// ─── pi ───

fn pi_home() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".pi/agent/sessions")
}

fn read_pi() -> Vec<SessionRecord> {
    let root = pi_home();
    let mut out = Vec::new();
    let Ok(dirs) = std::fs::read_dir(&root) else {
        return out;
    };
    for dir in dirs.flatten() {
        if !dir.path().is_dir() {
            continue;
        }
        let Ok(files) = std::fs::read_dir(dir.path()) else {
            continue;
        };
        for f in files.flatten() {
            let path = f.path();
            if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
                continue;
            }
            let cwd = pi_cwd(&path);
            let id = path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_default();
            out.push(SessionRecord {
                agent: "pi",
                cwd,
                title: None,
                bytes: path.metadata().ok().map(|m| m.len()),
                messages: None,
                modified: mtime_secs(&path),
                origin: path.display().to_string(),
                id,
            });
        }
    }
    out
}

/// pi 会话首行是 `{"type":"session","cwd":…}`，只读第一行
fn pi_cwd(path: &Path) -> Option<String> {
    let file = std::fs::File::open(path).ok()?;
    let mut line = String::new();
    BufReader::new(file).read_line(&mut line).ok()?;
    let v: serde_json::Value = serde_json::from_str(&line).ok()?;
    v.get("cwd").and_then(|c| c.as_str()).map(String::from)
}

fn pi_message_count(path: &Path) -> Option<u64> {
    let file = std::fs::File::open(path).ok()?;
    let mut n = 0u64;
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if line.contains("\"type\":\"message\"") {
            n += 1;
        }
    }
    Some(n)
}

// ─── opencode ───

fn opencode_db() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".local/share/opencode/opencode.db")
}

fn open_ro(path: &Path) -> Option<Connection> {
    if !path.exists() {
        return None;
    }
    match Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(c) => Some(c),
        Err(e) => {
            eprintln!("跳过 {}：{e}", path.display());
            None
        }
    }
}

fn read_opencode() -> Vec<SessionRecord> {
    let db = opencode_db();
    let Some(conn) = open_ro(&db) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let sql = "SELECT id, directory, title, time_created, time_updated FROM session";
    let Ok(mut stmt) = conn.prepare(sql) else {
        eprintln!("跳过 opencode：session 表读取失败");
        return out;
    };
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<i64>>(3)?,
            r.get::<_, Option<i64>>(4)?,
        ))
    });
    let Ok(rows) = rows else {
        return out;
    };
    for row in rows.flatten() {
        let (id, directory, title, created, updated) = row;
        out.push(SessionRecord {
            agent: "opencode",
            id,
            title,
            cwd: directory,
            bytes: None,
            messages: None,
            // opencode 的时间是毫秒
            modified: updated.or(created).map(|ms| (ms / 1000) as u64),
            origin: db.display().to_string(),
        });
    }
    out
}

// ─── hermes ───

fn hermes_db() -> PathBuf {
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".hermes/state.db")
}

fn read_hermes() -> Vec<SessionRecord> {
    let db = hermes_db();
    let Some(conn) = open_ro(&db) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let sql = "SELECT id, cwd, title, message_count, started_at, ended_at, last_activity_at FROM sessions";
    let Ok(mut stmt) = conn.prepare(sql) else {
        eprintln!("跳过 hermes：sessions 表读取失败");
        return out;
    };
    let rows = stmt.query_map([], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, Option<String>>(1)?,
            r.get::<_, Option<String>>(2)?,
            r.get::<_, Option<i64>>(3)?,
            r.get::<_, Option<f64>>(4)?,
            r.get::<_, Option<f64>>(5)?,
            r.get::<_, Option<f64>>(6)?,
        ))
    });
    let Ok(rows) = rows else {
        return out;
    };
    for row in rows.flatten() {
        let (id, cwd, title, messages, started, ended, last) = row;
        out.push(SessionRecord {
            agent: "hermes",
            id,
            title,
            cwd,
            bytes: None,
            messages: messages.map(|m| m as u64),
            modified: last.or(ended).or(started).map(|s| s as u64),
            origin: db.display().to_string(),
        });
    }
    out
}

// ─── 列表与详情 ───

fn mtime_secs(path: &Path) -> Option<u64> {
    path.metadata()
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

pub fn list(agents: &[&str], cwd: Option<&Path>, all: bool, as_json: bool) -> i32 {
    let mut records: Vec<SessionRecord> = Vec::new();
    for agent in agents {
        records.extend(read(agent));
    }

    if !all {
        let Some(target) = cwd else {
            eprintln!("错误: 无法确定当前目录");
            return EXIT_SOURCE;
        };
        records.retain(|r| match r.cwd.as_deref() {
            Some(c) => inside(c, target),
            None => false,
        });
    }
    records.sort_by(|a, b| {
        b.modified
            .cmp(&a.modified)
            .then_with(|| a.agent.cmp(b.agent))
            .then_with(|| a.id.cmp(&b.id))
    });

    if as_json {
        let items: Vec<serde_json::Value> = records.iter().map(to_json).collect();
        let out = serde_json::json!({
            "target": match (all, cwd) {
                (true, _) => serde_json::Value::Null,
                (false, Some(t)) => serde_json::Value::String(t.display().to_string()),
                _ => serde_json::Value::Null,
            },
            "count": items.len(),
            "sessions": items,
        });
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
    } else {
        match (all, cwd) {
            (true, _) => println!("── 会话 ──  （全部）"),
            (false, Some(t)) => println!("── 会话 ──  {}", t.display()),
            _ => println!("── 会话 ──"),
        }
        for r in &records {
            println!(
                "  {:<8} {:<46} {:>9}  {:>9}  {}",
                r.agent,
                r.id,
                r.bytes.map(human_size).unwrap_or_else(|| "-".into()),
                r.modified.map(rel_time).unwrap_or_else(|| "-".into()),
                r.cwd.as_deref().unwrap_or("(无 cwd)"),
            );
        }
        println!("\n共 {} 个会话", records.len());
    }

    if records.is_empty() {
        if all {
            eprintln!("0 个会话：本机没有可读的会话存储");
        } else if let Some(t) = cwd {
            eprintln!(
                "0 个匹配：{} 下没有可证归属的会话（用 --cwd <path> 换目录，或 --all 不过滤）",
                t.display()
            );
        }
        return EXIT_NONE;
    }
    EXIT_OK
}

pub fn show(agents: &[&str], id: &str, as_json: bool) -> i32 {
    let mut hits: Vec<SessionRecord> = Vec::new();
    for agent in agents {
        for mut r in read(agent) {
            if r.id == id {
                if r.agent == "pi" && r.messages.is_none() {
                    r.messages = pi_message_count(Path::new(&r.origin));
                }
                hits.push(r);
            }
        }
    }

    if hits.is_empty() {
        eprintln!("没有这个会话: {id}");
        return EXIT_NONE;
    }

    if as_json {
        let items: Vec<serde_json::Value> = hits.iter().map(to_json).collect();
        let out = serde_json::json!({ "count": items.len(), "sessions": items });
        println!("{}", serde_json::to_string_pretty(&out).unwrap_or_default());
        return EXIT_OK;
    }

    for r in &hits {
        println!("── {} ──", r.id);
        println!("  agent:     {}", r.agent);
        if let Some(t) = &r.title {
            println!("  title:     {t}");
        }
        println!("  cwd:       {}", r.cwd.as_deref().unwrap_or("(无 cwd)"));
        println!(
            "  消息:      {}",
            r.messages
                .map(|m| m.to_string())
                .unwrap_or_else(|| "-".into())
        );
        println!(
            "  体积:      {}",
            r.bytes.map(human_size).unwrap_or_else(|| "-".into())
        );
        match r.modified {
            Some(t) => println!("  最后活动:  {}（{}）", utc_time(t), rel_time(t)),
            None => println!("  最后活动:  -"),
        }
        println!("  位置:      {}\n", r.origin);
    }
    EXIT_OK
}

fn to_json(r: &SessionRecord) -> serde_json::Value {
    serde_json::json!({
        "agent": r.agent,
        "id": r.id,
        "title": r.title,
        "cwd": r.cwd,
        "bytes": r.bytes,
        "messages": r.messages,
        "modified_unix": r.modified,
        "origin": r.origin,
    })
}
