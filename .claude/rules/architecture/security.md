# 보안 및 안티치트 정책

> BattlEye 밴 방지가 프로젝트의 생존 요건.

---

## ✅ BattlEye 호환성 체크리스트

**새 기능 추가 시 반드시 자가 점검.** 하나라도 YES면 즉시 중단.

- [ ] 게임 프로세스 메모리에 접근하는가?
- [ ] DLL을 게임 프로세스에 로드하는가?
- [ ] 게임 창에 오버레이를 그리는가?
- [ ] 게임 설정 파일을 수정하는가? (읽기만 OK)
- [ ] 게임 네트워크 트래픽을 가로채는가?
- [ ] 게임 프로세스에 signal을 보내는가?

---

## 🔓 READ-ONLY 허용 목록

다음은 명시적으로 안전:

- ✅ 화면 픽셀 수집 (DXGI/WGC, 게임 외부에서)
- ✅ OS 레벨 마우스 입력 조회 (GetRawInputData)
- ✅ OS 레벨 오디오 캡처 (WASAPI, 시스템 오디오만)
- ✅ `GameUserSettings.ini` 파일 읽기 (사용자 문서 폴더)
- ✅ 게임 창 크기/모드 조회 (외부 API로만)

---

## 🔐 비밀 정보 관리

### 로컬 개발
- `.env.local` 사용, **절대 커밋 금지**
- `.gitignore`에 반드시 포함

### 서버
- Supabase Vault 또는 AWS Secrets Manager
- 환경 변수는 배포 시 CI/CD Secret에서 주입

### 클라이언트
- **API 키를 클라이언트에 넣지 않음**
- 모든 서버 호출은 사용자 JWT 경유
- Tauri `tauri-plugin-store` 사용 시 암호화 옵션 활성화

---

## 👤 사용자 데이터 정책

### 로컬 저장
- **비디오 캡처는 로컬에만** 저장
- 서버 업로드 일체 금지
- Auto-Purge로 디스크 용량 관리

### 서버 전송 허용 JSON 필드
- ✅ 감도 수치, 오차율
- ✅ 게임명, 세션 ID
- ✅ 사용 통계 (익명 집계)

### 서버 전송 금지 필드
- ❌ IP 주소 직접 저장 (Supabase 자체 로그는 OK)
- ❌ 마우스 제조사 외 PII
- ❌ PUBG 닉네임

---

## 📋 감사 로그 (Audit Log)

**목적**: HARD LIMITS 준수를 코드 레벨에서 증명.

### 기록 대상
- 모든 화면 캡처 이벤트 (트리거 소스, 타임스탬프, API 종류)
- 모든 외부 프로세스 실행 (FFmpeg, Python 사이드카)
- 모든 서버 API 호출
- 설정 변경

### 저장
- 위치: `client/src-tauri/data/audit.db` (SQLite)
- 보존: 30일 자동 rotation
- 포맷: `audit_log` 테이블 (상세: `data-model.md`)

### 감사 로그 사용 예 (Rust)
```rust
use crate::audit::{log_event, EventType, TriggerSource};

pub async fn start_capture(source: TriggerSource) -> Result<(), CaptureError> {
    log_event(EventType::CaptureStarted, source, None).await?;
    // ... 캡처 로직
    log_event(EventType::CaptureCompleted, source, Some(meta)).await?;
    Ok(())
}
```

---

## 🚨 사고 대응 (Incident Response)

### BattlEye 밴 보고가 들어오면
1. **즉시 서비스 일시 중단** 공지 (Discord)
2. 사용자 버전 / 로그 수집
3. 해당 버전 배포 롤백 (Tauri 업데이트로 강제)
4. 24시간 내 `docs/adr/` 에 원인 분석 문서 작성
5. 수정 패치 배포 후 재개

### 보안 취약점 제보 (CVE)
- 이메일: security@[도메인]
- 제보 후 90일 공개 정책
- 수정 완료 전까지 비공개 유지
