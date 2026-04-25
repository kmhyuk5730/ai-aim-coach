//! DXGI Desktop Duplication 캡처 모듈.
//!
//! Windows DXGI Desktop Duplication API를 사용해 화면을 캡처합니다.
//! Exclusive Fullscreen을 포함한 모든 화면 모드를 지원합니다.
//!
//! # 안티치트 안전성 (BattlEye 준수)
//! - 게임 프로세스 메모리 접근 없음
//! - DLL 인젝션 없음
//! - 게임 창 오버레이 없음
//! - OS 레벨 화면 읽기만 수행 (DXGI Desktop Duplication API)
//!
//! # Safety
//! 내부 unsafe 블록은 Windows DXGI/D3D11 API 호출에만 사용됩니다.
//! 모든 포인터는 API 호출 전 초기화 검증됩니다.

use std::time::{Duration, Instant};

use thiserror::Error;
use tracing::{debug, info, warn};
use windows::{
    core::Interface,
    Win32::{
        Foundation::HMODULE,
        Graphics::{
            Direct3D::D3D_DRIVER_TYPE_HARDWARE,
            Direct3D11::{
                D3D11CreateDevice, ID3D11Device, ID3D11DeviceContext, D3D11_CREATE_DEVICE_FLAG,
                D3D11_SDK_VERSION,
            },
            Dxgi::{
                IDXGIAdapter, IDXGIDevice, IDXGIOutput1, IDXGIOutputDuplication, IDXGIResource,
                DXGI_ERROR_ACCESS_LOST, DXGI_ERROR_WAIT_TIMEOUT, DXGI_OUTDUPL_FRAME_INFO,
            },
        },
    },
};

/// DXGI 캡처 에러.
#[derive(Error, Debug)]
pub enum CaptureError {
    /// Windows DXGI/D3D11 API 에러.
    #[error("Windows API 에러: {0}")]
    Windows(#[from] windows::core::Error),

    /// 디스플레이 출력을 찾을 수 없음.
    #[error("디스플레이 출력을 찾을 수 없습니다 (모니터 연결 확인)")]
    NoOutput,

    /// 프레임 획득 타임아웃 (화면 변화 없음).
    #[error("프레임 획득 타임아웃")]
    Timeout,

    /// 디스플레이 접근 권한 상실 (화면 잠금, 해상도 변경 등).
    #[error("디스플레이 접근 권한 상실 — 재초기화 필요")]
    AccessLost,
}

/// DXGI Desktop Duplication 기반 화면 캡처기.
///
/// # 사용 예
/// ```no_run
/// use std::time::Duration;
/// use ai_aim_coach_lib::capture::dxgi::DxgiCapturer;
///
/// let capturer = DxgiCapturer::new().expect("캡처기 초기화 실패");
/// let fps = capturer.measure_fps(Duration::from_secs(3));
/// println!("측정 FPS: {fps:.1}");
/// ```
pub struct DxgiCapturer {
    _device: ID3D11Device,
    duplication: IDXGIOutputDuplication,
}

impl DxgiCapturer {
    /// 첫 번째 디스플레이 어댑터 출력에 대한 캡처기를 생성한다.
    ///
    /// # Errors
    /// - [`CaptureError::Windows`] — D3D11 디바이스 생성 또는 DXGI 초기화 실패
    /// - [`CaptureError::NoOutput`] — 디스플레이 출력을 찾을 수 없음
    pub fn new() -> Result<Self, CaptureError> {
        let (device, _ctx) = Self::create_d3d11_device()?;
        let duplication = Self::create_duplication(&device)?;
        info!("DXGI Desktop Duplication 초기화 완료");
        Ok(Self {
            _device: device,
            duplication,
        })
    }

    /// D3D11 하드웨어 디바이스와 즉시 컨텍스트를 생성한다.
    fn create_d3d11_device() -> Result<(ID3D11Device, ID3D11DeviceContext), CaptureError> {
        let mut device: Option<ID3D11Device> = None;
        let mut context: Option<ID3D11DeviceContext> = None;

        // Safety: Windows API 호출. 모든 출력 포인터는 Some()으로 전달되어
        //         API가 성공 시 반드시 채워줌.
        unsafe {
            D3D11CreateDevice(
                None::<&IDXGIAdapter>, // 기본 어댑터 사용
                D3D_DRIVER_TYPE_HARDWARE,
                HMODULE::default(),          // 소프트웨어 래스터라이저 없음
                D3D11_CREATE_DEVICE_FLAG(0), // 디버그 플래그 없음
                None,                        // 기본 Feature Level
                D3D11_SDK_VERSION,
                Some(&mut device),
                None, // Feature Level 출력 불필요
                Some(&mut context),
            )?;
        }

        let device = device.ok_or(CaptureError::NoOutput)?;
        let context = context.ok_or(CaptureError::NoOutput)?;
        Ok((device, context))
    }

