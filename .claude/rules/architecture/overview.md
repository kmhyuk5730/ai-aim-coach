# 시스템 아키텍처 개요

> Edge-Cloud Hybrid 구조. 비디오는 로컬, JSON만 서버로.

---

## 🏗 전체 구조

```
┌─────────────────────────────────────────────────┐
│  Client (Windows, Tauri 2)                      │
├─────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────┐   │
│  │ Rust Core (Tauri 메인)                   │   │
│  │  ├─ Capture: DXGI(1차) + WGC(2차)       │   │
│  │  ├─ 게임 모드 자동 감지                    │   │
│  │  ├─ GetRawInputData (Rust FFI)          │   │
│  │  ├─ WASAPI 오디오 (cpal)                 │   │
│  │  ├─ FFmpeg subprocess                   │   │
│  │  └─ Python sidecar 관리                 │   │
│  └─────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────┐   │
│  │ Python Sidecar (FastAPI, PyInstaller)   │   │
│  │  ├─ ONNX Runtime + DirectML EP          │   │
│  │  ├─ YOLO26n/s 추론                      │   │
│  │  └─ 수학 엔진 (핀홀 카메라)                │   │
│  └─────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────┐   │
│  │ WebView2 (React + TS + Vite + Zustand)  │   │
│  └─────────────────────────────────────────┘   │
│  ┌─────────────────────────────────────────┐   │
│  │ SQLite + .mp4 임시 폴더 (Auto-Purge)     │   │
│  └─────────────────────────────────────────┘   │
└─────────────────────────────────────────────────┘
                 ↓ JSON (경량 메타만)
┌─────────────────────────────────────────────────┐
│  Server (FastAPI on Supabase/Railway)           │
│  ├─ Auth / Session / Strategy DB                │
│  ├─ IQR 이상치 필터링                             │
│  ├─ Stripe 결제                                  │
│  └─ Discord Bot                                  │
└─────────────────────────────────────────────────┘
```

---

## 📐 데이터 흐름 원칙

1. **비디오는 서버로 업로드 금지** — 로컬만 저장, JSON 메타만 서버로
2. **AI 연산은 엣지에서 먼저** — 서버 부하 최소화
3. **서버는 통계/저장/결제만** — 실시간 분석 책임 없음
4. **민감 작업은 Rust 코어에서** — Python 사이드카는 AI 추론만

---

## 🛠 기술 스택 (2026-04-24 검증 확정)

| 영역 | 기술 | 비고 |
|------|------|------|
| 데스크탑 | **Tauri 2** | Electron 대비 15배 작음 |
| 클라이언트 코어 | **Rust** stable | 캡처/I/O |
| UI | React 18 + TS + Vite | strict mode |
| 상태관리 | Zustand | Redux는 과잉 |
| 스타일 | Tailwind + shadcn/ui | |
| Python 사이드카 | FastAPI + PyInstaller | 3.11+ |
| 서버 | FastAPI | 3.11+ |
| 캡처 1차 | **DXGI Desktop Duplication** | Exclusive Fullscreen |
| 캡처 2차 | **WGC** | Borderless/Windowed |
| 오디오 | cpal (Rust) | 또는 PyAudioWPatch |
| 마우스 입력 | GetRawInputData | windows 크레이트 |
| 인코딩 | FFmpeg essentials (custom) | 8.0.1+, ~25MB |
| AI 런타임 | ONNX Runtime + DirectML EP | 1.24+, 12MB |
| AI 모델 (무료) | **YOLO26n** | ~6MB |
| AI 모델 (유료) | **YOLO26s** | ~20MB |
| 클라이언트 DB | SQLite | 세션/감사로그 |
| 서버 DB | PostgreSQL 16+ | Supabase |
| 결제 | Stripe | |
| 인증 | Supabase Auth | |
| 커뮤니티 | Discord + discord.py | |
| 에러 추적 | Sentry | Claude 에이전트 연동 |
| 운영 자동화 | Claude API | claude-sonnet-4-6 |

---

## 📂 디렉토리 구조 (목표)

```
ai-aim-coach/
├── .github/workflows/        # CI/CD
├── client/
│   ├── src-tauri/            # Rust 코어
│   └── src/                  # React + TS UI
├── sidecar/                  # Python AI
├── server/                   # FastAPI 서버
├── bots/                     # 운영 자동화
│   ├── discord_bot/
│   └── ops_agent/
├── training/                 # AI 모델 학습 (배포 제외)
├── docs/adr/                 # 결정 기록
├── .claude/rules/            # Claude Code 규칙
└── CLAUDE.md
```

### 핵심 하위 구조

**`client/src-tauri/src/`** (Rust 코어)
```
capture/
  ├── dxgi.rs (Primary)
  ├── wgc.rs (Secondary)
  └── detector.rs (게임 모드 감지)
input/raw_input.rs
audio/wasapi.rs
display/info.rs
ffmpeg/pipe.rs
sidecar/manager.rs
storage/{sqlite.rs, purge.rs}
audit.rs
```

**`sidecar/app/`** (Python AI)
```
inference/
  ├── detector.py (YOLO26)
  └── preprocessor.py
math_engine/
  ├── pinhole.py
  └── strategy/
      ├── base.py
      └── pubg.py
models/
  ├── yolo26n.onnx
  └── yolo26s.onnx
```

---

## 📎 관련 규칙 (자동 로드)

- 캡처 상세: `capture.md` (src-tauri/src/capture/** 작업 시)
- AI 추론 상세: `ai-inference.md` (sidecar/app/inference/** 작업 시)
- 운영 자동화: `ai-automation.md` (bots/** 작업 시)
- 보안 정책: `security.md` (항상 로드)
- 데이터 모델: `data-model.md` (항상 로드)
- 기술 결정 근거: `decision-log.md` (항상 로드)
