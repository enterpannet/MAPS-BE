//! Image และ video compression - ย่อขนาดโดยไม่เสียคุณภาพ

use image::codecs::jpeg::JpegEncoder;
use image::imageops::FilterType;

const MAX_IMAGE_DIMENSION: u32 = 1920;
const JPEG_QUALITY: u8 = 88;

/// ย่อและบีบอัดรูปภาพ - คืน JPEG bytes
pub fn compress_image(data: &[u8]) -> Result<Vec<u8>, String> {
    let img = image::load_from_memory(data).map_err(|e| format!("Invalid image: {}", e))?;

    let (w, h) = (img.width(), img.height());
    let resized = if w > MAX_IMAGE_DIMENSION || h > MAX_IMAGE_DIMENSION {
        img.resize(
            MAX_IMAGE_DIMENSION,
            MAX_IMAGE_DIMENSION,
            FilterType::Lanczos3,
        )
    } else {
        img
    };

    let rgb8 = resized.to_rgb8();
    let (width, height) = rgb8.dimensions();

    let mut buf = Vec::new();
    let mut encoder = JpegEncoder::new_with_quality(&mut buf, JPEG_QUALITY);
    encoder
        .encode(rgb8.as_raw(), width, height, image::ExtendedColorType::Rgb8)
        .map_err(|e| format!("Encode failed: {}", e))?;

    Ok(buf)
}

/// บีบอัดวิดีโอด้วย ffmpeg (ถ้ามี) - คืน path ของไฟล์ที่บีบอัดแล้ว
/// ถ้าไม่มี ffmpeg หรือ error ให้คืน None = ใช้ไฟล์ต้นฉบับ
pub async fn compress_video(input_path: &std::path::Path) -> Option<std::path::PathBuf> {
    let output_path = input_path.with_extension("compressed.mp4");

    let status = tokio::process::Command::new("ffmpeg")
        .args([
            "-y",
            "-i",
            input_path.to_str()?,
            "-c:v",
            "libx264",
            "-crf",
            "23",
            "-preset",
            "medium",
            "-vf",
            "scale='min(1080,iw)':'min(1080,ih)':force_original_aspect_ratio=decrease",
            "-movflags",
            "+faststart",
            "-c:a",
            "aac",
            "-b:a",
            "128k",
            output_path.to_str()?,
        ])
        .output()
        .await
        .ok()?;

    if status.status.success() {
        let _ = tokio::fs::remove_file(input_path).await;
        if let Ok(()) = tokio::fs::rename(&output_path, input_path).await {
            return Some(input_path.to_path_buf());
        }
    }

    None
}