    /// D3D11 디바이스 → DXGI 어댑터 → 출력 → OutputDuplication 생성.
    fn create_duplication(device: &ID3D11Device) -> Result<IDXGIOutputDuplication, CaptureError> {
        // Safety: COM 인터페이스 캐스팅 및 DXGI API 호출.
        unsafe {
            let dxgi_device: IDXGIDevice = device.cast()?;
            let adapter = dxgi_device.GetAdapter()?;
            let output = adapter.EnumOutputs(0).map_err(|_| CaptureError::NoOutput)?;
            let output1: IDXGIOutput1 = output.cast()?;
            let duplication = output1.DuplicateOutput(device)?;
            Ok(duplication)
        }
    }

    /// 단일 프레임을 캡처하고 누적 프레임 수를 반환한다.
    ///
    /// `AccumulatedFrames`를 사용해 마지막 캡처 이후 실제로 렌더링된
    /// 화면 프레임 수를 정확히 반영합니다.
    ///
    /// # Returns
    /// 누적된 화면 프레임 수 (0 이상)
    ///
    /// # Errors
    /// - [`CaptureError::Timeout`] — 새 프레임 없음 (정상 상황)
    /// - [`CaptureError::AccessLost`] — 화면 잠금 등으로 접근 권한 상실
    /// - [`CaptureError::Windows`] — 그 외 DXGI API 에러
    pub fn capture_frame(&self) -> Result<u32, CaptureError> {
        let mut frame_info = DXGI_OUTDUPL_FRAME_INFO::default();
        let mut resource: Option<IDXGIResource> = None;

        // Safety: Windows DXGI API 호출. frame_info와 resource는 이 함수가
        //         소유하며 AcquireNextFrame 이후에만 사용됨.
        let result = unsafe {
            self.duplication
                .AcquireNextFrame(17, &mut frame_info, &mut resource)
        };

        match result {
            Ok(()) => {
                let accum = frame_info.AccumulatedFrames.max(1);
                debug!(accum_frames = accum, "프레임 캡처 성공");
                // Safety: AcquireNextFrame 성공 후 반드시 ReleaseFrame 호출.
                unsafe { self.duplication.ReleaseFrame()? };
                Ok(accum)
            }
            Err(e) if e.code() == DXGI_ERROR_WAIT_TIMEOUT => Err(CaptureError::Timeout),
            Err(e) if e.code() == DXGI_ERROR_ACCESS_LOST => Err(CaptureError::AccessLost),
            Err(e) => Err(CaptureError::Windows(e)),
        }
    }

    /// 지정된 기간 동안 연속 캡처하며 실제 FPS를 측정한다.
    ///
    /// `AccumulatedFrames`를 누적해 실제 렌더링된 화면 프레임 수를 기준으로
    /// FPS를 계산합니다. 타임아웃은 재시도하며, 그 외 에러 시 중단합니다.
    ///
    /// # Arguments
    /// * `duration` — 측정 기간 (권장: 3초 이상)
    ///
    /// # Returns
    /// 측정된 FPS. 측정 실패 시 0.0 반환.
    pub fn measure_fps(&self, duration: Duration) -> f64 {
        let start = Instant::now();
        let mut total_frames: u64 = 0;

        while start.elapsed() < duration {
            match self.capture_frame() {
                Ok(accum) => total_frames += u64::from(accum),
                Err(CaptureError::Timeout) => continue,
                Err(e) => {
                    warn!("FPS 측정 중 에러 발생: {e}");
                    break;
                }
            }
        }

        let elapsed = start.elapsed().as_secs_f64();
        let fps = if elapsed > 0.0 {
            total_frames as f64 / elapsed
        } else {
            0.0
        };

        info!(fps, total_frames, elapsed_secs = elapsed, "FPS 측정 완료");
        fps
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dxgi_capturer_initializes() {
        // 디스플레이가 없는 CI 환경에서는 에러를 반환하지만 패닉하지 않아야 함.
        match DxgiCapturer::new() {
            Ok(_) => println!("DXGI 초기화 성공"),
            Err(e) => println!("DXGI 초기화 실패 (디스플레이 없음 가능): {e}"),
        }
    }

    /// 실제 디스플레이가 필요한 FPS 측정 테스트.
    /// `cargo test -- --ignored` 로 로컬에서 실행.
    #[test]
    #[ignore = "실제 디스플레이 필요 — 로컬에서만 실행"]
    fn test_measure_fps_above_60() {
        let capturer = DxgiCapturer::new().expect("DXGI 초기화 실패");
        let fps = capturer.measure_fps(Duration::from_secs(3));
        println!("측정 FPS: {fps:.1}");
        // 브라우저 합성 오버헤드를 감안해 58 FPS를 임계값으로 설정.
        // 실제 게임(Exclusive Fullscreen)에서는 모니터 리프레시율과 동일하게 측정됨.
        assert!(fps >= 58.0, "FPS가 58 미만: {fps:.1}");
    }
}
