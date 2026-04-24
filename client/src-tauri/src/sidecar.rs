//! Python 사이드카 프로세스 관리.
//!
//! Tauri 앱 시작 시 `sidecar-x86_64-pc-windows-msvc.exe`를 subprocess로 실행.
//! 사이드카는 127.0.0.1:18080 에서 FastAPI 서버로 동작.

use tauri::AppHandle;
use tauri_plugin_shell::ShellExt;

/// 사이드카 포트.
pub const SIDECAR_PORT: u16 = 18080;

/// 사이드카 subprocess를 시작한다.
///
/// # Errors
/// 사이드카 바이너리를 찾을 수 없거나 실행에 실패하면 에러를 반환.
pub fn spawn(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let _child = app
        .shell()
        .sidecar("sidecar")?
        .args(["--port", &SIDECAR_PORT.to_string()])
        .spawn()?;

    Ok(())
}
