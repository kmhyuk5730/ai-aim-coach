---
paths:
  - "client/src-tauri/**/*.rs"
  - "client/src-tauri/Cargo.toml"
---

# Rust 코딩 규칙

> Tauri 2 클라이언트 코어 전용.

---

## 🎯 기본 설정

- **Rust 2024 edition**, stable 채널
- **Clippy warnings zero**: `#![deny(clippy::all)]`
- **비동기**: Tokio
- **로깅**: `tracing` 크레이트
- **에러**: `thiserror` + `Result<T, E>`

---

## ❌ 금지 사항

- `.unwrap()` 사용 (테스트 외)
- `.expect()` 남발
- `unsafe` 블록 주석 없이 사용
- `println!` / `eprintln!` (`tracing` 매크로 사용)
- 광범위한 `Box<dyn Error>` 반환 (구체적 에러 타입만)

---

## ✅ 에러 처리 패턴

### 나쁜 예
```rust
fn capture() -> Vec<u8> {
    let data = unsafe { /* ... */ };
    data.unwrap()  // ❌ 금지
}
```

### 좋은 예
```rust
use thiserror::Error;
use tracing::{info, error};

#[derive(Error, Debug)]
pub enum CaptureError {
    #[error("DXGI API failed: {0}")]
    DxgiFailure(String),
    #[error("Game mode not detected")]
    ModeNotDetected,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// DXGI Desktop Duplication으로 프레임 캡처.
///
/// # Safety
/// 내부 unsafe 블록은 Windows DXGI API 호출에 필요.
/// D3D11Device 포인터는 초기화 시 검증된 것을 사용.
pub fn capture_frame() -> Result<Vec<u8>, CaptureError> {
    info!("Capturing frame via DXGI");
    // ...
    Ok(frame)
}
```

---

## 📝 문서화

모든 공개 함수/구조체/enum에 **rustdoc** 필수:

```rust
/// 게임 모드를 자동 감지.
///
/// # Arguments
/// * `hwnd` - 대상 창의 핸들
///
/// # Returns
/// 감지된 게임 모드
///
/// # Errors
/// * `CaptureError::ModeNotDetected` - 모드 판별 실패
pub fn detect_mode(hwnd: HWND) -> Result<GameMode, CaptureError> {
    // ...
}
```

---

## 🧪 테스트

### 단위 테스트 형식
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_capture_frame_dropped_returns_none() {
        // Arrange
        let capture = Capture::new_mock();
        
        // Act
        let result = capture.frame_at(999_999);
        
        // Assert
        assert!(result.is_none());
    }
    
    #[tokio::test]
    async fn test_async_capture_respects_timeout() {
        // ...
    }
}
```

### 테스트 명명 규칙
- `test_<기능>_<조건>_<기대결과>`
- 예: `test_capture_frame_dropped_returns_none`

### 필수 실행
```bash
cargo test          # 모든 테스트
cargo clippy -- -D warnings  # 린트
cargo fmt --check   # 포맷
```

---

## 🔒 감사 로그 필수

캡처, 외부 프로세스 실행, 설정 변경 시:

```rust
use crate::audit::{log_event, EventType, TriggerSource};

pub async fn start_capture(trigger: TriggerSource) -> Result<(), CaptureError> {
    log_event(EventType::CaptureStarted, trigger, None).await?;
    // ... 로직
    log_event(EventType::CaptureCompleted, trigger, Some(&meta)).await?;
    Ok(())
}
```

---

## 📦 의존성 추가 규칙

새 크레이트 추가 시 반드시 사용자 승인:
- 크레이트명 + 버전
- 용도
- 배포 크기 영향
- 라이선스 (MIT/Apache/BSD만 허용, GPL 금지)
- 유지보수 활성도 (최근 6개월 내 커밋 확인)

---

## 🎨 포맷

- `rustfmt` 기본 설정 사용
- 줄 길이 100자 이내
- 들여쓰기 4칸 (rustfmt 기본)
