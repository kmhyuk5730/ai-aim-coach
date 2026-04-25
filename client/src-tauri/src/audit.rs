//! 감사 로그 모듈.
//!
//! BattlEye 준수를 코드 레벨에서 증명하기 위해 모든 캡처 이벤트를 기록합니다.
//!
//! # Phase 0
//! tracing으로 로깅합니다. Phase 2에서 SQLite 저장소로 교체 예정.

use tracing::info;

/// 감사 이벤트 유형.
#[derive(Debug, Clone, Copy)]
pub enum EventType {
    /// 화면 캡처 시작.
    CaptureStarted,
    /// 화면 캡처 완료.
    CaptureCompleted,
    /// 외부 프로세스 실행.
    ProcessSpawned,
    /// 설정 변경.
    ConfigChanged,
}

/// 이벤트 트리거 소스.
#[derive(Debug, Clone, Copy)]
pub enum TriggerSource {
    /// 사용자 UI 커맨드.
    UserCommand,
    /// 애플리케이션 자동 실행.
    AppAuto,
}

/// 감사 이벤트를 기록한다.
///
/// # Arguments
/// * `event_type` — 이벤트 유형
/// * `source` — 트리거 소스
/// * `detail` — 추가 세부 정보 (선택)
pub fn log_event(event_type: EventType, source: TriggerSource, detail: Option<&str>) {
    info!(
        event = ?event_type,
        source = ?source,
        detail = detail.unwrap_or("-"),
        "[감사로그]"
    );
}
