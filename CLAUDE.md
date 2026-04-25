# AI Aim Coach — 프로젝트 헌법

> 이 파일은 **모든 세션 시작 시 자동 로드**됩니다.
> 세부 규칙은 `.claude/rules/` 아래 파일들이 **필요 시 자동 로드**됩니다.
> 수정 사항은 `docs/adr/`에 결정 기록으로 남깁니다.

---

## 🎯 프로젝트 개요

- **이름**: AI Aim Coach (AAC)
- **정의**: PUBG 사격 영상을 AI로 분석하여 감도를 자동 교정하는 **비침습적 후처리 데스크탑 도구**
- **버전**: 2.2.0 (2026-04-24)
- **소유자**: kmhyuk5730

---

## 🚨 절대 넘지 않는 선 (HARD LIMITS)

### 안티치트 관련 (BattlEye 밴 방지)
- ❌ 게임 프로세스 메모리 접근 금지
- ❌ DLL 인젝션 금지
- ❌ 게임 내 오버레이 금지 (분석은 별도 창만)
- ❌ 게임 설정 파일 수정 금지 (읽기만 OK)
- ❌ "안티치트 우회" 표현 코드/마케팅 어디에도 금지
- ❌ 크래프톤/BattlEye 상표를 마케팅 외 영역에 사용 금지
- ✅ DXGI / WGC / GetRawInputData / WASAPI 만 사용

### 배포 크기
- ❌ CUDA EP, PyTorch, TensorFlow 번들 금지
- ❌ Electron 사용 금지 (Tauri 2만)
- ✅ Lite 100MB / Standard 150MB 상한
- ✅ AI 모델 상한: Nano 10MB, Small 25MB

### UX
- ❌ 온보딩 3스텝 초과 금지
- ✅ 사용자 대면 텍스트는 한국어 우선 (영어는 Phase 4)

---

## 🏛 거버넌스: PO + Reviewer 6단계

**모든 태스크**는 반드시 이 순서로 진행합니다:

```
[1.PO 분석] → [2.사용자 승인] → [3.구현] →
[4.셀프 테스트] → [5.Reviewer 검증] → [6.커밋]
```

- 단계 건너뛰기 금지
- PO 승인 없이 구현 착수 금지
- Reviewer REJECT한 코드는 커밋 금지

**상세 양식**: `.claude/rules/process/governance.md`

---

## 🛠 핵심 기술 스택 (요약)

| 영역 | 기술 |
|------|------|
| 데스크탑 | Tauri 2 (Rust + React/TS) |
| 캡처 1차 | DXGI Desktop Duplication |
| 캡처 2차 | Windows Graphics Capture |
| 사이드카 | Python 3.11+ + FastAPI + PyInstaller |
| AI 추론 | ONNX Runtime + DirectML |
| AI 모델 | YOLO26n (무료) / YOLO26s (프리미엄) |
| 로컬 DB | SQLite |
| 서버 | FastAPI + Supabase |
| 결제 | Stripe |
| 운영 자동화 | Sentry + Claude API + Discord |

**전체 아키텍처 및 이유**: `.claude/rules/architecture/*.md`

---

## 📅 개발 단계

- **Phase 0 (0~1주)**: 환경 셋업
- **Phase 1 (2~5주)**: 기술 검증 스파이크 (프로젝트 실현 가능성 검증)
- **Phase 2 (6~16주)**: MVP 핵심 기능 (오차 5% 목표)
- **Phase 3 (17~26주)**: 운영 자동화 + 프리미엄 (오차 2% 목표)
- **Phase 4-A (27주~)**: 타 FPS 확장
- **Phase 4-B (Gate Review 후)**: 매치 분석 모드 (자기장/낙하)

**각 Phase의 상세 AC**: `.claude/rules/process/roadmap.md`

---

## 💰 수익 모델

| 티어 | 가격 | AI 모델 |
|------|------|---------|
| Free | 무료 (월 50회) | YOLO26n |
| Basic | 월 3,900원 | YOLO26n |
| Pro | 월 7,900원 | YOLO26s |

**Phase별 수익 목표**: `.claude/rules/process/roadmap.md`

---

## 🔀 규칙 파일 구조 (`.claude/rules/`)

Claude Code가 파일 성격에 따라 **자동으로 로드**합니다:

### 항상 로드 (모든 세션)
- `process/governance.md` — PO/Reviewer 워크플로우 상세
- `process/working-instructions.md` — 작업 지침
- `process/communication.md` — 커뮤니케이션 양식
- `process/roadmap.md` — Phase별 AC
- `architecture/overview.md` — 시스템 구조 개요
- `architecture/security.md` — BattlEye 정책
- `architecture/data-model.md` — 데이터 스키마
- `architecture/decision-log.md` — 기술 선택 근거

### 경로별 자동 로드 (해당 파일 작업 시에만)
- `coding/rust.md` — `src-tauri/**/*.rs` 작업 시
- `coding/typescript.md` — `*.ts,tsx` 작업 시
- `coding/python.md` — `sidecar/**/*.py`, `server/**/*.py` 작업 시
- `architecture/capture.md` — `src-tauri/src/capture/**` 작업 시
- `architecture/ai-inference.md` — `sidecar/app/inference/**` 작업 시
- `architecture/ai-automation.md` — `bots/**` 작업 시

이렇게 구성하면 **작업에 관련된 규칙만 컨텍스트에 로드**되어 효율이 높아집니다.

---

## 🚀 빠른 시작

```
CLAUDE.md와 .claude/rules/process/*.md를 확인한 뒤,
.claude/rules/process/roadmap.md의 Phase 0 AC-1부터 수행해줘.

PO 분석 → 내 승인 → 구현 → Reviewer 검증 순서로 진행해줘.
```

---

## 📜 변경 이력

- **2.2.0 (2026-04-24)**: 공식 `.claude/rules/` 규격 준수 + `paths` frontmatter 활용
- 2.1.0: 모듈화 버전 (자체 규격)
- 2.0.0: 자립형 통합 버전
- 1.1.0: Phase 4-B 추가
- 1.0.0: 초기 버전
