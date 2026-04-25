//! FFmpeg 서브프로세스 하드웨어 인코딩 모듈.
//!
//! BGR24 원시 프레임을 FFmpeg stdin 파이프로 전달하여 H.264 .mp4를 생성합니다.
//! 하드웨어 인코더를 자동 프로브하고 사용 불가 시 소프트웨어 폴백합니다.
//!
//! # 인코더 우선순위
//! 1. `h264_nvenc` — NVIDIA NVENC
//! 2. `h264_amf`   — AMD AMF
//! 3. `h264_qsv`   — Intel Quick Sync
//! 4. `libx264`    — 소프트웨어 폴백 (항상 사용 가능)
//!
//! # 동작 구조
//! ```text
//! FfmpegPipe::new()
//!   ├─ find_ffmpeg()       — PATH에서 ffmpeg 탐색
//!   └─ HwEncoder::probe()  — 1프레임 테스트 인코딩으로 실제 가용 여부 확인
//!
//! encode_test_frames()
//!   ├─ ffmpeg -f rawvideo -pix_fmt bgr24 -s WxH -r FPS -i pipe:0
//!   │         -c:v {encoder} -y output.mp4
//!   ├─ stdin 파이프로 합성 BGR24 프레임 전송
//!   └─ wait_with_output() → 종료 코드 확인
//! ```

use std::{
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use thiserror::Error;
use tracing::{debug, info};

/// 하드웨어 인코더 종류.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HwEncoder {
    /// NVIDIA NVENC (`h264_nvenc`).
    Nvenc,
    /// AMD AMF (`h264_amf`).
    Amf,
    /// Intel Quick Sync (`h264_qsv`).
    Qsv,
    /// 소프트웨어 폴백 (`libx264`).
    Software,
}

impl HwEncoder {
    /// FFmpeg 코덱 이름을 반환한다.
    pub fn ffmpeg_name(self) -> &'static str {
        match self {
            HwEncoder::Nvenc => "h264_nvenc",
            HwEncoder::Amf => "h264_amf",
            HwEncoder::Qsv => "h264_qsv",
            HwEncoder::Software => "libx264",
        }
    }

    /// 1프레임 테스트 인코딩으로 실제 가용 여부를 확인한다.
    ///
    /// `ffmpeg -f lavfi` 합성 소스를 사용하므로 실제 입력 파일이 불필요하다.
    fn is_available(ffmpeg: &Path, encoder: &str) -> bool {
        Command::new(ffmpeg)
            .args([
                "-f", "lavfi",
                "-i", "color=black:s=16x16:r=1",
                "-frames:v", "1",
                "-c:v", encoder,
                "-f", "null",
                "-",
            ])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    }

    /// 하드웨어 인코더를 우선순위 순으로 프로브하고 첫 번째 가용 인코더를 반환한다.
    ///
    /// 모두 실패하면 `Software`(`libx264`)를 반환한다.
    fn probe(ffmpeg: &Path) -> Self {
        for encoder in [Self::Nvenc, Self::Amf, Self::Qsv] {
            debug!("인코더 프로브 중: {}", encoder.ffmpeg_name());
            if Self::is_available(ffmpeg, encoder.ffmpeg_name()) {
                info!("선택된 하드웨어 인코더: {}", encoder.ffmpeg_name());
                return encoder;
            }
        }
        info!("하드웨어 인코더 없음 → 소프트웨어 폴백 (libx264)");
        Self::Software
    }
}

impl std::fmt::Display for HwEncoder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.ffmpeg_name())
    }
}

/// FFmpeg 인코딩 에러.
#[derive(Error, Debug)]
pub enum FfmpegError {
    /// FFmpeg 바이너리를 찾을 수 없음.
    #[error("FFmpeg 바이너리를 찾을 수 없습니다 (PATH 확인 또는 번들 FFmpeg 배치 필요)")]
    NotFound,

    /// FFmpeg 인코딩 실패.
    #[error("FFmpeg 인코딩 실패:\n{0}")]
    EncodingFailed(String),

    /// 프로세스 I/O 에러.
    #[error("프로세스 I/O 에러: {0}")]
    Io(#[from] std::io::Error),
}

/// FFmpeg 서브프로세스 기반 H.264 인코더.
///
/// # 사용 예
/// ```no_run
/// use std::path::Path;
/// use ai_aim_coach_lib::ffmpeg::pipe::FfmpegPipe;
///
/// let pipe = FfmpegPipe::new().expect("FFmpeg 초기화 실패");
/// println!("선택된 인코더: {}", pipe.selected_encoder());
/// pipe.encode_test_frames(1280, 720, 60, 10, Path::new("out.mp4"))
///     .expect("인코딩 실패");
/// ```
pub struct FfmpegPipe {
    ffmpeg_path: PathBuf,
    encoder: HwEncoder,
}

impl FfmpegPipe {
    /// FFmpeg를 탐색하고 사용 가능한 최우선 인코더를 프로브하여 초기화한다.
    ///
    /// # Errors
    /// - [`FfmpegError::NotFound`] — 시스템 PATH에 `ffmpeg`가 없음
    pub fn new() -> Result<Self, FfmpegError> {
        let ffmpeg_path = find_ffmpeg().ok_or(FfmpegError::NotFound)?;
        let encoder = HwEncoder::probe(&ffmpeg_path);
        info!(
            ffmpeg = %ffmpeg_path.display(),
            encoder = %encoder,
            "FFmpeg 파이프 초기화 완료"
        );
        Ok(Self { ffmpeg_path, encoder })
    }

