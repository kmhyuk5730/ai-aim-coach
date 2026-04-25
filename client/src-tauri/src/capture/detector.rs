//! 게임 화면 모드 자동 감지 모듈.
//!
//! PUBG 창의 화면 모드(Exclusive Fullscreen / Borderless / Windowed)를
//! Win32 창 스타일, 모니터 크기 비교, WGC 캡처 가능성 3단계로 판별합니다.
//!
//! # 판별 흐름
//! ```text
//! HWND 입력
//!   ├─ IsWindow? 아니오 → InvalidWindow
//!   ├─ WS_CAPTION / WS_THICKFRAME 있음? → Windowed
//!   ├─ 창이 모니터를 채우지 않음? → Windowed
//!   └─ WGC CreateForWindow 성공? → Borderless / 실패 → Exclusive
//! ```
//!
//! # 안티치트 안전성 (BattlEye 준수)
//! - 게임 프로세스 메모리 접근 없음
//! - 창 핸들 및 스타일 조회만 수행 (읽기 전용 Win32 API)

use std::mem::size_of;

use thiserror::Error;
use tracing::debug;
use windows::{
    core::Interface,
    Graphics::Capture::GraphicsCaptureItem,
    Win32::{
        Foundation::{HWND, RECT},
        Graphics::Gdi::{
            GetMonitorInfoW, MonitorFromWindow, MONITOR_DEFAULTTONEAREST, MONITORINFO,
        },
        System::WinRT::Graphics::Capture::IGraphicsCaptureItemInterop,
        UI::WindowsAndMessaging::{
            FindWindowW, GetWindowLongW, GetWindowRect, IsWindow, GWL_STYLE, WS_CAPTION,
            WS_THICKFRAME,
        },
    },
};

/// 게임 화면 모드.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    /// DXGI 익스클루시브 풀스크린.
    ///
    /// DXGI Desktop Duplication을 1차로 사용합니다.
    ExclusiveFullscreen,

    /// 보더리스 풀스크린.
    ///
    /// WGC를 1차, DXGI를 2차로 사용합니다.
    BorderlessFullscreen,

    /// 창 모드.
    ///
    /// WGC를 1차, DXGI를 2차로 사용합니다.
    Windowed,
}

/// 게임 모드 감지 에러.
#[derive(Error, Debug)]
pub enum DetectorError {
    /// 유효하지 않은 창 핸들.
    #[error("유효하지 않은 창 핸들입니다")]
    InvalidWindow,

