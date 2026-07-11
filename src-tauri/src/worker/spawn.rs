use crate::worker::pool::WorkerType;
use std::path::Path;
use std::process::Command;

/// 检查 Worker 依赖是否就绪
pub fn check_worker(wt: &WorkerType, workers_dir: &str) -> bool {
    let script = format!("{}/{}_worker.py", workers_dir, wt.as_str());
    Path::new(&script).exists()
}

/// 检查 Python 和 pip 是否可用
pub fn check_python() -> bool {
    for cmd in &["python3", "python"] {
        if Command::new(cmd).arg("--version").output().is_ok() {
            return true;
        }
    }
    false
}
