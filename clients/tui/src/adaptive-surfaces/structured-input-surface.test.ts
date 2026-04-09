import { describe, expect, test } from "bun:test";
import {
  structuredQuestionControls,
  structuredQuestionHints,
  structuredQuestionToggleSummary,
} from "./structured-input-surface";
import { createStructuredFormState } from "../structured-input";

describe("structured input surface helpers", () => {
  test("uses toggle-specific controls and summary when requested", () => {
    const question = {
      id: "success",
      label: "Success",
      prompt: "Did it work?",
      kind: "boolean" as const,
      options: [
        { value: "true", label: "Yes" },
        { value: "false", label: "No" },
      ],
      defaultValue: "true",
      presentationHints: ["toggle" as const],
    };
    const state = createStructuredFormState("req-1", [question]);

    expect(structuredQuestionControls(question)).toContain("toggle");
    expect(structuredQuestionToggleSummary(question, state)).toBe("[Yes] / No");
  });

  test("adds multiline guidance for long-text inputs", () => {
    const question = {
      id: "notes",
      label: "Notes",
      prompt: "Additional notes",
      kind: "text" as const,
      presentationHints: ["multiline" as const],
    };

    expect(structuredQuestionHints(question)).toEqual([
      {
        color: "gray",
        text: "Long text hint: use /answer or /answers with escaped newlines for multi-line input.",
      },
    ]);
  });

  test("adds numeric and dangerous guidance when relevant", () => {
    const question = {
      id: "retries",
      label: "Retries",
      prompt: "Retry count",
      kind: "integer" as const,
      presentationHints: ["dangerous" as const],
    };

    expect(structuredQuestionHints(question)).toEqual([
      {
        color: "gray",
        text: "Numeric input: whole numbers only.",
      },
      {
        color: "red",
        text: "Dangerous input: review this value carefully before submitting.",
      },
    ]);
  });
});
