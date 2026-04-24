/**
 * 스모크 테스트 — 테스트 환경이 정상 작동하는지 확인.
 */
import { describe, it, expect } from "vitest";

describe("smoke", () => {
  it("vitest 환경이 정상 작동한다", () => {
    expect(1 + 1).toBe(2);
  });
});
