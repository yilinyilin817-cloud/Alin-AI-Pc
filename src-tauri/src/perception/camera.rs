use anyhow::Result;

/// 摄像头采集（feature-gated）
#[cfg(feature = "camera")]
pub fn capture_camera() -> Result<Vec<u8>> {
    use nokhwa::Camera;

    let mut camera = Camera::new(0).map_err(|e| anyhow::anyhow!("camera open: {e}"))?;
    let frame = camera
        .frame()
        .map_err(|e| anyhow::anyhow!("frame: {e}"))?;
    Ok(frame.buffer().to_vec())
}

#[cfg(not(feature = "camera"))]
pub fn capture_camera() -> Result<Vec<u8>> {
    anyhow::bail!("摄像头功能未启用（编译时未开启 camera feature）")
}
