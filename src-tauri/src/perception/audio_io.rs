use anyhow::{Context, Result};
use std::sync::{Arc, Mutex};

// ─── 设备枚举 ────────────────────────────────────────────

#[cfg(feature = "perception")]
pub fn list_devices() -> Result<Vec<String>> {
    use cpal::traits::{DeviceTrait, HostTrait};
    let host = cpal::default_host();
    let devices = host
        .devices()
        .map_err(|e| anyhow::anyhow!("cpal devices: {e}"))?;
    Ok(devices.filter_map(|d| d.name().ok()).collect())
}

#[cfg(not(feature = "perception"))]
pub fn list_devices() -> Result<Vec<String>> {
    Ok(vec!["默认设备 (feature disabled)".to_string()])
}

// ─── 录音 ────────────────────────────────────────────────

#[cfg(feature = "perception")]
pub struct AudioRecorder {
    stream: Option<cpal::Stream>,
    buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    channels: u16,
}

#[cfg(feature = "perception")]
impl AudioRecorder {
    /// 创建录音器（不启动）
    pub fn new() -> Result<Self> {
        Ok(Self {
            stream: None,
            buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate: 16000,
            channels: 1,
        })
    }

    /// 启动录音（从默认输入设备）
    pub fn start(&mut self) -> Result<()> {
        if self.stream.is_some() {
            anyhow::bail!("已在录音中");
        }

        use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
        let host = cpal::default_host();
        let device = host
            .default_input_device()
            .context("未找到麦克风设备")?;

        let config = device
            .default_input_config()
            .map_err(|e| anyhow::anyhow!("获取输入配置失败: {e}"))?;

        self.sample_rate = config.sample_rate().0;
        self.channels = config.channels();

        let buf = self.buffer.clone();
        let stream = match config.sample_format() {
            cpal::SampleFormat::F32 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[f32], _| {
                        if let Ok(mut b) = buf.lock() {
                            b.extend_from_slice(data);
                        }
                    },
                    |err| log::error!("录音错误: {err}"),
                    None,
                )
            }
            cpal::SampleFormat::I16 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[i16], _| {
                        if let Ok(mut b) = buf.lock() {
                            for s in data {
                                b.push(*s as f32 / 32768.0);
                            }
                        }
                    },
                    |err| log::error!("录音错误: {err}"),
                    None,
                )
            }
            cpal::SampleFormat::U16 => {
                device.build_input_stream(
                    &config.into(),
                    move |data: &[u16], _| {
                        if let Ok(mut b) = buf.lock() {
                            for s in data {
                                b.push((*s as f32 - 32768.0) / 32768.0);
                            }
                        }
                    },
                    |err| log::error!("录音错误: {err}"),
                    None,
                )
            }
            _ => anyhow::bail!("不支持的采样格式: {:?}", config.sample_format()),
        }
        .context("创建录音流失败")?;

        stream.play().context("启动录音流失败")?;
        self.stream = Some(stream);
        Ok(())
    }

    /// 停止录音，返回 WAV 字节
    pub fn stop(&mut self) -> Result<Vec<u8>> {
        let stream = self.stream.take().context("未在录音")?;
        drop(stream); // 停止流

        let samples = {
            let mut b = self.buffer.lock().map_err(|e| anyhow::anyhow!("{e}"))?;
            std::mem::take(&mut *b)
        };

        encode_wav(&samples, self.sample_rate, self.channels)
    }

    /// 获取当前缓冲区长度（用于判断是否有数据）
    pub fn buffer_len(&self) -> usize {
        self.buffer.lock().map(|b| b.len()).unwrap_or(0)
    }
}

#[cfg(not(feature = "perception"))]
pub struct AudioRecorder;

#[cfg(not(feature = "perception"))]
impl AudioRecorder {
    pub fn new() -> Result<Self> {
        anyhow::bail!("感知模块未启用（编译时未开启 perception feature）")
    }
    pub fn start(&mut self) -> Result<()> {
        anyhow::bail!("感知模块未启用")
    }
    pub fn stop(&mut self) -> Result<Vec<u8>> {
        anyhow::bail!("感知模块未启用")
    }
    pub fn buffer_len(&self) -> usize {
        0
    }
}

// ─── WAV 编码 ────────────────────────────────────────────

/// 将 f32 采样编码为 WAV 格式（PCM 16-bit）
#[cfg(feature = "perception")]
fn encode_wav(samples: &[f32], sample_rate: u32, channels: u16) -> Result<Vec<u8>> {
    if samples.is_empty() {
        anyhow::bail!("录音缓冲区为空（可能麦克风权限未授予或设备静音）");
    }

    let bits_per_sample: u16 = 16;
    let byte_rate = sample_rate * channels as u32 * (bits_per_sample / 8) as u32;
    let block_align = channels * (bits_per_sample / 8);
    let data_size = samples.len() * 2; // 16-bit = 2 bytes per sample
    let file_size = 36 + data_size;

    let mut wav = Vec::with_capacity(44 + data_size);

    // RIFF header
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&(file_size as u32).to_le_bytes());
    wav.extend_from_slice(b"WAVE");

    // fmt chunk
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());       // chunk size
    wav.extend_from_slice(&1u16.to_le_bytes());         // PCM
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&bits_per_sample.to_le_bytes());

    // data chunk
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&(data_size as u32).to_le_bytes());

    // samples: f32 [-1.0, 1.0] → i16 [-32768, 32767]
    for s in samples {
        let clamped = s.clamp(-1.0, 1.0);
        let sample = (clamped * 32767.0) as i16;
        wav.extend_from_slice(&sample.to_le_bytes());
    }

    Ok(wav)
}

// ─── 播放 ────────────────────────────────────────────────

#[cfg(feature = "perception")]
pub fn play_wav(wav_bytes: &[u8]) -> Result<()> {
    use rodio::Source;
    use std::io::Cursor;
    let cursor = Cursor::new(wav_bytes.to_vec());
    let source = rodio::Decoder::new(cursor)
        .context("WAV 解码失败（音频数据可能损坏）")?;
    let (_stream, handle) = rodio::OutputStream::try_default()
        .context("无法打开音频输出设备")?;
    handle
        .play_raw(source.convert_samples())
        .context("播放失败")?;
    // 让音频播完（简单估算：字节数 / 字节率）
    let duration_ms = (wav_bytes.len() as u64 * 1000 / 32000).max(500);
    std::thread::sleep(std::time::Duration::from_millis(duration_ms));
    Ok(())
}

#[cfg(not(feature = "perception"))]
pub fn play_wav(_wav_bytes: &[u8]) -> Result<()> {
    anyhow::bail!("感知模块未启用（编译时未开启 perception feature）")
}
