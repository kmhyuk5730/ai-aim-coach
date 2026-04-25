//! GetRawInputData 기반 마우스 Δ 수집 모듈.
//!
//! Windows Raw Input API로 OS 마우스 가속(포인터 정밀도)을 우회하여
//! 하드웨어 수준의 ΔX/ΔY를 수집합니다.
//!
//! # 동작 구조
//! - 백그라운드 스레드에 메시지 전용 창(`HWND_MESSAGE`) 생성
//! - `RegisterRawInputDevices(RIDEV_INPUTSINK)` — 포커스 없이도 수신
//! - `WM_INPUT` → `GetRawInputData` → `RAWMOUSE.lLastX/lLastY` 추출
//! - `thread_local!` DELTA_SINK + `Arc<Mutex<Vec<MouseDelta>>>` 경유 수집
//!
//! # 안티치트 안전성 (BattlEye 준수)
//! - 게임 프로세스 메모리 접근 없음
//! - OS 레벨 마우스 입력 조회만 수행 (`GetRawInputData`)

use std::{
    cell::RefCell,
    mem::size_of,
    sync::{mpsc, Arc, Mutex},
    thread,
};

use thiserror::Error;
use tracing::{debug, info, warn};
use windows::{
    Win32::{
        Foundation::{HINSTANCE, HWND, LPARAM, LRESULT, WPARAM},
        System::{
            LibraryLoader::GetModuleHandleW,
            Threading::GetCurrentThreadId,
        },
        UI::{
            Input::{
                GetRawInputData, RegisterRawInputDevices, HRAWINPUT, RAWINPUT,
                RAWINPUTDEVICE, RAWINPUTDEVICE_FLAGS, RAWINPUTHEADER, RID_INPUT,
                RIM_TYPEMOUSE,
            },
            WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW,
                GetMessageW, PostThreadMessageW, RegisterClassExW, HWND_MESSAGE,
                MSG, WINDOW_EX_STYLE, WINDOW_STYLE, WNDCLASSEXW, WM_INPUT, WM_QUIT,
            },
        },
    },
};

/// `RAWINPUTDEVICE.dwFlags` — 포커스 없이도 입력 수신.
/// 값: `RIDEV_INPUTSINK = 0x0000_0100`
const RIDEV_INPUTSINK: u32 = 0x0000_0100;

/// Windows 마우스 가속 없는 하드웨어 수준 마우스 이동 델타.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseDelta {
    /// X축 이동 (양수=오른쪽).
    pub dx: i32,
    /// Y축 이동 (양수=아래).
    pub dy: i32,
}

/// Raw Input 수집 에러.
#[derive(Error, Debug)]
pub enum RawInputError {
    /// Windows API 에러.
    #[error("Windows API 에러: {0}")]
    Windows(#[from] windows::core::Error),

    /// 스레드 생성 실패.
    #[error("스레드 생성 실패: {0}")]
    ThreadSpawn(#[from] std::io::Error),

    /// 스레드 초기화 응답 없음.
    #[error("Raw Input 스레드 초기화 응답 없음")]
    ThreadInitFailed,
}

// WndProc가 동일 스레드에서 호출되므로 thread_local로 공유 버퍼 전달.
thread_local! {
    static DELTA_SINK: RefCell<Option<Arc<Mutex<Vec<MouseDelta>>>>> =
        RefCell::new(None);
}

/// GetRawInputData 기반 마우스 Δ 수집기.
///
/// 백그라운드 스레드에서 메시지 전용 창을 생성하고
/// `RIDEV_INPUTSINK`로 마우스 Raw Input을 수신합니다.
///
/// # 사용 예
/// ```no_run
/// use ai_aim_coach_lib::input::raw_input::RawMouseCollector;
///
/// let collector = RawMouseCollector::new().expect("Raw Input 초기화 실패");
/// // 마우스를 이동한 뒤
/// let deltas = collector.take_deltas();
/// println!("수집된 델타: {deltas:?}");
/// ```
pub struct RawMouseCollector {
    buffer: Arc<Mutex<Vec<MouseDelta>>>,
    thread_id: u32,
    _thread: thread::JoinHandle<()>,
}

impl RawMouseCollector {
    /// Raw Input 수집기를 초기화하고 백그라운드 스레드를 시작한다.
    ///
    /// # Errors
    /// - [`RawInputError::Windows`] — Windows API 호출 실패
    /// - [`RawInputError::ThreadSpawn`] — 스레드 생성 실패
    /// - [`RawInputError::ThreadInitFailed`] — 스레드 초기화 응답 없음
    pub fn new() -> Result<Self, RawInputError> {
        let buffer = Arc::new(Mutex::new(Vec::<MouseDelta>::new()));
        let buffer_for_thread = Arc::clone(&buffer);

        // 스레드 준비 완료 신호 채널 (thread_id 또는 에러)
        let (ready_tx, ready_rx) = mpsc::sync_channel::<Result<u32, RawInputError>>(1);

        let thread = thread::Builder::new()
            .name("raw-input-collector".into())
            .spawn(move || {
                run_collector_thread(buffer_for_thread, ready_tx);
            })?;

        // 스레드가 메시지 루프에 진입했음을 확인한 뒤 반환
        let thread_id = ready_rx
            .recv()
            .map_err(|_| RawInputError::ThreadInitFailed)??;

        info!("Raw Input 수집기 초기화 완료 (스레드 ID: {thread_id})");

        Ok(Self {
            buffer,
            thread_id,
            _thread: thread,
        })
    }

    /// 누적된 마우스 델타를 전부 가져오고 버퍼를 비운다.
    ///
    /// # Returns
    /// 마지막 호출 이후 수집된 `MouseDelta` 목록.
    pub fn take_deltas(&self) -> Vec<MouseDelta> {
        self.buffer
            .lock()
            .map(|mut g| g.drain(..).collect())
            .unwrap_or_default()
    }
}

impl Drop for RawMouseCollector {
    fn drop(&mut self) {
        // Safety: WM_QUIT를 메시지 루프 스레드에 전송해 정상 종료.
        if let Err(e) = unsafe {
            PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0))
        } {
            warn!("Raw Input 스레드 종료 신호 전송 실패: {e}");
        }
    }
}

