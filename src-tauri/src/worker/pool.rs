use crate::worker::ipc::{WorkerRequest, WorkerResponse};
use std::io::Read;
use anyhow::{Context, Result};
use futures_util::stream::BoxStream;
use std::collections::HashMap;
use std::io::{BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, ChildStdout, Command, Stdio};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

/// 移除 Windows 冗长路径前缀 `\\?\`，避免传递给子进程时解析失败
#[cfg(windows)]
pub fn normalize_path(p: &Path) -> PathBuf {
    let s = p.to_string_lossy();
    if s.starts_with(r"\\?\") {
        PathBuf::from(&s[4..])
    } else {
        p.to_path_buf()
    }
}

#[cfg(not(windows))]
pub fn normalize_path(p: &Path) -> PathBuf {
    p.to_path_buf()
}

/// 解析 workers 目录（与 lib.rs 中 resolve_workers_dir 相同逻辑，供命令模块复用）
pub fn resolve_workers_dir() -> PathBuf {
    let mut candidates: Vec<PathBuf> = Vec::new();

    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            candidates.push(exe_dir.join("workers"));
            let mut up = exe_dir.to_path_buf();
            for _ in 0..3 {
                if let Some(parent) = up.parent() {
                    up = parent.to_path_buf();
                } else {
                    break;
                }
            }
            candidates.push(up.join("workers"));
        }
    }

    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let manifest = PathBuf::from(manifest);
        if let Some(parent) = manifest.parent() {
            candidates.push(parent.join("workers"));
        }
    }

    candidates.push(PathBuf::from("workers"));

    for c in &candidates {
        let normalized = normalize_path(c);
        if normalized.join("asr_worker.py").exists() {
            return normalized;
        }
    }

    candidates
        .last()
        .map(|p| normalize_path(p))
        .unwrap_or_else(|| PathBuf::from("workers"))
}

/// Python Worker 类型
#[derive(Debug, Clone, Hash, Eq, PartialEq)]
pub enum WorkerType {
    Llamacpp,    // llama.cpp LLM 推理
    Asr,         // faster-whisper
    Tts,         // CosyVoice / pyttsx3
    Embedding,   // bge-m3
    Vad,         // silero-vad
    Emotion,     // Wav2Vec2 SER
}

impl WorkerType {
    pub fn as_str(&self) -> &'static str {
        match self {
            WorkerType::Llamacpp => "llm",
            WorkerType::Asr => "asr",
            WorkerType::Tts => "tts",
            WorkerType::Embedding => "embedding",
            WorkerType::Vad => "vad",
            WorkerType::Emotion => "emotion",
        }
    }
}

/// 单个 Worker 进程
struct WorkerProcess {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
    child: Option<Child>,
}

unsafe impl Send for WorkerProcess {}
unsafe impl Sync for WorkerProcess {}

/// Worker 进程池
pub struct WorkerPool {
    workers: Arc<Mutex<HashMap<WorkerType, WorkerProcess>>>,
    workers_dir: String,
    models_dir: Arc<Mutex<String>>,
}

impl WorkerPool {
    pub fn new(workers_dir: &str, models_dir: &str) -> Self {
        Self {
            workers: Arc::new(Mutex::new(HashMap::new())),
            workers_dir: workers_dir.to_string(),
            models_dir: Arc::new(Mutex::new(models_dir.to_string())),
        }
    }

    pub fn update_models_dir(&self, new_dir: &str) {
        let mut dir = self.models_dir.lock().unwrap();
        *dir = new_dir.to_string();
        drop(dir);
        self.shutdown();
    }

