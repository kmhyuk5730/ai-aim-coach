# 데이터 모델

> 수동 입력(보안) / 자동 수집(인프라) / DB 스키마 분리 명세.

---

## 👤 사용자 수동 입력 (Client AppState)

보안(안티치트 적발 방지)을 위해 OS에서 강제로 읽어오지 않는 변수.

| 변수명 | 타입 | 초기값 | UI | 목적 |
|--------|------|--------|-----|------|
| `gameTitle` | String | "PUBG" | 드롭다운 | 엔진 상수 호출 식별자 |
| `dpi` | Integer | 800 | 텍스트 | 마우스 DPI (OS 역산 불가) |
| `fov` | Integer | 90 | 텍스트 | 핀홀 카메라 모델 필수값 |
| `sensGeneral` | Integer | 50 | 텍스트 | 일반 감도 |
| `sensAim` | Integer | 50 | 텍스트 | 조준(ADS) 감도 |
| `sensScope` | Integer | 50 | 텍스트 | 기본 스코프 감도 |
| `sensVertical` | Float | 1.20 | 텍스트 | Y축 배수 |
| `sensScope2x~15x` | Integer | 48, 47... | 텍스트 | 배율별 감도 6종 |
| `targetCoordinate` | String | "head"/"body" | 라디오 | Y축 타겟 |
| `captureShortcut` | String | "F10" | 키보드 | 캡처 단축키 |
| `isSoundEnabled` | Boolean | true | 토글 | 효과음 |
| `theme` | String | "dark" | 드롭다운 | 다크/라이트 |
| `resolution` | String | "default" | 드롭다운 | 창 크기 |

**자동 채움**: `GameUserSettings.ini` 파싱으로 감도 5종 자동 입력 가능 (사용자가 "자동 감지" 버튼 클릭 시).

---

## 🤖 시스템 자동 수집 (Rust Core / Python Sidecar)

OS API로 백그라운드에서 획득.

| 변수명 | 타입 | 획득 경로 | 목적 |
|--------|------|-----------|------|
| `actualResolution` | (W, H) | DXGI/WGC + 윈도우 핸들 | 실제 렌더링 해상도 |
| `refreshRate` | Float | EnumDisplaySettings | 주사율 |
| `frameDelta` | Float | 캡처 타임스탬프 | Δt = t[n]-t[n-1] |
| `rawMouseDelta` | (ΔX, ΔY) | GetRawInputData | 가속 배제 델타 |
| `gameAudio` | Binary | WASAPI | 게임 오디오만 분리 |
| `engineConstants` | Dict | Server API | m_yaw 등 게임별 상수 |
| `rotationAngleError` | Float | 내부 수학 엔진 | 핀홀 카메라 연산 |

---

## 💾 세션 데이터 (SQLite 로컬 DB)

```sql
-- 세션 테이블
sessions (
  session_id    TEXT PRIMARY KEY,
  created_at    TIMESTAMP,
  game_title    TEXT,
  dpi           INTEGER,
  fov           INTEGER,
  sens_general  INTEGER,
  sens_aim      INTEGER,
  sens_scope    INTEGER,
  sens_vertical REAL,
  sens_scopes   TEXT,  -- JSON
  status        TEXT   -- 'active' / 'closed'
)

-- 사격 데이터
shots (
  shot_id               INTEGER PRIMARY KEY,
  session_id            TEXT REFERENCES sessions,
  timestamp             TIMESTAMP,
  weapon                TEXT,
  horizontal_error_pct  REAL,
  vertical_error_pct    REAL,
  bullet_count          INTEGER,
  local_video_path      TEXT,
  in_use_flag           INTEGER
)

-- 감사 로그
audit_log (
  event_id        INTEGER PRIMARY KEY,
  timestamp       TIMESTAMP,
  event_type      TEXT,
  trigger_source  TEXT,
  details         TEXT  -- JSON
)
```

---

## ☁️ 서버 데이터 (PostgreSQL / Supabase)

```sql
-- 유저 계정
users (
  id                UUID PRIMARY KEY,
  email             TEXT UNIQUE,
  oauth_provider    TEXT,
  subscription_tier TEXT,  -- 'free' / 'basic' / 'pro'
  created_at        TIMESTAMP
)

-- 세션 메타 (영상 없음)
sessions_meta (
  session_id       UUID PRIMARY KEY,
  user_id          UUID REFERENCES users,
  aggregated_stats JSONB,
  created_at       TIMESTAMP
)

-- Strategy Pattern 엔진 상수
engine_constants (
  game_title     TEXT PRIMARY KEY,
  m_yaw          REAL,
  fov_scaling    REAL,
  other_params   JSONB,
  version        TEXT
)
```

---

## 🔐 PII 정책 요약

**서버로 보낼 때 금지**: IP 직접 저장, PUBG 닉네임, 마우스 외 PII
**허용**: 감도 수치, 오차율, 게임명, 세션 ID, 익명 통계
