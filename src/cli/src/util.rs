use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

/// 归属判定：候选目录是否落在目标目录内（物理路径优先，缺失时退回原样；按路径段比对）
pub fn inside(candidate: &str, target: &Path) -> bool {
    let cand = Path::new(candidate);
    let cand_real = cand.canonicalize().unwrap_or_else(|_| cand.to_path_buf());
    let target_real = target
        .canonicalize()
        .unwrap_or_else(|_| target.to_path_buf());
    cand_real.starts_with(&target_real)
}

pub fn dir_size(path: &Path) -> u64 {
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

pub fn human_size(bytes: u64) -> String {
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

pub fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// 相对时间（输入 unix 秒）
pub fn rel_time(ts: u64) -> String {
    let now = now_unix();
    if ts > now {
        return "-".into();
    }
    let s = now - ts;
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

/// UTC 时间（yyyy-mm-dd hh:mm），不引第三方日期库
pub fn utc_time(ts: u64) -> String {
    let days = (ts / 86400) as i64;
    let secs_of_day = ts % 86400;
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        secs_of_day / 3600,
        (secs_of_day % 3600) / 60
    )
}

/// Howard Hinnant 的 civil_from_days
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inside_requires_same_or_child() {
        let target = Path::new("/home/user/repo");
        assert!(inside("/home/user/repo", target));
        assert!(inside("/home/user/repo/sub/dir", target));
        assert!(!inside("/home/user/repo-other", target));
        assert!(!inside("/home/user", target));
    }

    #[test]
    fn utc_time_formats_epoch() {
        assert_eq!(utc_time(0), "1970-01-01 00:00");
        assert_eq!(utc_time(1_700_000_000), "2023-11-14 22:13");
    }

    #[test]
    fn human_size_scales() {
        assert_eq!(human_size(512), "512 B");
        assert_eq!(human_size(1024), "1.0 KB");
        assert_eq!(human_size(1024 * 1024), "1.0 MB");
    }
}
