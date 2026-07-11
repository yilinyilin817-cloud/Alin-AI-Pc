pub mod audio_io;
pub mod screen;

#[cfg(feature = "camera")]
pub mod camera;

/// 截屏
pub use screen::capture_screen;

/// 摄像头采集（feature-gated）
#[cfg(feature = "camera")]
pub use camera::capture_camera;

#[cfg(not(feature = "camera"))]
pub fn capture_camera() -> anyhow::Result<Vec<u8>> {
    anyhow::bail!("摄像头功能未启用（编译时未开启 camera feature）")
}
