# 작업 지침

> 작업 시작 전/중/후 체크리스트 및 금지/필수 사항.

---

## ✅ 작업 시작 전

1. **메인 CLAUDE.md 확인** (HARD LIMITS 필독)
2. **관련 `.claude/rules/` 파일 확인**
   - 작업 내용에 따라 자동 로드되지만, 의심스러우면 직접 확인
3. **기존 코드 탐색** (중복 구현 방지)
4. **PO 분석 작성** (`process/governance.md` 형식)
5. **사용자 승인 대기**

---

## 🛠 구현 중

1. **작은 커밋 단위** (한 커밋 = 하나의 논리적 변경)
2. **중간 점검** (30분마다 진행상황 요약)
3. **의존성 추가 시 사용자 확인 필수**
   - `Cargo.toml`, `package.json`, `requirements.txt` 변경
4. **배포 크기 영향 확인** (Lite 100MB 상한 체크)

---

## ✔️ 작업 완료 후

1. **자체 테스트 실행**
   - Rust: `cargo test` + `cargo clippy`
   - TS: `vitest` + `tsc --noEmit` + ESLint
   - Python: `pytest` + `ruff check` + `mypy`
2. **Reviewer 모드 전환** (`process/governance.md` 형식)
3. **APPROVE 시에만 커밋**
4. **PR 설명에 PO AC + Reviewer 체크리스트 포함**
5. **CHANGELOG.md 갱신**

---

## ❌ 절대 하지 말 것

### 프로세스 위반
- CLAUDE.md HARD LIMITS 위반 (안티치트/배포크기/UX)
- PO 승인 없이 스코프 확장
- Reviewer 검증 없이 커밋
- 테스트 작성 생략
- "일단 돌아가게" 주석 + 미완성 코드 커밋

### 기술 위반
- Razer Shot, 크래프톤, PUBG 상표를 마케팅 외 영역에 사용
- Gemini API / OpenAI API 의존성 추가 (운영 자동화는 Claude API만)
- CUDA, PyTorch, TensorFlow를 배포 번들에 포함
- Electron 사용 (Tauri만)

### 문서 위반
- 하위 규칙 파일 참조 없이 "기억대로" 결정
- CLAUDE.md에 없는 기술 도입 (반드시 사용자 승인)
- 중요 결정을 `docs/adr/` 에 기록하지 않음

---

## ✅ 반드시 할 것

### 문서화
- 모든 함수에 docstring (Python) / JSDoc (TS) / rustdoc (Rust)
- 새 HARD LIMIT 추가 시 메인 `CLAUDE.md` 반영
- 보안 관련 결정은 `docs/adr/` 에 기록

### 사용자 경험
- 에러 메시지는 사용자 친화적 한국어 + 내부 로그
- 로딩 상태 명확히 (무한 로딩 금지)

### 감사 로그
- 캡처, 외부 프로세스 실행, 설정 변경 시 `audit.rs`/`audit.py` 호출
- 로그 없이 외부 API 호출 금지

---

## 💬 좋은 태스크 요청 vs 나쁜 요청

### ✅ 좋은 요청
```
"Phase 1 AC-1을 수행해줘.
DXGI Desktop Duplication으로 PUBG Exclusive Fullscreen을 60FPS로
캡처하는 Rust 모듈을 src-tauri/src/capture/dxgi.rs에 구현.
windows 크레이트 사용, 단위 테스트 포함.
먼저 PO 분석부터 제시해줘."
```

### ❌ 나쁜 요청 (불명확)
```
"캡처 기능 만들어줘."
```

---

## 🔀 작업별 확인할 규칙 파일

파일 작업 시 경로별 규칙이 자동 로드되지만, 수동으로 확인이 필요하면:

| 작업 유형 | 자동 로드되는 규칙 |
|-----------|-------------------|
| Rust 코드 작성 | `coding/rust.md` (src-tauri/**/*.rs) |
| TypeScript/React | `coding/typescript.md` (**/*.ts, **/*.tsx) |
| Python 코드 작성 | `coding/python.md` (sidecar/**, server/**) |
| 화면 캡처 구현 | `architecture/capture.md` (src-tauri/src/capture/**) |
| AI 추론 구현 | `architecture/ai-inference.md` (sidecar/app/inference/**) |
| 운영 자동화 봇 | `architecture/ai-automation.md` (bots/**) |

**항상 로드되는 규칙** (`paths` 없음):
- `process/*.md` 전체
- `architecture/overview.md`, `security.md`, `data-model.md`, `decision-log.md`