    /// Windows API 에러.
    #[error("Windows API 에러: {0}")]
    Windows(#[from] windows::core::Error),
}

/// 주어진 `HWND`의 게임 화면 모드를 판별한다.
///
/// # 판별 단계
/// 1. 창 스타일 확인 (`WS_CAPTION` / `WS_THICKFRAME`) — 있으면 Windowed
/// 2. 창 크기 vs 모니터 크기 비교 — 채우지 않으면 Windowed
/// 3. WGC `CreateForWindow` 성공 여부 — 성공이면 Borderless, 실패이면 Exclusive
///
/// # Errors
/// - [`DetectorError::InvalidWindow`] — `hwnd`가 유효하지 않은 창 핸들
/// - [`DetectorError::Windows`] — Win32 API 호출 실패
pub fn detect_game_mode(hwnd: HWND) -> Result<GameMode, DetectorError> {
    // Safety: IsWindow는 어떤 HWND 값에 대해서도 안전하게 호출 가능.
    if !unsafe { IsWindow(hwnd) }.as_bool() {
        return Err(DetectorError::InvalidWindow);
    }

    // 1단계: WS_CAPTION / WS_THICKFRAME → Windowed
    // Safety: GWL_STYLE 조회는 읽기 전용 API.
    let style = unsafe { GetWindowLongW(hwnd, GWL_STYLE) } as u32;
    let has_caption = (style & WS_CAPTION.0) != 0;
    let has_thickframe = (style & WS_THICKFRAME.0) != 0;

    if has_caption || has_thickframe {
        debug!(has_caption, has_thickframe, "창 모드 감지 (스타일)");
        return Ok(GameMode::Windowed);
    }

    // 2단계: 창 Rect vs 모니터 Rect 비교 → 채우지 않으면 Windowed
    let mut window_rect = RECT::default();
    // Safety: hwnd 유효성은 IsWindow로 검증됨.
    unsafe { GetWindowRect(hwnd, &mut window_rect)? };

    let monitor = unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    let mut monitor_info = MONITORINFO {
        cbSize: size_of::<MONITORINFO>() as u32,
        ..Default::default()
    };
    // Safety: monitor_info는 cbSize가 초기화된 유효한 구조체.
    if !unsafe { GetMonitorInfoW(monitor, &mut monitor_info) }.as_bool() {
        return Err(DetectorError::Windows(windows::core::Error::from_win32()));
    }

    if window_rect != monitor_info.rcMonitor {
        debug!("창 모드 감지 (화면 미충전)");
        return Ok(GameMode::Windowed);
    }

    // 3단계: WGC CreateForWindow — Borderless vs Exclusive 구분
    // DWM이 합성하는 창(Borderless)은 성공, Exclusive Fullscreen은 실패.
    // Safety: WinRT 활성화 팩토리 획득 및 COM 인터페이스 호출.
    let mode = unsafe {
        let interop =
            windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        match interop.CreateForWindow::<GraphicsCaptureItem>(hwnd) {
            Ok(_) => {
                debug!("Borderless Fullscreen 감지 (WGC 성공)");
                GameMode::BorderlessFullscreen
            }
            Err(_) => {
                debug!("Exclusive Fullscreen 감지 (WGC 실패)");
                GameMode::ExclusiveFullscreen
            }
        }
    };

    Ok(mode)
}

/// PUBG 창 핸들(`HWND`)을 탐색한다.
///
/// Unreal Engine 창 클래스명 `UnrealWindow`로 검색합니다.
/// PUBG가 실행 중이지 않으면 `None`을 반환합니다.
///
/// # Returns
/// PUBG 창 핸들. 실행 중이지 않으면 `None`.
pub fn find_pubg_window() -> Option<HWND> {
    // Safety: 읽기 전용 Win32 창 탐색.
    let hwnd = unsafe {
        FindWindowW(
            windows::core::w!("UnrealWindow"),
            windows::core::PCWSTR::null(),
        )
    };
    if hwnd.0 == 0 {
        None
    } else {
        debug!("PUBG 창 발견 (HWND: {hwnd:?})");
        Some(hwnd)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_null_hwnd_returns_invalid_window() {
        let result = detect_game_mode(HWND(0));
        assert!(
            matches!(result, Err(DetectorError::InvalidWindow)),
            "null HWND는 InvalidWindow 에러를 반환해야 함: {result:?}"
        );
    }

    #[test]
    fn test_find_pubg_window_no_crash() {
        // PUBG가 실행 중이지 않을 때 None 반환, 패닉 없음.
        let hwnd = find_pubg_window();
        println!("PUBG 창 핸들: {hwnd:?}");
        // CI 환경에서는 None이 기대값.
    }

    /// PUBG 실행 중 모드 감지 통합 테스트.
    /// `cargo test -- --ignored` 로 로컬에서 실행.
    #[test]
    #[ignore = "PUBG 실행 필요 — 로컬에서만 실행"]
    fn test_detect_pubg_returns_valid_mode() {
        let hwnd = find_pubg_window().expect("PUBG가 실행 중이지 않음");
        let mode = detect_game_mode(hwnd).expect("모드 감지 실패");
        println!("감지된 게임 모드: {mode:?}");
        assert!(
            matches!(
                mode,
                GameMode::ExclusiveFullscreen
                    | GameMode::BorderlessFullscreen
                    | GameMode::Windowed
            ),
            "알 수 없는 모드: {mode:?}"
        );
    }

    /// 데스크탑 창으로 감지 로직 동작 확인 (실제 모드는 환경에 따라 다름).
    /// `cargo test -- --ignored` 로 로컬에서 실행.
    #[test]
    #[ignore = "실제 디스플레이 필요 — 로컬에서만 실행"]
    fn test_detect_desktop_window_no_panic() {
        use windows::Win32::UI::WindowsAndMessaging::GetDesktopWindow;
        // Safety: GetDesktopWindow는 항상 유효한 HWND를 반환.
        let hwnd = unsafe { GetDesktopWindow() };
        let result = detect_game_mode(hwnd);
        println!("데스크탑 창 모드: {result:?}");
        assert!(result.is_ok(), "데스크탑 창 감지는 에러 없이 완료되어야 함");
    }
}
