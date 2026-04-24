"""헬스체크 엔드포인트 테스트."""

from fastapi.testclient import TestClient

from app.main import app

client = TestClient(app)


def test_health_returns_ok() -> None:
    """/health 엔드포인트가 {"status": "ok"}를 반환한다."""
    response = client.get("/health")
    assert response.status_code == 200
    data = response.json()
    assert data["status"] == "ok"
    assert data["service"] == "ai-aim-coach-sidecar"
