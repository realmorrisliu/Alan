import { describe, expect, test } from "bun:test";
import {
  asString,
  asStringArray,
  confirmationOptions,
  confirmationSummary,
  normalizeYieldKind,
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

  test("normalizeYieldKind supports protocol and custom kinds", () => {
    expect(normalizeYieldKind("confirmation")).toBe("confirmation");
    expect(normalizeYieldKind("structured_input")).toBe("structured_input");
    expect(normalizeYieldKind("dynamic_tool")).toBe("dynamic_tool");
    expect(normalizeYieldKind("other_kind")).toBe("custom");
    expect(normalizeYieldKind({ custom: "handoff" })).toBe("custom");
    expect(normalizeYieldKind(undefined)).toBeNull();
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
        {
          id: "q1",
          label: "Workspace",
          prompt: "workspace name",
          required: true,
          options: [
            { value: "dev", label: "Development" },
            { value: "prod", label: "Production", description: "Use with care" },
          ],
        },
        { id: "q2", label: "Branch", prompt: "branch name" },
      ],
    };

    expect(structuredTitle(payload)).toBe("Need input");
    expect(structuredPrompt(payload)).toBe("Please provide answers");
    expect(structuredQuestions(payload)).toEqual([
      {
        id: "q1",
        label: "Workspace",
        prompt: "workspace name",
        required: true,
        options: [
          { value: "dev", label: "Development" },
          { value: "prod", label: "Production", description: "Use with care" },
        ],
      },
      {
        id: "q2",
        label: "Branch",
        prompt: "branch name",
        required: undefined,
        options: undefined,
      },
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
      { id: "q2", label: "L2", prompt: "P2", required: undefined, options: undefined },
    ]);
  });

  test("structuredQuestions filters invalid options", () => {
    const payload = {
      questions: [
        {
          id: "q1",
          label: "Choose",
          prompt: "Pick one",
          options: [
            { value: "a", label: "A" },
            { value: "", label: "Nope" },
            { value: "b", label: "B", description: 1 },
          ],
        },
      ],
    };

    expect(structuredQuestions(payload)).toEqual([
      {
        id: "q1",
        label: "Choose",
        prompt: "Pick one",
        required: undefined,
        options: [
          { value: "a", label: "A" },
          { value: "b", label: "B" },
        ],
      },
    ]);
  });
});
