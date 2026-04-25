"""BGR 이미지를 ONNX 추론용 입력 텐서로 변환하는 전처리 모듈."""

from typing import Final
import logging

import cv2
import numpy as np

logger = logging.getLogger(__name__)

INPUT_SIZE: Final[tuple[int, int]] = (640, 640)


def preprocess(image: np.ndarray) -> np.ndarray:
    """BGR 이미지를 ONNX 추론용 입력 텐서로 변환한다.

    Args:
        image: BGR 이미지 배열 (H, W, 3), dtype=uint8

    Returns:
        float32 텐서 (1, 3, 640, 640), 값 범위 [0.0, 1.0]

    Raises:
        ValueError: 이미지 형태가 올바르지 않을 때 (채널 수 != 3 또는 차원 != 3)
    """
    if image.ndim != 3 or image.shape[2] != 3:
        raise ValueError(
            f"Expected BGR image with shape (H, W, 3), got {image.shape}"
        )

    rgb = cv2.cvtColor(image, cv2.COLOR_BGR2RGB)
    resized = cv2.resize(rgb, INPUT_SIZE)

    # HWC → CHW, 정규화 [0, 1], 배치 차원 추가
    tensor = resized.astype(np.float32) / 255.0
    tensor = np.transpose(tensor, (2, 0, 1))   # (3, 640, 640)
    tensor = np.expand_dims(tensor, axis=0)    # (1, 3, 640, 640)

    logger.debug("전처리 완료: %s → %s", image.shape, tensor.shape)
    return tensor
