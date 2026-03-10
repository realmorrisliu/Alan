import { describe, expect, test } from "bun:test";
import { getAdaptiveSurface } from "./registry";
import type { PendingYield } from "./yield-state";

function pendingYield(kind: PendingYield["kind"]): PendingYield {
  return {
    requestId: "req-1",
    kind,
    payload: {},
  };
}

describe("adaptive surface registry", () => {
  test("returns the matching surface for each pending yield kind", () => {
    expect(getAdaptiveSurface(pendingYield("confirmation"))?.kind).toBe(
      "confirmation",
    );
    expect(getAdaptiveSurface(pendingYield("structured_input"))?.kind).toBe(
      "structured_input",
    );
    expect(getAdaptiveSurface(pendingYield("dynamic_tool"))?.kind).toBe(
      "dynamic_tool",
    );
    expect(getAdaptiveSurface(pendingYield("custom"))?.kind).toBe("custom");
  });

  test("returns null when there is no pending yield", () => {
    expect(getAdaptiveSurface(null)).toBeNull();
  });
});
