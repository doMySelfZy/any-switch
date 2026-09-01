use crate::error::{AppError, AppResult};
use crate::paths::locks_dir;
use fs2::FileExt;
use once_cell::sync::Lazy;
use parking_lot::Mutex;
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::sync::Arc;

static PROCESS_LOCKS: Lazy<Mutex<HashMap<String, Arc<Mutex<()>>>>> =
    Lazy::new(|| Mutex::new(HashMap::new()));

/// Holds process-level + cross-process exclusive lock for a target.
pub struct HeldLock {
    _guard: parking_lot::MutexGuard<'static, ()>,
    _file: std::fs::File,
    _arc: Arc<Mutex<()>>,
}

fn process_mutex(target: &str) -> Arc<Mutex<()>> {
    let mut map = PROCESS_LOCKS.lock();
    map.entry(target.to_string())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

/// Try to acquire per-site lock shared by key switch, route rewrite, and apply.
pub fn try_lock_site(site_id: &str) -> AppResult<HeldLock> {
    try_lock_target(&format!("site:{site_id}"))
}

/// Try to acquire per-target process + cross-process lock.
pub fn try_lock_target(target: &str) -> AppResult<HeldLock> {
    let arc = process_mutex(target);
    let guard = arc
        .try_lock()
        .ok_or_else(|| AppError::new("lock_busy", "target is busy"))?;

    let dir = locks_dir()?;
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(format!("apply_{target}.lock"));
    let file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .open(&path)?;
    file.try_lock_exclusive()
        .map_err(|_| AppError::new("lock_busy", "target is locked by another process"))?;

    // Keep Arc alive for guard lifetime.
    let guard_static: parking_lot::MutexGuard<'static, ()> = unsafe { std::mem::transmute(guard) };

    Ok(HeldLock {
        _guard: guard_static,
        _file: file,
        _arc: arc,
    })
}
