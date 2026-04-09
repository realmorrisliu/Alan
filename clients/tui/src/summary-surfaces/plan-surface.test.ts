import { describe, expect, test } from "bun:test";
import { planItemRowKey } from "./plan-surface";

describe("plan surface helpers", () => {
  test("produces unique keys when plan item ids repeat", () => {
    const duplicateId = "step";

    expect(
      planItemRowKey(
        { id: duplicateId, content: "First", status: "pending" },
        0,
      ),
    ).toBe("step:0");
    expect(
      planItemRowKey(
        { id: duplicateId, content: "Second", status: "in_progress" },
        1,
      ),
    ).toBe("step:1");
  });
});
