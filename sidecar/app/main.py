"""AI Aim Coach 사이드카 — FastAPI 진입점.

Tauri subprocess로 실행됨. 포트는 --port 인수로 지정.
PyInstaller onefile 빌드 시 _MEIPASS 경로 처리 포함.
"""

import argparse

import uvicorn
from fastapi import FastAPI

from app.api.health import router as health_router
from app.api.inference import router as inference_router

app = FastAPI(title="AI Aim Coach Sidecar", version="0.1.0")
app.include_router(health_router)
app.include_router(inference_router)


def main() -> None:
    """사이드카 진입점. --port 인수로 포트 지정 가능."""
    parser = argparse.ArgumentParser(description="AI Aim Coach Sidecar")
    parser.add_argument("--port", type=int, default=18080, help="수신 포트 (기본값: 18080)")
    args = parser.parse_args()

    uvicorn.run(app, host="127.0.0.1", port=args.port, log_level="info")


if __name__ == "__main__":
    main()
