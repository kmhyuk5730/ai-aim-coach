---
paths:
  - "bots/**/*.py"
  - ".github/workflows/ops-agent.yml"
---

# AI 운영 자동화

> Sentry → Claude 에이전트 → Discord → 원클릭 승인 파이프라인.
> 이게 경쟁사(Razer Shot) 대비 **진짜 차별화 포인트**.

---

## 🔄 전체 파이프라인

```
[사용자 크래시]
     ↓
[Sentry 캡처]
     ↓
[Webhook → ops_agent (Claude API)]
     ↓
[Claude가 분석]
  ├─ 스택 트레이스 읽기
  ├─ 원인 분류 (UI / 캡처 / 인코딩 / 서버)
  ├─ 심각도 판정 (P0/P1/P2)
  └─ 수정 패치 초안 생성 (가능 시)
     ↓
[Discord 알림 → 개발자]
  ┌─────────────────────────────┐
  │ 🚨 [P1] Crash in capture    │
  │ File: dxgi.rs:142           │
  │ Affected: 3 users (30min)   │
  │ [분석] [패치 제안]            │
  │ [승인] [거부] [상세 보기]    │
  └─────────────────────────────┘
     ↓
[개발자 '승인' 클릭]
     ↓
[GitHub Actions]
  ├─ 자동 PR 생성
  ├─ 테스트 통과 확인
  └─ 자동 머지 → 배포
```

---

## ⚖️ 자동화 규칙

### 자동 머지 허용
- P2 (개선) 이슈
- 테스트 커버리지 영향 없음
- 파일 변경 < 50줄
- 보안/안티치트 관련 파일 아님

### 자동 머지 **금지**
- P0 (블로커) 또는 보안 관련 → **반드시 사람 확인**
- `src-tauri/src/capture/` 변경 (BattlEye 민감)
- 의존성 변경 (`Cargo.toml`, `package.json`, `requirements.txt`)
- 50줄 초과 변경

### 타임아웃
- 승인 대기 24시간 → 자동 취소
- 취소 시 이슈로 자동 전환

### 롤백 자동화
- 배포 후 1시간 내 Sentry 에러율 **평소의 3배 이상** 급증
- 즉시 자동 롤백 + Discord 알림

---

## 📝 감사 로그

모든 자동 처리 내역은 **`bots/ops_agent/audit.log`**:

```
[2026-04-24T15:30:00Z] P1 crash detected in dxgi.rs:142
[2026-04-24T15:30:15Z] Claude analysis: race condition suspected
[2026-04-24T15:30:20Z] Discord notification sent to #dev-alerts
[2026-04-24T15:45:30Z] Developer approved patch
[2026-04-24T15:46:00Z] PR #123 created and auto-merged
[2026-04-24T15:50:00Z] Deployed to production
[2026-04-24T16:00:00Z] Error rate normal, no rollback needed
```

---

## 🤖 Discord 봇 기능

### 유저 대면
- 문의/버그 제보 **티켓 자동 생성**
- 카테고리 자동 분류 (감도교정 / 결제 / 버그 / 기타)
- FAQ 자동 답변 (Claude 연동)

### 개발자 대면
- 크래시 알림 (위 파이프라인)
- 유료 구독 결제 확인
- 일일/주간 사용 통계
- 새 가입자 수 알림

### 자동 응대 예시
```
[사용자]: 감도 적용이 안돼요
[봇]: 감도 미적용 이슈는 보통 GameUserSettings.ini 권한 때문입니다.
     다음을 확인해 주세요:
     1. PUBG를 관리자 권한으로 실행했는지
     2. 문서 폴더에 쓰기 권한이 있는지
     해결되지 않으면 '티켓 생성' 버튼을 눌러주세요.
     [티켓 생성] [FAQ 보기]
```

---

## 🎯 왜 이 시스템이 차별화 포인트인가

**Razer Shot의 약점**:
- 솔로/소규모 팀 → 버그 수정 속도 느림
- PUBG 패치 시 호환성 이슈 빈번
- 디스코드 버그 제보가 수동 처리

**AI Aim Coach의 강점**:
- 크래시 감지부터 수정 배포까지 **자동화**
- PUBG 패치 시 상수 테이블 자동 업데이트
- 사용자 체감: "**버그 생겨도 하루 안에 고쳐진다**"

이게 **"작은 팀이 큰 팀처럼 움직이는" 진짜 경쟁력**.

---

## 🔐 보안 주의사항

### API 키 관리
- Claude API 키: GitHub Secrets에만
- Discord Bot 토큰: Secrets + 최소 권한
- Sentry DSN: 서버 환경변수에만

### 승인 없는 자동 배포 금지
- 코드 변경 전 반드시 개발자 Discord 승인
- 감사 로그 없이 배포 금지
- 롤백 트리거는 사람이 수동 해제 가능해야 함

---

## 📎 관련 문서
- Phase 3 AC-3, AC-4 (이 기능 구현 시점): `.claude/rules/process/roadmap.md`
- Python 코딩 규칙: `.claude/rules/coding/python.md` (자동 로드됨)
