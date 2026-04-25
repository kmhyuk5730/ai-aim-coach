---
paths:
  - "client/src/**/*.ts"
  - "client/src/**/*.tsx"
  - "client/package.json"
  - "client/tsconfig.json"
---

# TypeScript / React 코딩 규칙

> Tauri 2 WebView2 UI 전용.

---

## 🎯 기본 설정

- **Strict mode 필수**:
  - `strict: true`
  - `noUncheckedIndexedAccess: true`
  - `noImplicitAny: true`
- **any 금지** (불가피 시 eslint-disable + 사유 주석)
- **함수형 컴포넌트 + Hooks**
- **기본 export 사용**

---

## 🏗 기술 스택

- React 18 + TypeScript + Vite
- **상태 관리**: Zustand (Redux 금지, 과잉)
- **스타일**: Tailwind CSS + shadcn/ui
- **라우팅**: React Router v6
- **테스트**: Vitest + React Testing Library

---

## ❌ 금지 사항

### any 사용 금지
```typescript
// ❌ 나쁜 예
const data: any = await invoke("get_session");
```

```typescript
// ✅ 좋은 예
interface Session {
  id: string;
  createdAt: string;
  status: 'active' | 'closed';
}

const session: Session = await invoke<Session>("get_session");
```

### 인라인 스타일 금지
```tsx
// ❌ 나쁜 예
<div style={{ color: 'red', padding: '10px' }}>...</div>

// ✅ 좋은 예 (Tailwind)
<div className="text-red-500 p-2.5">...</div>
```

### Redux 금지
```typescript
// ❌ Redux 도입 금지

// ✅ Zustand로 스토어 생성
import { create } from 'zustand';

interface SessionStore {
  session: Session | null;
  setSession: (s: Session) => void;
}

export const useSessionStore = create<SessionStore>((set) => ({
  session: null,
  setSession: (session) => set({ session }),
}));
```

---

## 🔌 Tauri 호출

Tauri 명령 호출 시 **타입 명시 필수**:

```typescript
import { invoke } from '@tauri-apps/api/core';

// 타입 정의
interface CaptureResult {
  success: boolean;
  frameCount: number;
  errorMessage?: string;
}

// 호출
async function startCapture(mode: GameMode): Promise<CaptureResult> {
  return await invoke<CaptureResult>('start_capture', { mode });
}
```

---

## 📝 컴포넌트 구조

```tsx
import { useState, useEffect } from 'react';
import { useSessionStore } from '@/stores/session';

interface QuickAnalysisProps {
  sessionId: string;
  onComplete?: (result: AnalysisResult) => void;
}

/**
 * 빠른 분석 탭 컴포넌트.
 * 
 * 50발 누적 시 감도 교정 수치를 제안합니다.
 */
export default function QuickAnalysis({ 
  sessionId, 
  onComplete 
}: QuickAnalysisProps) {
  const [loading, setLoading] = useState(false);
  const session = useSessionStore((s) => s.session);
  
  // ...
  
  return (
    <div className="p-4">
      {/* ... */}
    </div>
  );
}
```

---

## 🧪 테스트

### Vitest + React Testing Library
```typescript
import { describe, it, expect } from 'vitest';
import { render, screen } from '@testing-library/react';
import QuickAnalysis from './QuickAnalysis';

describe('QuickAnalysis', () => {
  it('renders empty state when no sessions exist', () => {
    render(<QuickAnalysis sessionId="" />);
    expect(screen.getByText(/세션이 없습니다/)).toBeInTheDocument();
  });
});
```

### 필수 실행
```bash
pnpm vitest          # 단위 테스트
pnpm tsc --noEmit    # 타입 체크
pnpm eslint .        # 린트
```

---

## 📦 의존성 추가 규칙

새 패키지 추가 시 반드시 사용자 승인:
- 패키지명 + 버전
- 용도
- 번들 크기 영향 (bundlephobia 확인)
- 라이선스
- 주간 다운로드 (npm)

---

## 🎨 파일 명명

- 컴포넌트: `PascalCase.tsx` (예: `QuickAnalysis.tsx`)
- Hooks: `camelCase.ts` 접두사 `use` (예: `useSession.ts`)
- 유틸: `camelCase.ts` (예: `formatDate.ts`)
- 타입: 별도 파일 시 `types.ts`

---

## 🌐 i18n (Phase 4+)

Phase 4에서 영어/일본어/중국어 추가 예정. 지금은 한국어 하드코딩 OK.

나중을 위해 문자열을 상수로 분리:
```typescript
// 좋은 예
const MESSAGES = {
  EMPTY_SESSION: '세션이 없습니다',
  LOADING: '분석 중...',
} as const;
```
