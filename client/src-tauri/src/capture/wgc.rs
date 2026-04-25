//! Windows Graphics Capture (WGC) 캡처 모듈.
//!
//! Windows 10 1903+ WGC API를 사용해 Borderless/Windowed 화면 모드를 캡처합니다.
//! DXGI Desktop Duplication의 2차 수단으로 사용됩니다.
//!
//! # 안티치트 안전성 (BattlEye 준수)
//! - 게임 프로세스 메모리 접근 없음
//! - DLL 인젝션 없음
//! - 게임 창 오버레이 없음
//! - OS 레벨 화면 읽기만 수행 (Windows Graphics Capture API)
//!
//! # 최소 요구사항
//! Windows 10 버전 1903 (빌드 18362) 이상

use std::{
    sync::{
        atomic::{AtomicU64, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};

use thiserror::Error;
use tracing::{debug, info, warn};
use windows::{
    core::Interface,
    Foundation::{EventRegistrationToken, TypedEventHandler},
    Graphics::{
        Capture::{Direct3D11CaptureFramePool, GraphicsCaptureItem, GraphicsCaptureSession},
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
        SizeInt32,
    },
    Win32::{
        Foundation::{HMODULE, POINT},
        Graphics::{
            Direct3D::D3D_DRIVER_TYPE_HARDWARE,
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, D3D11_CREATE_DEVICE_FLAG,
                D3D11_SDK_VERSION,
            },
            Dxgi::{IDXGIAdapter, IDXGIDevice},
            Gdi::{MonitorFromPoint, HMONITOR, MONITOR_DEFAULTTOPRIMARY},
        },
        System::WinRT::{
            Direct3D11::CreateDirect3D11DeviceFromDXGIDevice,
            Graphics::Capture::IGraphicsCaptureItemInterop,
            RoInitialize, RO_INIT_MULTITHREADED,
        },
    },
};

/// WGC 캡처 에러.
#[derive(Error, Debug)]
pub enum WgcError {
    /// Windows API 또는 WinRT 에러.
    #[error("Windows API 에러: {0}")]
    Windows(#[from] windows::core::Error),

    /// 기본 모니터를 찾을 수 없음.
    #[error("기본 모니터를 찾을 수 없습니다 (모니터 연결 확인)")]
    NoMonitor,
}

/// WGC 기반 화면 캡처기.
///
/// Windows Graphics Capture API로 기본 모니터를 캡처합니다.
/// Borderless Fullscreen 및 Windowed 모드에서 안정적으로 동작합니다.
///
/// # 사용 예
/// ```no_run
/// use std::time::Duration;
/// use ai_aim_coach_lib::capture::wgc::WgcCapturer;
///
/// let capturer = WgcCapturer::new().expect("WGC 초기화 실패");
/// let fps = capturer.measure_fps(Duration::from_secs(3));
/// println!("측정 FPS: {fps:.1}");
/// ```
pub struct WgcCapturer {
    _device: ID3D11Device,
    _item: GraphicsCaptureItem,
    frame_pool: Direct3D11CaptureFramePool,
    _session: GraphicsCaptureSession,
    event_token: EventRegistrationToken,
    frame_counter: Arc<AtomicU64>,
}

impl WgcCapturer {
    /// 기본 모니터 대상 WGC 캡처기를 초기화한다.
    ///
    /// # Errors
    /// - [`WgcError::Windows`] — Windows API 또는 WinRT 호출 실패
    /// - [`WgcError::NoMonitor`] — 기본 모니터를 찾을 수 없음
    pub fn new() -> Result<Self, WgcError> {
        // Safety: WinRT 런타임 초기화.
        //         S_FALSE(이미 초기화됨) 또는 RPC_E_CHANGED_MODE(다른 모드로 초기화됨)는
        //         정상 상황이므로 .ok()로 무시한다.
        unsafe { RoInitialize(RO_INIT_MULTITHREADED).ok() };

        let (device, _ctx) = Self::create_d3d11_device()?;
        let winrt_device = Self::to_winrt_device(&device)?;
        let monitor = Self::get_primary_monitor()?;
        let item = Self::create_capture_item(monitor)?;
        let size = item.Size()?;

        let frame_pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            &winrt_device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2, // 버퍼 2개 — 레이턴시/성능 균형
            size,
        )?;

        let frame_counter = Arc::new(AtomicU64::new(0));
        let counter_ref = Arc::clone(&frame_counter);

        // FrameArrived: 새 프레임 도착 시 즉시 소비 후 카운터 증가.
        // 프레임을 소비하지 않으면 프레임 풀 버퍼가 가득 차 캡처가 중단됨.
        let event_token = frame_pool.FrameArrived(&TypedEventHandler::new(
            move |pool: &Option<Direct3D11CaptureFramePool>, _| {
                if let Some(p) = pool {
                    if let Ok(frame) = p.TryGetNextFrame() {
                        drop(frame);
                        counter_ref.fetch_add(1, Ordering::Relaxed);
                        debug!("WGC 프레임 수신");
                    }
                }
                Ok(())
            },
        ))?;

        let session = frame_pool.CreateCaptureSession(&item)?;
        session.StartCapture()?;

        info!("Windows Graphics Capture 초기화 완료 ({}×{})", size.Width, size.Height);

        Ok(Self {
            _device: device,
            _item: item,
            frame_pool,
            _session: session,
            event_token,
            frame_counter,
        })
    }

    /// D3D11 하드웨어 디바이스와 즉시 컨텍스트를 생성한다.
    fn create_d3d11_device() -> Result<(ID3D11Device, ID3D11DeviceContext), WgcError> {
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;

        // Safety: Windows D3D11 API 호출. 출력 포인터는 성공 시 API가 채워줌.
        unsafe {
            D3D11CreateDevice(
                None::<&IDXGIAdapter>,
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),
                D3D11_CREATE_DEVICE_FLAG(0),
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )?;
        }

        let device = device.ok_or(WgcError::NoMonitor)?;
        let context = context.ok_or(WgcError::NoMonitor)?;
        Ok((device, context))
    }

    /// `ID3D11Device`를 WinRT `IDirect3DDevice`로 변환한다.
    ///
    /// WGC API는 WinRT `IDirect3DDevice` 인터페이스를 요구한다.
    fn to_winrt_device(device: &ID3D11Device) -> Result<IDirect3DDevice, WgcError> {
        // Safety: COM/WinRT 인터페이스 캐스팅.
        //         IDXGIDevice → IInspectable → IDirect3DDevice 변환 체인.
        unsafe {
            let dxgi_device: IDXGIDevice = device.cast()?;
            let inspectable = CreateDirect3D11DeviceFromDXGIDevice(&dxgi_device)?;
            let winrt_device: IDirect3DDevice = inspectable.cast()?;
            Ok(winrt_device)
        }
    }

    /// 좌표 (0, 0)으로부터 기본 모니터 핸들을 가져온다.
    fn get_primary_monitor() -> Result<HMONITOR, WgcError> {
        // Safety: MonitorFromPoint는 MONITOR_DEFAULTTOPRIMARY 설정 시
        //         항상 유효한 핸들을 반환한다.
        let monitor =
            unsafe { MonitorFromPoint(POINT { x: 0, y: 0 }, MONITOR_DEFAULTTOPRIMARY) };
        if monitor.0 == 0 {
            return Err(WgcError::NoMonitor);
        }
        Ok(monitor)
    }

    /// `HMONITOR`로부터 WGC `GraphicsCaptureItem`을 생성한다.
    fn create_capture_item(monitor: HMONITOR) -> Result<GraphicsCaptureItem, WgcError> {
        // Safety: WinRT 활성화 팩토리 획득 및 COM 인터페이스 메서드 호출.
        unsafe {
            let interop =
                windows::core::factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
            let item: GraphicsCaptureItem = interop.CreateForMonitor(monitor)?;
            Ok(item)
        }
    }

    /// 지정된 기간 동안 수신된 프레임 수를 측정해 FPS를 계산한다.
    ///
    /// `FrameArrived` 이벤트 카운터를 기반으로 실제 화면 갱신 FPS를 측정합니다.
    ///
    /// # Arguments
    /// * `duration` — 측정 기간 (권장: 3초 이상)
    ///
    /// # Returns
    /// 측정된 FPS. 측정 실패 시 0.0 반환.
    pub fn measure_fps(&self, duration: Duration) -> f64 {
        self.frame_counter.store(0, Ordering::Relaxed);
        let start = Instant::now();

        thread::sleep(duration);

        let elapsed = start.elapsed().as_secs_f64();
        let frames = self.frame_counter.load(Ordering::Relaxed) as f64;
        let fps = if elapsed > 0.0 { frames / elapsed } else { 0.0 };

        info!(
            fps,
            frames_total = frames as u64,
            elapsed_secs = elapsed,
            "WGC FPS 측정 완료"
        );
        fps
    }
}

