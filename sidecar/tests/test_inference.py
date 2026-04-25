"""ONNX 추론 모듈 단위 테스트.

CI 안전 설계 — 실제 ONNX 모델 파일 또는 GPU 없이 실행 가능.
ort.InferenceSession은 unittest.mock으로 대체.
"""

from __future__ import annotations

import tempfile
from pathlib import Path
from typing import Any
from unittest.mock import MagicMock, patch

import cv2
import numpy as np
import pytest

from app.inference.preprocessor import preprocess


# ─── 전처리 테스트 ────────────────────────────────────────────────────────────


def test_preprocess_returns_correct_shape() -> None:
    """preprocess()가 (1, 3, 640, 640) float32 텐서를 반환한다."""
    image = np.zeros((480, 640, 3), dtype=np.uint8)
    tensor = preprocess(image)
    assert tensor.shape == (1, 3, 640, 640)
    assert tensor.dtype == np.float32


def test_preprocess_normalizes_pixel_255_to_1() -> None:
    """픽셀 값 255가 1.0으로 정규화된다."""
    image = np.full((100, 100, 3), 255, dtype=np.uint8)
    tensor = preprocess(image)
    assert float(tensor.max()) == pytest.approx(1.0)
    assert float(tensor.min()) == pytest.approx(1.0)


def test_preprocess_normalizes_pixel_0_to_0() -> None:
    """픽셀 값 0이 0.0으로 정규화된다."""
    image = np.zeros((100, 100, 3), dtype=np.uint8)
    tensor = preprocess(image)
    assert float(tensor.max()) == pytest.approx(0.0)


def test_preprocess_invalid_2d_image_raises_valueerror() -> None:
    """2차원 배열을 입력하면 ValueError가 발생한다."""
    image = np.zeros((480, 640), dtype=np.uint8)
    with pytest.raises(ValueError, match="Expected BGR image"):
        preprocess(image)


def test_preprocess_invalid_4_channel_raises_valueerror() -> None:
    """4채널 이미지를 입력하면 ValueError가 발생한다."""
    image = np.zeros((480, 640, 4), dtype=np.uint8)
    with pytest.raises(ValueError, match="Expected BGR image"):
        preprocess(image)


# ─── 탐지 테스트 ─────────────────────────────────────────────────────────────


def _make_mock_session(output: np.ndarray) -> Any:
    """더미 ort.InferenceSession 목 객체를 생성한다."""
    mock_input: Any = MagicMock()
    mock_input.name = "images"
    session: Any = MagicMock()
    session.get_inputs.return_value = [mock_input]
    session.run.return_value = [output]
    return session


def test_detect_returns_empty_when_all_confidence_below_threshold() -> None:
    """모든 예측의 신뢰도가 임계값 미만이면 빈 목록을 반환한다."""
    from app.inference.detector import detect

    output = np.zeros((1, 84, 8400), dtype=np.float32)
    session = _make_mock_session(output)
    image = np.zeros((640, 640, 3), dtype=np.uint8)

    detections = detect(image, session, confidence_threshold=0.5)
    assert detections == []


def test_detect_returns_detection_when_confidence_above_threshold() -> None:
    """신뢰도 0.9인 탄착점 1개가 감지되어 반환된다."""
    from app.inference.detector import detect

    output = np.zeros((1, 84, 8400), dtype=np.float32)
    # 앵커 0: cx=320, cy=320, w=10, h=10, class_0=0.9
    output[0, 0, 0] = 320.0
    output[0, 1, 0] = 320.0
    output[0, 2, 0] = 10.0
    output[0, 3, 0] = 10.0
    output[0, 4, 0] = 0.9

    session = _make_mock_session(output)
    image = np.zeros((640, 640, 3), dtype=np.uint8)

    detections = detect(image, session, confidence_threshold=0.5)
    assert len(detections) == 1
    _cx, _cy, _w, _h, conf = detections[0]
    assert conf == pytest.approx(0.9)


def test_detect_scales_coordinates_to_original_resolution() -> None:
    """640x640 기준 좌표가 원본 해상도로 정확히 변환된다."""
    from app.inference.detector import detect

    output = np.zeros((1, 84, 8400), dtype=np.float32)
    output[0, 0, 0] = 320.0  # cx in 640-space
    output[0, 1, 0] = 240.0  # cy in 640-space
    output[0, 4, 0] = 0.9

    session = _make_mock_session(output)
    # 원본 이미지: 1280x960
    image = np.zeros((960, 1280, 3), dtype=np.uint8)

    detections = detect(image, session, confidence_threshold=0.5)
    assert len(detections) == 1
    cx, cy, _w, _h, _conf = detections[0]
    # 320 / 640 * 1280 = 640.0, 240 / 640 * 960 = 360.0
    assert cx == pytest.approx(640.0)
    assert cy == pytest.approx(360.0)


