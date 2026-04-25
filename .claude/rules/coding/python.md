---
paths:
  - "sidecar/**/*.py"
  - "server/**/*.py"
  - "bots/**/*.py"
  - "training/**/*.py"
  - "**/pyproject.toml"
  - "**/requirements.txt"
---

# Python 코딩 규칙

> 사이드카 + 서버 + 봇 전용.

---

## 🎯 기본 설정

- **Python 3.11+**
- **타입 힌트 필수**
- **Ruff + mypy(strict)** 통과 의무
- **비동기 우선** (FastAPI, asyncio)
- **로깅**: `logging.getLogger(__name__)`

---

## ❌ 금지 사항

- 광범위한 `except Exception:` (구체적 예외만)
- `print()` (로깅은 `logging`)
- 매직 넘버 (상수는 `Final` + 분리)
- 전역 변수 (Pydantic Settings 사용)
- 타입 힌트 없는 함수 시그니처

---

## ✅ 좋은 예

```python
from typing import Final
import logging
import numpy as np
import onnxruntime as ort

logger = logging.getLogger(__name__)

INPUT_SIZE: Final[tuple[int, int]] = (640, 640)
CONFIDENCE_THRESHOLD: Final[float] = 0.5

def detect_bullet_holes(
    image: np.ndarray,
    session: ort.InferenceSession,
) -> list[tuple[float, float, float]]:
    """탄착점 감지하여 (x, y, confidence) 리스트 반환.

    Raises:
        ValueError: 이미지 형태가 올바르지 않을 때
    """
    if image.ndim != 3 or image.shape[2] != 3:
        raise ValueError(f"Expected BGR image, got shape {image.shape}")
    # ... 추론 로직
    logger.debug("Detected %d bullet holes", len(detections))
    return detections
```

---

## 🚨 배포 크기 HARD LIMIT

사이드카는 PyInstaller로 번들되므로 **의존성 최소화 필수**:

### ✅ 허용
- FastAPI, Pydantic, uvicorn
- ONNX Runtime + DirectML EP
- NumPy, OpenCV (cv2)
- Pillow (이미지 처리)

### ❌ 금지
- **PyTorch, TensorFlow** (훈련은 `training/` 환경에만)
- **scikit-learn** (사이드카 배포용 아님)
- **CUDA 관련 패키지** 일체
- pandas (사이드카에 필요 없음, 서버만 OK)

### 대안
- ML 모델: ONNX 형식으로 export 후 ONNX Runtime으로 추론
- 수치 계산: NumPy로 대부분 가능
- 이미지 전처리: OpenCV 또는 Pillow

---

## 🧪 테스트

### pytest + pytest-asyncio
```python
import pytest
import numpy as np
from sidecar.app.inference.detector import detect_bullet_holes

def test_detect_bullet_holes_empty_image_raises_valueerror(session):
    with pytest.raises(ValueError, match="Expected BGR image"):
        detect_bullet_holes(np.zeros((0, 0)), session)

@pytest.mark.asyncio
async def test_api_analyze_returns_result(async_client):
    response = await async_client.post("/analyze", json={...})
    assert response.status_code == 200
```

### 필수 실행
```bash
pytest              # 단위 테스트
ruff check .        # 린트
mypy --strict .     # 타입 체크
```

---

## ⚙️ FastAPI 구조

```python
from fastapi import FastAPI, HTTPException, Depends
from pydantic import BaseModel

class AnalyzeRequest(BaseModel):
    frame_path: str
    game_title: str

class AnalyzeResponse(BaseModel):
    bullet_holes: list[tuple[float, float, float]]
    confidence: float

@app.post("/analyze", response_model=AnalyzeResponse)
async def analyze_frame(
    request: AnalyzeRequest,
    session: ort.InferenceSession = Depends(get_session),
) -> AnalyzeResponse:
    """프레임에서 탄착점 분석."""
    try:
        image = load_image(request.frame_path)
    except FileNotFoundError as e:
        raise HTTPException(status_code=404, detail=str(e))
    holes = detect_bullet_holes(image, session)
    return AnalyzeResponse(bullet_holes=holes, confidence=0.95)
```

---

## 📦 의존성 관리

### requirements.txt 작성 규칙
- 버전 고정: `fastapi==0.110.0` (>= 금지)
- 주석으로 용도 명시
- 훈련용과 배포용 분리

```
# 사이드카 배포용 (최소화)
fastapi==0.110.0
uvicorn==0.27.0
pydantic==2.6.0
onnxruntime-directml==1.24.4
numpy==1.26.0
opencv-python-headless==4.9.0.80
pillow==10.2.0

# 개발 전용 (배포 제외)
pytest==8.0.0
pytest-asyncio==0.23.0
ruff==0.2.0
mypy==1.8.0
```

### 훈련용은 별도
```
# training/requirements.txt
ultralytics==8.3.0  # YOLO 학습용
torch==2.2.0         # 학습 시에만 필요
```

---

## 📝 감사 로그

사이드카에서도 캡처 관련 작업 시 감사 로그 기록:

```python
from sidecar.app.audit import log_event, EventType

async def analyze_frame(request: AnalyzeRequest) -> AnalyzeResponse:
    await log_event(EventType.ANALYSIS_STARTED, details={"game": request.game_title})
    try:
        result = await _do_analysis(request)
        await log_event(EventType.ANALYSIS_COMPLETED)
        return result
    except Exception as e:
        logger.exception("Analysis failed")
        await log_event(EventType.ANALYSIS_FAILED, details={"error": str(e)})
        raise
```
