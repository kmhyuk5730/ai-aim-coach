"""헬스체크 라우터."""

from fastapi import APIRouter

router = APIRouter()


@router.get("/health")
async def health() -> dict[str, str]:
    """사이드카 헬스체크 엔드포인트.

    Returns:
        상태 정보 딕셔너리
    """
    return {"status": "ok", "service": "ai-aim-coach-sidecar"}
