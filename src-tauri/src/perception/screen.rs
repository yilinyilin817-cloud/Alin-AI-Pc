use anyhow::Result;

/// 截屏（feature-gated）
#[cfg(feature = "perception")]
pub fn capture_screen() -> Result<Vec<u8>> {
    let screenshots = screenshots::Screen::all().map_err(|e| anyhow::anyhow!("screens: {e}"))?;
    if screenshots.is_empty() {
        anyhow::bail!("No screens found");
    }
    let image = screenshots[0]
        .capture()
        .map_err(|e| anyhow::anyhow!("capture: {e}"))?;
    Ok(image.into_raw())
}

#[cfg(not(feature = "perception"))]
pub fn capture_screen() -> Result<Vec<u8>> {
    anyhow::bail!("截屏功能未启用（编译时未开启 perception feature）")
}