impl Drop for WgcCapturer {
    fn drop(&mut self) {
        // FrameArrived 핸들러를 명시적으로 해제해 이벤트 루프 정리.
        if let Err(e) = self.frame_pool.RemoveFrameArrived(self.event_token) {
            warn!("WGC FrameArrived 핸들러 해제 중 에러: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wgc_capturer_initializes() {
        // 디스플레이가 없는 CI 환경에서는 에러를 반환하지만 패닉하지 않아야 함.
        match WgcCapturer::new() {
            Ok(_) => println!("WGC 초기화 성공"),
            Err(e) => println!("WGC 초기화 실패 (디스플레이 없음 가능): {e}"),
        }
    }

    /// 실제 디스플레이가 필요한 FPS 측정 테스트.
    /// `cargo test -- --ignored` 로 로컬에서 실행.
    #[test]
    #[ignore = "실제 디스플레이 필요 — 로컬에서만 실행"]
    fn test_measure_fps_above_60() {
        let capturer = WgcCapturer::new().expect("WGC 초기화 실패");
        let fps = capturer.measure_fps(Duration::from_secs(3));
        println!("측정 FPS: {fps:.1}");
        // 브라우저 합성 오버헤드를 감안해 58 FPS를 임계값으로 설정.
        // 실제 게임(Borderless Fullscreen)에서는 모니터 리프레시율과 동일하게 측정됨.
        assert!(fps >= 58.0, "FPS가 58 미만: {fps:.1}");
    }
}
