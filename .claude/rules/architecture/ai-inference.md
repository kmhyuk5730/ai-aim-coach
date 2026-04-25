---
paths:
  - "sidecar/app/inference/**/*.py"
  - "sidecar/app/math_engine/**/*.py"
  - "sidecar/app/models/**"
---

# AI 추론 전략

> YOLO26 기반 탄착점 인식 + 수학 엔진.

---

## 🎯 모델 선택: YOLO26

**2026년 1월 Ultralytics 출시**.

### 왜 YOLO26인가
- CPU 추론 **43% 빠름** (YOLOv11 대비)
- NMS-free end-to-end 추론 → 후처리 지연 제거
- FP16/INT8 양자화에서도 일관된 성능
- ONNX export가 이전 버전보다 단순

### 티어별 모델
| 티어 | 모델 | 크기 | 용도 |
|------|------|------|------|
| Free / Basic | **YOLO26n** (Nano) | ~6 MB | 무료 기본 분석 |
| Pro | **YOLO26s** (Small) | ~20 MB | 프리미엄 정밀 분석 |

---

## 🚀 런타임: ONNX Runtime + DirectML

**왜 CUDA가 아닌가**:
- DirectML: 12 MB
- CUDA + cuDNN + TensorRT: 2.6 GB
- DirectML은 **NVIDIA, AMD, Intel 모두 지원** (벤더 중립)

### 초기화 예시
```python
import onnxruntime as ort

def create_session(model_path: str) -> ort.InferenceSession:
    """DirectML EP로 ONNX 세션 생성.
    
    DirectML을 우선 시도, 실패 시 CPU로 fallback.
    """
    providers = [
        ('DmlExecutionProvider', {'device_id': 0}),
        'CPUExecutionProvider',
    ]
    session = ort.InferenceSession(
        model_path,
        providers=providers,
    )
    return session
```

---

## 🔢 수학 엔진: 핀홀 카메라 모델

### 핵심 수식
픽셀 이동량 → 물리적 회전 각도 변환:

```
θ = 2 · atan(Δpx / (2 · f))

여기서:
  θ  = 회전 각도 (라디안)
  Δpx = 픽셀 이동량
  f  = 초점 거리 = (화면_너비 / 2) / tan(FOV / 2)
```

### 구현 예시
```python
import math
from typing import Final

def pixel_to_angle(
    delta_px: float,
    screen_width: int,
    fov_degrees: float,
) -> float:
    """픽셀 이동량을 회전 각도(도)로 변환.
    
    Args:
        delta_px: 픽셀 이동량
        screen_width: 화면 너비 (픽셀)
        fov_degrees: 수평 FOV (도)
    
    Returns:
        회전 각도 (도)
    """
    fov_rad = math.radians(fov_degrees)
    focal_length = (screen_width / 2) / math.tan(fov_rad / 2)
    angle_rad = 2 * math.atan(delta_px / (2 * focal_length))
    return math.degrees(angle_rad)
```

---

## 🎯 Strategy Pattern: 게임별 엔진 상수

서버에서 `game_title`에 맞는 상수를 동적으로 받아옴.

```python
from abc import ABC, abstractmethod

class GameStrategy(ABC):
    @abstractmethod
    def get_m_yaw(self) -> float:
        """게임 엔진의 m_yaw 상수 (마우스 민감도 기본값)."""
        pass
    
    @abstractmethod
    def get_fov_scaling(self, scope_level: int) -> float:
        """배율별 FOV 스케일링 계수."""
        pass

class PubgStrategy(GameStrategy):
    def get_m_yaw(self) -> float:
        return 0.022  # 배틀그라운드 고유값
    
    def get_fov_scaling(self, scope_level: int) -> float:
        # 2x=0.5, 3x=0.33, 4x=0.25, 6x=0.167, 8x=0.125, 15x=0.067
        return 1.0 / scope_level
```

---

## 📊 탄착점 검출 정확도 목표

| 모델 | 50발 샘플 | 100발 샘플 |
|------|-----------|------------|
| YOLO26n | 오차 ~5% (MVP 목표) | ~3% |
| YOLO26s | 오차 ~3% | ~2% (프리미엄 목표) |

### 정확도 영향 요인
1. 탄착점 검출 정확도 (모델 품질)
2. 사용자 캘리브레이션 정확성 (DPI/FOV)
3. 샘플 크기 (50발 vs 100발+)
4. IQR 이상치 필터링 효과

---

## 🚨 배포 크기 HARD LIMIT

- ❌ PyTorch, TensorFlow 런타임 번들 금지
- ❌ CUDA EP 사용 금지
- ✅ ONNX Runtime + DirectML만
- ✅ 학습 코드는 `training/` 디렉토리에만 (배포 제외)

---

## 📎 관련 문서
- 전체 기술 결정: `.claude/rules/architecture/decision-log.md`
- Python 코딩 규칙: `.claude/rules/coding/python.md` (자동 로드됨)