    /// 启动 Worker 进程（懒启动）
    fn spawn_worker(&self, wt: &WorkerType) -> Result<()> {
        let mut workers = self.workers.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        if workers.contains_key(wt) {
            return Ok(());
        }

        let python = find_python().context("Python not found")?;
        let script_name = format!("{}_worker.py", wt.as_str());
        let workers_path = normalize_path(std::path::Path::new(&self.workers_dir));
        let script = normalize_path(&workers_path.join(script_name));
        let models_dir_str = self.models_dir.lock().unwrap().clone();
        let models_path = normalize_path(std::path::Path::new(&models_dir_str));

        if !script.exists() {
            return Err(anyhow::anyhow!("worker script not found: {:?}", script));
        }

        let mut child = Command::new(&python)
            .env("PYTHONPATH", &workers_path)
            .env("PYTHONIOENCODING", "utf-8")
            .env("AI_MODELS_DIR", models_path.to_string_lossy().to_string())
            .arg(&script)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .current_dir(&workers_path)
            .spawn()
            .context(format!("spawn {:?}", script))?;

        let stdin = child.stdin.take().context("stdin not available")?;
        let stdout = BufReader::new(child.stdout.take().context("stdout not available")?);

        workers.insert(
            wt.clone(),
            WorkerProcess {
                stdin,
                stdout,
                child: Some(child),
            },
        );
        Ok(())
    }

    /// 发送请求并等待完整响应（非流式）
    pub fn call(&self, wt: WorkerType, req: WorkerRequest) -> Result<WorkerResponse> {
        self.spawn_worker(&wt)?;
        let mut workers = self.workers.lock().map_err(|e| anyhow::anyhow!("lock: {e}"))?;
        let proc = workers.get_mut(&wt).context("worker not found")?;

        let req_data = rmp_serde::to_vec_named(&req).context("msgpack serialize")?;
        let len_prefix = (req_data.len() as u32).to_be_bytes();
        proc.stdin.write_all(&len_prefix).context("write length")?;
        proc.stdin.write_all(&req_data).context("write data")?;
        proc.stdin.flush()?;

        // 读取响应
        let mut len_buf = [0u8; 4];
        proc.stdout.read_exact(&mut len_buf).context("read resp length")?;
        let resp_len = u32::from_be_bytes(len_buf) as usize;
        let mut resp_buf = vec![0u8; resp_len];
        proc.stdout.read_exact(&mut resp_buf).context("read resp data")?;

        let resp: WorkerResponse = rmp_serde::from_slice(&resp_buf).context("msgpack deserialize")?;
        Ok(resp)
    }

    /// 发送流式请求，返回异步流
    pub async fn call_stream(&self, wt: WorkerType, req: WorkerRequest) -> Result<BoxStream<'static, WorkerResponse>> {
        self.spawn_worker(&wt)?;
        let (tx, rx) = mpsc::unbounded_channel();

        let workers_arc = self.workers.clone();
        let wt_clone = wt.clone();

        tokio::task::spawn_blocking(move || {
            let mut workers = workers_arc.lock().unwrap();
            let proc = workers.get_mut(&wt_clone).unwrap();

            let req_data = rmp_serde::to_vec_named(&req).unwrap();
            let len_prefix = (req_data.len() as u32).to_be_bytes();
            proc.stdin.write_all(&len_prefix).unwrap();
            proc.stdin.write_all(&req_data).unwrap();
            proc.stdin.flush().unwrap();

            loop {
                let mut len_buf = [0u8; 4];
                if proc.stdout.read_exact(&mut len_buf).is_err() {
                    break;
                }
                let resp_len = u32::from_be_bytes(len_buf) as usize;
                let mut resp_buf = vec![0u8; resp_len];
                if proc.stdout.read_exact(&mut resp_buf).is_err() {
                    break;
                }
                let resp: WorkerResponse = rmp_serde::from_slice(&resp_buf).unwrap();
                let done = resp.done;
                let _ = tx.send(resp);
                if done {
                    break;
                }
            }
        });

        Ok(Box::pin(tokio_stream::wrappers::UnboundedReceiverStream::new(rx)))
    }

    /// 关闭所有 Worker
    pub fn shutdown(&self) {
        let mut workers = self.workers.lock().ok();
        if let Some(ref mut workers) = workers {
            for (_, proc) in workers.drain() {
                if let Some(mut child) = proc.child {
                    let _ = child.kill();
                }
            }
        }
    }
}

/// 查找系统中的 Python 解释器
fn find_python() -> Result<String> {
    for cmd in &["python3", "python"] {
        if Command::new(cmd).arg("--version").output().is_ok() {
            return Ok(cmd.to_string());
        }
    }
    Err(anyhow::anyhow!("No Python interpreter found"))
}