// ─── 백그라운드 스레드 ────────────────────────────────────────────────────────

fn run_collector_thread(
    buffer: Arc<Mutex<Vec<MouseDelta>>>,
    ready_tx: mpsc::SyncSender<Result<u32, RawInputError>>,
) {
    // thread_local에 공유 버퍼 설정 (WndProc이 동일 스레드에서 호출됨)
    DELTA_SINK.with(|s| *s.borrow_mut() = Some(Arc::clone(&buffer)));

    let hwnd = match create_message_window() {
        Ok(h) => h,
        Err(e) => {
            let _ = ready_tx.send(Err(e));
            return;
        }
    };

    if let Err(e) = register_raw_input_device(hwnd) {
        // Safety: 생성한 창을 오류 시 정리.
        unsafe { DestroyWindow(hwnd).ok() };
        let _ = ready_tx.send(Err(e));
        return;
    }

    // Safety: GetCurrentThreadId는 항상 유효한 스레드 ID 반환.
    let thread_id = unsafe { GetCurrentThreadId() };

    // 메시지 루프 진입 전 준비 완료 신호
    let _ = ready_tx.send(Ok(thread_id));

    run_message_loop();

    // Safety: 메시지 루프 종료 후 창 정리.
    unsafe { DestroyWindow(hwnd).ok() };

    DELTA_SINK.with(|s| *s.borrow_mut() = None);
}

/// 메시지 전용 창(`HWND_MESSAGE`)을 생성한다.
fn create_message_window() -> Result<HWND, RawInputError> {
    let hinstance = unsafe {
        let h = GetModuleHandleW(None)?;
        HINSTANCE(h.0)
    };

    let class_name = windows::core::w!("AAC_RawInput");

    let wc = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(raw_input_wnd_proc),
        hInstance: hinstance,
        lpszClassName: class_name,
        ..Default::default()
    };

    // 이미 등록된 경우 무시 (다중 초기화 허용)
    unsafe { RegisterClassExW(&wc) };

    // Safety: HWND_MESSAGE 부모로 화면 렌더링 없는 메시지 전용 창 생성.
    let hwnd = unsafe {
        CreateWindowExW(
            WINDOW_EX_STYLE(0),
            class_name,
            windows::core::w!(""),
            WINDOW_STYLE(0),
            0,
            0,
            0,
            0,
            HWND_MESSAGE,
            None,
            hinstance,
            None,
        )
    };

    if hwnd.0 == 0 {
        return Err(RawInputError::Windows(windows::core::Error::from_win32()));
    }

    Ok(hwnd)
}

/// 마우스 Raw Input 장치를 등록한다 (`RIDEV_INPUTSINK`).
fn register_raw_input_device(hwnd: HWND) -> Result<(), RawInputError> {
    let device = RAWINPUTDEVICE {
        usUsagePage: 0x01, // HID_USAGE_PAGE_GENERIC
        usUsage: 0x02,     // HID_USAGE_GENERIC_MOUSE
        dwFlags: RAWINPUTDEVICE_FLAGS(RIDEV_INPUTSINK),
        hwndTarget: hwnd,
    };

    // Safety: device 슬라이스는 유효한 RAWINPUTDEVICE 배열.
    unsafe {
        RegisterRawInputDevices(&[device], size_of::<RAWINPUTDEVICE>() as u32)?;
    }

    Ok(())
}