    /// 선택된 인코더를 반환한다.
    pub fn selected_encoder(&self) -> HwEncoder {
        self.encoder
    }

    /// 합성 BGR24 프레임을 FFmpeg stdin 파이프로 전달하여 .mp4를 생성한다.
    ///
    /// 실제 캡처 프레임 없이 파이프라인 동작을 검증하는 스파이크 메서드입니다.
    /// 프레임은 단색(검정) BGR24 데이터로 채워집니다.
    ///
    /// # Arguments
    /// * `width`       — 프레임 너비 (픽셀)
    /// * `height`      — 프레임 높이 (픽셀)
    /// * `fps`         — 초당 프레임 수
    /// * `frame_count` — 생성할 프레임 수
    /// * `output_path` — 출력 .mp4 경로
    ///
    /// # Errors
    /// - [`FfmpegError::Io`] — stdin 쓰기 또는 프로세스 실행 실패
    /// - [`FfmpegError::EncodingFailed`] — FFmpeg 비정상 종료
    pub fn encode_test_frames(
        &self,
        width: u32,
        height: u32,
        fps: u32,
        frame_count: u32,
        output_path: &Path,
    ) -> Result<(), FfmpegError> {
        debug!(
            encoder = %self.encoder,
            width, height, fps, frame_count,
            output = %output_path.display(),
            "FFmpeg 인코딩 시작"
        );

        let mut child = Command::new(&self.ffmpeg_path)
            .args([
                "-f", "rawvideo",
                "-pix_fmt", "bgr24",
                "-s", &format!("{width}x{height}"),
                "-r", &fps.to_string(),
                "-i", "pipe:0",
                "-c:v", self.encoder.ffmpeg_name(),
            ])
            .args(quality_args(self.encoder))
            .arg("-y")
            .arg(output_path.as_os_str())
            .stdin(Stdio::piped())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()?;

        // stdin 파이프 소유권 취득 후 프레임 전송
        {
            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| FfmpegError::Io(std::io::Error::other("stdin 취득 실패")))?;

            let frame = vec![0u8; (width * height * 3) as usize]; // 검정 프레임
            for _ in 0..frame_count {
                stdin.write_all(&frame)?;
            }
            // stdin drop → FFmpeg에 EOF 전송
        }

        let exit = child.wait_with_output()?;

        if !exit.status.success() {
            let stderr = String::from_utf8_lossy(&exit.stderr).into_owned();
            return Err(FfmpegError::EncodingFailed(stderr));
        }

        info!(
            output = %output_path.display(),
            encoder = %self.encoder,
            "FFmpeg 인코딩 완료"
        );
        Ok(())
    }
}

/// 인코더별 품질 옵션을 반환한다.
fn quality_args(encoder: HwEncoder) -> &'static [&'static str] {
    match encoder {
        HwEncoder::Nvenc => &["-preset", "fast"],
        HwEncoder::Amf => &[], // AMF 기본값 사용
        HwEncoder::Qsv => &["-preset", "fast"],
        HwEncoder::Software => &["-preset", "fast", "-crf", "23"],
    }
}

/// 시스템 PATH에서 `ffmpeg` 바이너리를 탐색한다.
///
/// `-version` 명령으로 실행 가능 여부를 확인한다.
fn find_ffmpeg() -> Option<PathBuf> {
    let ok = Command::new("ffmpeg")
        .arg("-version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if ok {
        Some(PathBuf::from("ffmpeg"))
    } else {
        None
    }
}

// ─── 테스트 ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ffmpeg_pipe_new_no_crash() {
        // CI 환경에 FFmpeg 없으면 NotFound, 패닉 없음.
        match FfmpegPipe::new() {
            Ok(pipe) => {
                println!("FFmpeg 발견, 선택된 인코더: {}", pipe.selected_encoder());
            }
            Err(FfmpegError::NotFound) => {
                println!("FFmpeg 없음 (CI 환경 가능) — NotFound 에러 정상");
            }
            Err(e) => println!("초기화 실패: {e}"),
        }
    }

    #[test]
    fn hw_encoder_ffmpeg_names_are_correct() {
        assert_eq!(HwEncoder::Nvenc.ffmpeg_name(), "h264_nvenc");
        assert_eq!(HwEncoder::Amf.ffmpeg_name(), "h264_amf");
        assert_eq!(HwEncoder::Qsv.ffmpeg_name(), "h264_qsv");
        assert_eq!(HwEncoder::Software.ffmpeg_name(), "libx264");
    }

    /// FFmpeg 설치 + 실제 인코딩이 필요한 통합 테스트.
    /// `cargo test -- --ignored` 로 로컬에서 실행.
    #[test]
    #[ignore = "FFmpeg 설치 필요 — 로컬에서만 실행"]
    fn test_encode_test_frames_produces_mp4() {
        let pipe = FfmpegPipe::new().expect("FFmpeg 초기화 실패");
        println!("선택된 인코더: {}", pipe.selected_encoder());

        let output_path = std::env::temp_dir().join("aac_spike_test.mp4");
        pipe.encode_test_frames(1280, 720, 60, 30, &output_path)
            .expect("인코딩 실패");

        assert!(output_path.exists(), ".mp4 파일이 생성되어야 함");
        let size = std::fs::metadata(&output_path).unwrap().len();
        assert!(size > 0, ".mp4 파일 크기가 0보다 커야 함");
        println!(
            "인코딩 성공: {} ({size} bytes)",
            output_path.display()
        );

        // 임시 파일 정리
        std::fs::remove_file(&output_path).ok();
    }
}
