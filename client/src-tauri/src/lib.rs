pub mod audit;
pub mod capture;
pub mod input;
mod sidecar;

use std::time::Duration;

use audit::{EventType, TriggerSource};
use capture::dxgi::DxgiCapturer;

/// DXGI Desktop Duplication으로 3초간 캡처하여 FPS를 측정한다.
///
/// # Returns
/// 측정된 FPS. 초기화 실패 시 에러 메시지 반환.
#[tauri::command]
fn start_capture_test() -> Result<f64, String> {
    audit::log_event(EventType::CaptureStarted, TriggerSource::UserCommand, None);

    let capturer = DxgiCapturer::new().map_err(|e| e.to_string())?;
    let fps = capturer.measure_fps(Duration::from_secs(3));

    audit::log_event(
        EventType::CaptureCompleted,
        TriggerSource::UserCommand,
        Some(&format!("{fps:.1} FPS")),
    );

    Ok(fps)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            sidecar::spawn(app.handle())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![start_capture_test])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