/// Win32 메시지 루프를 실행한다.
fn run_message_loop() {
    let mut msg = MSG::default();
    loop {
        // Safety: 표준 Win32 GetMessage 메시지 루프 패턴.
        let ret = unsafe { GetMessageW(&mut msg, HWND::default(), 0, 0) };
        match ret.0 {
            -1 => {
                warn!("GetMessageW 에러 발생");
                break;
            }
            0 => break, // WM_QUIT
            _ => {
                // Safety: 표준 메시지 디스패치.
                unsafe { DispatchMessageW(&msg) };
            }
        }
    }
}

// ─── WndProc ─────────────────────────────────────────────────────────────────

/// Raw Input 전용 WndProc.
///
/// # Safety
/// Win32 콜백 함수. 모든 포인터는 Windows가 유효함을 보장한다.
unsafe extern "system" fn raw_input_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_INPUT {
        process_raw_input(HRAWINPUT(lparam.0));
    }
    // MSDN: WM_INPUT 처리 후 DefWindowProc 호출로 Raw Input 버퍼 해제 필수.
    DefWindowProcW(hwnd, msg, wparam, lparam)
}

/// `WM_INPUT` 메시지에서 마우스 Δ를 추출해 DELTA_SINK에 기록한다.
fn process_raw_input(handle: HRAWINPUT) {
    let mut size: u32 = 0;

    // 1차 호출: 필요한 버퍼 크기 조회.
    // Safety: pdata = None이면 size만 채워줌 (표준 두 번 호출 패턴).
    let ret = unsafe {
        GetRawInputData(
            handle,
            RID_INPUT,
            None,
            &mut size,
            size_of::<RAWINPUTHEADER>() as u32,
        )
    };
    if ret != 0 || size == 0 {
        return;
    }

    let mut buf = vec![0u8; size as usize];

    // 2차 호출: 실제 데이터 수신.
    // Safety: buf는 GetRawInputData가 요구한 크기로 할당됨.
    let written = unsafe {
        GetRawInputData(
            handle,
            RID_INPUT,
            Some(buf.as_mut_ptr() as *mut _),
            &mut size,
            size_of::<RAWINPUTHEADER>() as u32,
        )
    };
    if written == 0 || written == u32::MAX {
        return;
    }

    // Safety: buf는 RAWINPUT 구조체를 담기에 충분한 크기로 할당됐으며
    //         GetRawInputData가 정상적으로 채워줌.
    let raw = unsafe { &*(buf.as_ptr() as *const RAWINPUT) };

    if raw.header.dwType != RIM_TYPEMOUSE.0 {
        return; // 키보드 등 마우스 외 장치 무시
    }

    // Safety: dwType == RIM_TYPEMOUSE이므로 data.mouse 필드 접근이 유효.
    let (dx, dy) = unsafe { (raw.data.mouse.lLastX, raw.data.mouse.lLastY) };

    if dx == 0 && dy == 0 {
        return; // 버튼 이벤트 등 이동 없는 입력 무시
    }

    debug!(dx, dy, "마우스 Raw 델타 수신");

    DELTA_SINK.with(|sink| {
        if let Some(buf) = sink.borrow().as_ref() {
            if let Ok(mut guard) = buf.lock() {
                guard.push(MouseDelta { dx, dy });
            }
        }
    });
}

// ─── 테스트 ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_mouse_collector_initializes() {
        // CI 환경(헤드리스)에서도 패닉 없이 에러 반환해야 함.
        match RawMouseCollector::new() {
            Ok(collector) => {
                let deltas = collector.take_deltas();
                assert!(
                    deltas.is_empty(),
                    "초기화 직후 take_deltas는 빈 벡터를 반환해야 함"
                );
                println!("Raw Input 수집기 초기화 성공");
            }
            Err(e) => println!("Raw Input 초기화 실패 (CI 환경 가능): {e}"),
        }
    }

    /// 실제 마우스 이동이 필요한 Δ 수집 테스트.
    /// `cargo test -- --ignored` 로 로컬에서 실행.
    #[test]
    #[ignore = "마우스 이동 필요 — 로컬에서만 실행"]
    fn test_collect_mouse_deltas_on_movement() {
        use std::{thread, time::Duration};

        let collector = RawMouseCollector::new().expect("Raw Input 초기화 실패");

        println!("3초 안에 마우스를 이동하세요...");
        thread::sleep(Duration::from_secs(3));

        let deltas = collector.take_deltas();
        println!("수집된 델타 수: {}", deltas.len());
        for d in deltas.iter().take(10) {
            println!("  dx={}, dy={}", d.dx, d.dy);
        }

        assert!(!deltas.is_empty(), "마우스 이동 후 델타가 수집되어야 함");
    }
}
