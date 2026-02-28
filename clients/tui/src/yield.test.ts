import { describe, expect, test } from "bun:test";
import {
  asString,
  asStringArray,
  confirmationOptions,
  confirmationSummary,
  structuredPrompt,
  structuredQuestions,
  structuredTitle,
} from "./yield";

describe("yield parsing helpers", () => {
  test("asString handles non-string values", () => {
    expect(asString("ok")).toBe("ok");
    expect(asString(1)).toBeNull();
    expect(asString(null)).toBeNull();
  });

  test("asStringArray keeps only strings", () => {
    expect(asStringArray(["a", 1, "b", null])).toEqual(["a", "b"]);
    expect(asStringArray("a")).toEqual([]);
  });

  test("confirmation helpers read expected fields", () => {
    const payload = {
      summary: "Approve file write?",
      options: ["approve", "reject", 1],
    };

    expect(confirmationSummary(payload)).toBe("Approve file write?");
    expect(confirmationOptions(payload)).toEqual(["approve", "reject"]);
  });

  test("structured helpers parse title/prompt and valid questions", () => {
    const payload = {
      title: "Need input",
      prompt: "Please provide answers",
      questions: [
        { id: "q1", label: "Workspace", prompt: "workspace name", required: true },
        { id: "q2", label: "Branch", prompt: "branch name" },
      ],
    };

    expect(structuredTitle(payload)).toBe("Need input");
    expect(structuredPrompt(payload)).toBe("Please provide answers");
    expect(structuredQuestions(payload)).toEqual([
      { id: "q1", label: "Workspace", prompt: "workspace name", required: true },
      { id: "q2", label: "Branch", prompt: "branch name", required: undefined },
    ]);
  });

  test("structuredQuestions drops invalid question entries", () => {
    const payload = {
      questions: [
        { id: "", label: "L", prompt: "P" },
        { id: "q", label: "", prompt: "P" },
        { id: "q2", label: "L2", prompt: "P2" },
      ],
    };

    expect(structuredQuestions(payload)).toEqual([
      { id: "q2", label: "L2", prompt: "P2", required: undefined },
    ]);
  });
});
