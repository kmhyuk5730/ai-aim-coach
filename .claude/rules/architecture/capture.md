---
paths:
  - "client/src-tauri/src/capture/**/*.rs"
  - "client/src-tauri/src/display/**/*.rs"
---

# 캡처 전략 상세

> PUBG 화면을 어떻게 캡처할 것인가. Multi-Tier 전략.

---

## 🎯 Multi-Tier 캡처 (핵심 결정)

PUBG는 3가지 모드로 실행되며, 각 모드마다 다른 API가 필요합니다:

| 게임 모드 | 1순위 API | 2순위 API | 타겟 유저 |
|-----------|-----------|-----------|-----------|
| Exclusive Fullscreen | **DXGI Desktop Duplication** | 없음 | 프로/준프로 주력 |
| Borderless Fullscreen | **WGC** | DXGI | 캐주얼 주력 |
| Windowed | **WGC** | DXGI | 드묾 |

**왜 WGC 단일이 안 되는가**:
- WGC는 Exclusive Fullscreen에서 불안정 (Microsoft 공식 답변)
- HAGS + HDR 설정 요구
- PUBG 프로층은 Exclusive Fullscreen 선호

---

## 🔍 자동 모드 감지 로직

```rust
pub enum GameMode {
    ExclusiveFullscreen,
    BorderlessFullscreen,
    Windowed,
}

pub fn detect_game_mode(hwnd: HWND) -> Result<GameMode, CaptureError> {
    // 1. 윈도우 스타일 확인 (WS_POPUP, WS_CAPTION 등)
    // 2. 화면 크기와 모니터 크기 비교
    // 3. DXGI swap chain 정보 확인
    // ...
}

pub async fn start_capture(mode: GameMode) -> Result<Capture, CaptureError> {
    match mode {
        GameMode::ExclusiveFullscreen => {
            dxgi::start().await
                .or_else(|_| Err(CaptureError::NoCompatibleApi))
        }
        GameMode::BorderlessFullscreen | GameMode::Windowed => {
            wgc::start().await
                .or_else(|_| dxgi::start().await)
        }
    }
}
```

### 실패 시 동작
1. 1순위 API 실패 → 2순위 자동 시도
2. 2순위도 실패 → 사용자에게 명확한 안내 메시지
3. 절대로 자동 재시도 무한 루프 금지

---

## 📺 해상도/주사율 자동 인식

**다중 계층 조회**:
1. `EnumDisplaySettings` (Windows 기본 API) — 1순위
2. DXGI `GetDisplayModeList` — Fallback
3. `GameUserSettings.ini` 파싱 — 게임 내 설정값 확인
4. 실측 Δt(프레임 도착 시간차) — 실제 FPS 검증

**핵심 공식**: `Δt = timestamp[n] - timestamp[n-1]`

### 주사율 계산 예시
```rust
pub fn calculate_actual_fps(timestamps: &[Instant]) -> f64 {
    if timestamps.len() < 2 {
        return 0.0;
    }
    let deltas: Vec<f64> = timestamps.windows(2)
        .map(|w| w[1].duration_since(w[0]).as_secs_f64())
        .collect();
    let avg_delta = deltas.iter().sum::<f64>() / deltas.len() as f64;
    1.0 / avg_delta
}
```

---

## 🚨 BattlEye 호환성 (절대 지킬 것)

- ✅ 화면 픽셀만 읽음 (게임 외부에서)
- ✅ 게임 프로세스에 접근 금지
- ✅ 오버레이 그리지 않음
- ❌ 절대 게임 메모리 접근 금지
- ❌ DLL 인젝션 금지

### 캡처 함수 진입/종료 감사 로그 필수
```rust
use crate::audit::{log_event, EventType};

pub async fn capture_frame(trigger: TriggerSource) -> Result<Frame, CaptureError> {
    log_event(EventType::CaptureStarted, trigger, None).await?;
    let result = internal_capture().await;
    log_event(EventType::CaptureCompleted, trigger, Some(&result)).await?;
    result
}
```

---

## ⚙️ 성능 목표

- 최소 60 FPS 캡처 (Phase 1 AC-1)
- 메모리 누수 없음 (10분 연속 실행 시 RSS 증가 < 50MB)
- CPU 사용률 < 10% (게임 플레이 방해 최소화)

---

## 📎 관련 문서
- 보안 정책: `.claude/rules/architecture/security.md`
- Rust 코딩 규칙: `.claude/rules/coding/rust.md` (자동 로드됨)
