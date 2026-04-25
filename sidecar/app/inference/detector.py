"""YOLO26n ONNX Runtime 탐지 모듈.

DirectML EP(GPU) 우선, 사용 불가 시 CPU EP 폴백.
"""

from typing import Final
import logging
from pathlib import Path

import numpy as np
import onnxruntime as ort

from app.inference.preprocessor import preprocess

logger = logging.getLogger(__name__)

CONFIDENCE_THRESHOLD: Final[float] = 0.5

# YOLO26 출력 형태: (batch=1, features=84, anchors=8400)
# features = 4 (cx, cy, w, h) + 80 (class scores)
EXPECTED_OUTPUT_SHAPE: Final[tuple[int, int, int]] = (1, 84, 8400)

# (x_center, y_center, width, height, confidence) — 원본 이미지 기준 픽셀 좌표
Detection = tuple[float, float, float, float, float]


def load_session(model_path: Path) -> ort.InferenceSession:
    """ONNX 모델을 로드하여 추론 세션을 초기화한다.

    DirectML EP를 우선 시도하고, 사용 불가 시 CPU EP로 폴백한다.

    Args:
        model_path: .onnx 모델 파일 경로

    Returns:
        초기화된 InferenceSession

    Raises:
        FileNotFoundError: 모델 파일이 존재하지 않을 때
    """
    if not model_path.exists():
        raise FileNotFoundError(f"ONNX 모델을 찾을 수 없습니다: {model_path}")

    available_providers = ort.get_available_providers()
    providers: list[str] = []

    if "DmlExecutionProvider" in available_providers:
        providers.append("DmlExecutionProvider")
        logger.info("DirectML EP 사용")
    else:
        logger.info("DirectML EP 미지원 환경 → CPU EP 폴백")

    providers.append("CPUExecutionProvider")

    session = ort.InferenceSession(str(model_path), providers=providers)
    active_ep = session.get_providers()[0]
    logger.info(
        "ONNX 세션 초기화 완료 — 활성 EP: %s, 모델: %s",
        active_ep,
        model_path.name,
    )
    return session


def detect(
    image: np.ndarray,
    session: ort.InferenceSession,
    confidence_threshold: float = CONFIDENCE_THRESHOLD,
) -> list[Detection]:
    """BGR 이미지에서 탄착점을 감지한다.

    Args:
        image: BGR 이미지 (H, W, 3)
        session: 초기화된 ONNX Runtime InferenceSession
        confidence_threshold: 반환할 최소 신뢰도 (기본값: 0.5)

    Returns:
        감지된 탄착점 목록. 각 항목은 (cx, cy, w, h, confidence).
        좌표와 크기는 원본 이미지 기준 픽셀 단위.

    Raises:
        ValueError: 이미지 형태가 올바르지 않을 때
    """
    orig_h, orig_w = image.shape[:2]
    tensor = preprocess(image)

    input_name: str = session.get_inputs()[0].name
    outputs = session.run(None, {input_name: tensor})

    raw: np.ndarray = outputs[0]  # (1, 84, 8400)
    if raw.shape != EXPECTED_OUTPUT_SHAPE:
        logger.warning(
            "예상치 않은 출력 shape: %s (기대: %s)",
            raw.shape,
            EXPECTED_OUTPUT_SHAPE,
        )

    predictions: np.ndarray = raw[0].T  # (8400, 84)
    scale_x = orig_w / 640.0
    scale_y = orig_h / 640.0

    detections: list[Detection] = []
    for pred in predictions:
        class_scores: np.ndarray = pred[4:]
        confidence = float(class_scores.max())
        if confidence < confidence_threshold:
            continue

        cx = float(pred[0]) * scale_x
        cy = float(pred[1]) * scale_y
        w = float(pred[2]) * scale_x
        h = float(pred[3]) * scale_y
        detections.append((cx, cy, w, h, confidence))

    logger.debug(
        "탄착점 감지: %d개 (임계값: %.2f)", len(detections), confidence_threshold
    )
    return detections
