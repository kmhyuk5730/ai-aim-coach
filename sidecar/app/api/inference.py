"""추론 라우터 — /inference 엔드포인트."""

import logging
import sys
from functools import lru_cache
from pathlib import Path

import cv2
import numpy as np
import onnxruntime as ort
from fastapi import APIRouter, Depends, HTTPException
from pydantic import BaseModel

from app.inference.detector import Detection, detect, load_session

logger = logging.getLogger(__name__)

router = APIRouter(prefix="/inference", tags=["inference"])


def _model_path() -> Path:
    """개발/PyInstaller 환경에서 YOLO26n 모델 경로를 반환한다."""
    if hasattr(sys, "_MEIPASS"):
        return Path(sys._MEIPASS) / "models" / "yolo26n.onnx"  # type: ignore[attr-defined]
    return Path(__file__).parent.parent.parent / "models" / "yolo26n.onnx"


@lru_cache(maxsize=None)
def _load_cached_session() -> ort.InferenceSession:
    """ONNX 세션을 한 번만 로드하고 캐시한다."""
    return load_session(_model_path())


def get_session() -> ort.InferenceSession:
    """FastAPI 의존성 — ONNX 세션을 반환한다. 모델 없으면 503."""
    try:
        return _load_cached_session()
    except FileNotFoundError as exc:
        raise HTTPException(status_code=503, detail=str(exc)) from exc


class DetectRequest(BaseModel):
    frame_path: str


class DetectionItem(BaseModel):
    x_center: float
    y_center: float
    width: float
    height: float
    confidence: float


class DetectResponse(BaseModel):
    detections: list[DetectionItem]
    model: str = "yolo26n"


@router.post("/detect", response_model=DetectResponse)
async def detect_frame(
    request: DetectRequest,
    session: ort.InferenceSession = Depends(get_session),
) -> DetectResponse:
    """프레임에서 탄착점을 감지한다.

    Args:
        request: 분석할 프레임 파일 경로
        session: ONNX 추론 세션 (의존성 주입)

    Returns:
        감지된 탄착점 목록

    Raises:
        HTTPException 404: 프레임 파일을 읽을 수 없을 때
        HTTPException 422: 이미지 형태가 올바르지 않을 때
        HTTPException 503: ONNX 모델이 로드되지 않았을 때
    """
    image: np.ndarray = cv2.imread(request.frame_path)
    if image is None:
        raise HTTPException(
            status_code=404,
            detail=f"이미지를 읽을 수 없습니다: {request.frame_path}",
        )

    try:
        raw_detections: list[Detection] = detect(image, session)
    except ValueError as exc:
        raise HTTPException(status_code=422, detail=str(exc)) from exc

    items = [
        DetectionItem(x_center=cx, y_center=cy, width=w, height=h, confidence=conf)
        for cx, cy, w, h, conf in raw_detections
    ]
    return DetectResponse(detections=items)
