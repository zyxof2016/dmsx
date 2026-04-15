use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

pub(super) async fn primary_capture_size() -> Result<(u32, u32), String> {
    tokio::task::spawn_blocking(|| {
        let display = scrap::Display::primary().map_err(|e| e.to_string())?;
        Ok::<_, String>((display.width() as u32, display.height() as u32))
    })
    .await
    .map_err(|e| format!("capture init task join failed: {e}"))?
}

pub(super) fn spawn_capture_loop(
    capture_source: libwebrtc::video_source::native::NativeVideoSource,
    stop_flag: Arc<AtomicBool>,
) -> tokio::task::JoinHandle<()> {
    tokio::task::spawn_blocking(move || {
        use libwebrtc::video_frame::{VideoFrame, VideoRotation};

        let display = match scrap::Display::primary() {
            Ok(d) => d,
            Err(e) => {
                tracing::error!("failed to get primary display: {e}");
                return;
            }
        };

        let width = display.width() as u32;
        let height = display.height() as u32;
        let mut capturer = match scrap::Capturer::new(display) {
            Ok(c) => c,
            Err(e) => {
                tracing::error!("failed to create capturer: {e}");
                return;
            }
        };

        let frame_interval = std::time::Duration::from_millis(100);
        while !stop_flag.load(Ordering::Relaxed) {
            match capturer.frame() {
                Ok(frame) => {
                    if let Some(buffer) = bgra_to_i420_buffer(&frame, width, height) {
                        let video_frame = VideoFrame {
                            rotation: VideoRotation::VideoRotation0,
                            timestamp_us: 0,
                            buffer,
                        };
                        capture_source.capture_frame(&video_frame);
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
                Err(e) => {
                    tracing::error!("capture error: {e}");
                    break;
                }
            }
            std::thread::sleep(frame_interval);
        }
    })
}

fn bgra_to_i420_buffer(
    bgra: &[u8],
    width: u32,
    height: u32,
) -> Option<libwebrtc::video_frame::I420Buffer> {
    use libwebrtc::video_frame::I420Buffer;

    let expected = (width * height * 4) as usize;
    if bgra.len() < expected {
        return None;
    }

    let mut buffer = I420Buffer::new(width, height);
    let (y_plane, u_plane, v_plane) = buffer.data_mut();
    let width_usize = width as usize;
    let height_usize = height as usize;
    let chroma_width = width_usize.div_ceil(2);

    for y in (0..height_usize).step_by(2) {
        for x in (0..width_usize).step_by(2) {
            let mut u_acc = 0f32;
            let mut v_acc = 0f32;
            let mut samples = 0f32;

            for dy in 0..2 {
                for dx in 0..2 {
                    let px = x + dx;
                    let py = y + dy;
                    if px >= width_usize || py >= height_usize {
                        continue;
                    }

                    let idx = (py * width_usize + px) * 4;
                    let b = bgra[idx] as f32;
                    let g = bgra[idx + 1] as f32;
                    let r = bgra[idx + 2] as f32;

                    let y_value = (0.257 * r + 0.504 * g + 0.098 * b + 16.0)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                    y_plane[py * width_usize + px] = y_value;

                    u_acc += -0.148 * r - 0.291 * g + 0.439 * b + 128.0;
                    v_acc += 0.439 * r - 0.368 * g - 0.071 * b + 128.0;
                    samples += 1.0;
                }
            }

            let uv_index = (y / 2) * chroma_width + (x / 2);
            u_plane[uv_index] = (u_acc / samples).round().clamp(0.0, 255.0) as u8;
            v_plane[uv_index] = (v_acc / samples).round().clamp(0.0, 255.0) as u8;
        }
    }

    Some(buffer)
}