def test_detect_filters_multiple_predictions_by_threshold() -> None:
    """임계값 이상인 탄착점만 반환된다."""
    from app.inference.detector import detect

    output = np.zeros((1, 84, 8400), dtype=np.float32)
    output[0, 4, 0] = 0.9   # 앵커 0: 통과
    output[0, 4, 1] = 0.3   # 앵커 1: 걸림 (0.3 < 0.5)
    output[0, 4, 2] = 0.8   # 앵커 2: 통과

    session = _make_mock_session(output)
    image = np.zeros((640, 640, 3), dtype=np.uint8)

    detections = detect(image, session, confidence_threshold=0.5)
    assert len(detections) == 2


# ─── load_session 테스트 ──────────────────────────────────────────────────────


def test_load_session_raises_file_not_found_for_missing_model() -> None:
    """모델 파일이 없으면 FileNotFoundError가 발생한다."""
    from app.inference.detector import load_session

    with pytest.raises(FileNotFoundError, match="ONNX 모델을 찾을 수 없습니다"):
        load_session(Path("/nonexistent/yolo26n.onnx"))


def test_load_session_uses_cpu_ep_when_dml_unavailable(tmp_path: Path) -> None:
    """DirectML EP가 없을 때 CPU EP로 세션을 초기화한다."""
    from app.inference.detector import load_session

    dummy_model = tmp_path / "dummy.onnx"
    dummy_model.write_bytes(b"")

    mock_session: Any = MagicMock()
    mock_session.get_providers.return_value = ["CPUExecutionProvider"]

    with (
        patch(
            "app.inference.detector.ort.get_available_providers",
            return_value=["CPUExecutionProvider"],
        ),
        patch(
            "app.inference.detector.ort.InferenceSession",
            return_value=mock_session,
        ),
    ):
        session = load_session(dummy_model)
        assert session is mock_session


def test_load_session_prefers_dml_ep_when_available(tmp_path: Path) -> None:
    """DirectML EP가 있으면 providers 목록 첫 번째로 전달된다."""
    from app.inference.detector import load_session

    dummy_model = tmp_path / "dummy.onnx"
    dummy_model.write_bytes(b"")

    mock_session: Any = MagicMock()
    mock_session.get_providers.return_value = ["DmlExecutionProvider"]
    captured_providers: list[list[str]] = []

    def fake_session(path: str, providers: list[str]) -> Any:
        captured_providers.append(providers)
        return mock_session

    with (
        patch(
            "app.inference.detector.ort.get_available_providers",
            return_value=["DmlExecutionProvider", "CPUExecutionProvider"],
        ),
        patch("app.inference.detector.ort.InferenceSession", side_effect=fake_session),
    ):
        load_session(dummy_model)
        assert captured_providers[0][0] == "DmlExecutionProvider"


# ─── API 엔드포인트 테스트 ──────────────────────────────────────────────────


def test_detect_endpoint_returns_200_with_mock_session() -> None:
    """mock 세션으로 /inference/detect 엔드포인트가 200을 반환한다."""
    import tempfile

    from fastapi.testclient import TestClient

    from app.api.inference import get_session
    from app.main import app

    output = np.zeros((1, 84, 8400), dtype=np.float32)
    mock_session = _make_mock_session(output)
    app.dependency_overrides[get_session] = lambda: mock_session

    with tempfile.NamedTemporaryFile(suffix=".png", delete=False) as f:
        tmp_img = f.name
    cv2.imwrite(tmp_img, np.zeros((100, 100, 3), dtype=np.uint8))

    try:
        client = TestClient(app)
        response = client.post("/inference/detect", json={"frame_path": tmp_img})
        assert response.status_code == 200
        data = response.json()
        assert "detections" in data
        assert data["model"] == "yolo26n"
        assert isinstance(data["detections"], list)
    finally:
        app.dependency_overrides.clear()
        Path(tmp_img).unlink(missing_ok=True)


def test_detect_endpoint_returns_404_for_missing_frame() -> None:
    """존재하지 않는 프레임 경로로 요청 시 404를 반환한다."""
    from fastapi.testclient import TestClient

    from app.api.inference import get_session
    from app.main import app

    output = np.zeros((1, 84, 8400), dtype=np.float32)
    mock_session = _make_mock_session(output)
    app.dependency_overrides[get_session] = lambda: mock_session

    try:
        client = TestClient(app)
        response = client.post(
            "/inference/detect",
            json={"frame_path": "/nonexistent/frame.png"},
        )
        assert response.status_code == 404
    finally:
        app.dependency_overrides.clear()


def test_detect_endpoint_returns_503_when_model_missing() -> None:
    """ONNX 모델이 없을 때 /inference/detect가 503을 반환한다."""
    from fastapi.testclient import TestClient

    from app.main import app

    # lru_cache를 우회하여 항상 FileNotFoundError 발생
    with patch(
        "app.api.inference._load_cached_session",
        side_effect=FileNotFoundError("ONNX 모델을 찾을 수 없습니다: /models/yolo26n.onnx"),
    ):
        client = TestClient(app)
        response = client.post(
            "/inference/detect",
            json={"frame_path": "/some/frame.png"},
        )
        assert response.status_code == 503
