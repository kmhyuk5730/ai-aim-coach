# 핵심 기술 결정 근거 (Decision Log)

> 왜 이 기술을 선택했는지 기록. 상세 ADR은 `docs/adr/` 에.

---

## 🎯 기술 결정 요약

| 결정 | 대안 | 선택 이유 | 근거 |
|------|------|-----------|------|
| **Tauri 2** | Electron, Qt | 번들 크기 15배 작음, Rust 네이티브 | 2026-02 벤치마크 |
| **DXGI Primary** | WGC 단일 | PUBG Exclusive Fullscreen 지원 | Microsoft Q&A 2026-03 |
| **YOLO26n** | YOLOv11, YOLOv8 | CPU 43% 빠름, NMS-free | Ultralytics 2026-01 |
| **ONNX Runtime + DirectML** | CUDA EP | 12MB vs 2.6GB, 벤더 중립 | NuGet 1.24.4 |
| **Python 사이드카** | Rust 전용 | ONNX Python 바인딩 성숙 | - |
| **SQLite (로컬)** | JSON 파일 | 감사 로그 쿼리, 트랜잭션 | - |
| **Supabase (서버)** | 자체 구축 | 초기 비용 0, Auth 포함 | - |
| **Stripe** | PayPal, 토스 | 글로벌 확장 대비 | - |

---

## 🔍 주요 결정 상세

### 1. Tauri 2 (Electron 대신)

**벤치마크 (2026-02)**:
- 번들: Tauri 8MB vs Electron 120MB (**15배 차이**)
- 메모리: Tauri 30~50MB vs Electron 150~300MB
- 콜드 스타트: Tauri 0.3~1초 vs Electron 1~3초

**선택 이유**:
- Rust로 Windows API 직접 호출 → 캡처 성능 우위
- 기본 deny 보안 모델 → 공격 표면 축소
- 경쟁 제품 Razer Shot 대비 첫 다운로드 크기 우위

### 2. DXGI Desktop Duplication (Primary)

**왜 WGC 단일이 아닌가**:
- WGC는 Exclusive Fullscreen에서 불안정 (Microsoft Q&A 2026-03)
- PUBG 프로/준프로 층은 Exclusive Fullscreen 압도적 선호
- WGC는 HAGS + HDR 설정 요구

**해결**: DXGI 1차, WGC 2차 **Multi-Tier 전략**

### 3. YOLO26 (YOLOv11 대신)

**2026년 1월 Ultralytics 출시**:
- CPU 추론 **43% 빠름**
- NMS-free end-to-end → 후처리 지연 제거
- FP16/INT8 양자화 일관 성능

**모델 크기**: Nano ~6MB, Small ~20MB

### 4. ONNX Runtime + DirectML (CUDA 대신)

**결정적 이유**:
- DirectML 패키지 **12 MB**
- CUDA + cuDNN + TensorRT 조합 시 **2.6 GB**
- GPU 벤더 중립 (NVIDIA, AMD, Intel)
- Windows 10 1903+ 전체 지원

---

## 📏 배포 크기 상세 (2026-04-24 검증)

### Lite 구성 (무료 배포)

| 구성 요소 | 크기 |
|-----------|------|
| Tauri + React UI | 8~12 MB |
| FFmpeg essentials | 20~25 MB |
| ONNX Runtime DirectML | 12 MB |
| YOLO26n 모델 | 6 MB |
| Python 사이드카 (최소) | 20~30 MB |
| 기타 | 3~5 MB |
| **합계** | **~70~90 MB** |

### Standard 구성 (프리미엄)

| 추가 요소 | 추가 크기 |
|-----------|-----------|
| Lite 구성 | 90 MB |
| YOLO26s 모델 | +20 MB |
| 추가 UI 에셋 | +10 MB |
| 감사 로그 DB | +5 MB |
| 추가 언어 | +5 MB |
| **합계** | **~130 MB** |

### Pro 구성 (향후 멀티 게임)

| 추가 요소 | 추가 크기 |
|-----------|-----------|
| Standard 구성 | 130 MB |
| 게임별 엔진 상수 DB | +10 MB |
| 발로란트/에이펙스 모델 | +12 MB |
| 음성 피드백 (TTS) | +30 MB |
| 예비 | +20 MB |
| **합계** | **~200~250 MB** |

---

## 🚨 만약 잘못된 선택을 했다면

**Electron + PyInstaller + CUDA 조합 시**:
- Electron 기본: 120 MB
- PyInstaller (PyTorch/TF): 1,500 MB
- CUDA + cuDNN: 2,600 MB
- **합계: 4 GB+** ← 프로젝트 사망

**현재 선택 (Tauri + DirectML)**:
- 약 70~90 MB
- **50배 이상 작음**

이 선택이 **프로젝트 생존을 결정**.

---

## 🔄 결정 재검토 트리거

다음 상황에서 이 문서 재검토:

- Windows 11 24H2+ 점유율 50% 초과 → WinML 전환 검토
- YOLO27 또는 후속 버전 출시 → 벤치마크 재실행
- Tauri 3.0 출시 → 마이그레이션 비용 평가
- Razer Shot이 기능/가격 전략 크게 변경 → 포지셔닝 재검토
