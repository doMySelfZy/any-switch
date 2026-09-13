//! 请求日志：定长环形缓冲，供 UI 轮询展示。
//!
//! 刻意只记录 method / path / status / 耗时：请求头与请求体可能含密钥材料，
//! 一律不入日志。`path` 进缓冲前已剥掉路径口令（见 server.rs 的构造点）。

use crate::domain::LocalProxyRequestLogEntry;
use std::collections::VecDeque;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

/// 上限：够排查问题，又不会随运行时间无界增长。
pub const MAX_ENTRIES: usize = 200;

#[derive(Default)]
pub struct RequestLog {
    entries: Mutex<VecDeque<LocalProxyRequestLogEntry>>,
    next_id: AtomicU64,
}

impl RequestLog {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn push(&self, mut entry: LocalProxyRequestLogEntry) {
        entry.id = self.next_id.fetch_add(1, Ordering::Relaxed) + 1;
        let Ok(mut entries) = self.entries.lock() else {
            return;
        };
        if entries.len() >= MAX_ENTRIES {
            entries.pop_front();
        }
        entries.push_back(entry);
    }

    /// 最新在前。
    pub fn list(&self, limit: usize) -> Vec<LocalProxyRequestLogEntry> {
        let Ok(entries) = self.entries.lock() else {
            return Vec::new();
        };
        entries.iter().rev().take(limit).cloned().collect()
    }

    pub fn clear(&self) {
        if let Ok(mut entries) = self.entries.lock() {
            entries.clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(path: &str) -> LocalProxyRequestLogEntry {
        LocalProxyRequestLogEntry {
            id: 0,
            at: 1,
            target: "claude_code".into(),
            method: "POST".into(),
            path: path.into(),
            status: 200,
            duration_ms: 5,
            error: None,
        }
    }

    #[test]
    fn newest_first_and_bounded() {
        let log = RequestLog::new();
        for i in 0..(MAX_ENTRIES + 25) {
            log.push(entry(&format!("/p{i}")));
        }
        let listed = log.list(MAX_ENTRIES + 100);
        assert_eq!(listed.len(), MAX_ENTRIES, "超过上限必须淘汰最旧条目");
        assert_eq!(listed[0].path, format!("/p{}", MAX_ENTRIES + 24));
        assert!(
            listed.iter().all(|e| e.path != "/p0"),
            "最旧的条目应已被淘汰"
        );
    }

    #[test]
    fn ids_are_unique_and_increasing() {
        let log = RequestLog::new();
        log.push(entry("/a"));
        log.push(entry("/b"));
        let listed = log.list(10);
        assert!(listed[0].id > listed[1].id);
        log.clear();
        assert!(log.list(10).is_empty());
    }
}
