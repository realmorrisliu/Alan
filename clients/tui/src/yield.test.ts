import { describe, expect, test } from "bun:test";
import {
  confirmationActionOptions,
  confirmationDetails,
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
      details: { path: "/tmp/example.txt" },
      options: ["approve", "reject", 1],
    };

    expect(confirmationSummary(payload)).toBe("Approve file write?");
    expect(confirmationOptions(payload)).toEqual(["approve", "reject"]);
    expect(confirmationActionOptions(payload)).toEqual(["approve", "reject"]);
    expect(confirmationDetails(payload)).toEqual({
      path: "/tmp/example.txt",
    });
  });

  test("confirmation helpers provide default actions when options are absent", () => {
    expect(confirmationActionOptions({ summary: "Need approval" })).toEqual([
      "approve",
      "modify",
      "reject",
    ]);
    expect(confirmationDetails({ summary: "Need approval" })).toBeNull();
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
          kind: "single_select",
          required: true,
          help_text: "Pick one workspace.",
          default: "dev",
          options: [
            { value: "dev", label: "Development" },
            {
              value: "prod",
              label: "Production",
              description: "Use with care",
            },
          ],
        },
        {
          id: "q2",
          label: "Branch",
          prompt: "branch name",
          placeholder: "feature/adaptive-yield-ui",
        },
      ],
    };

    expect(structuredTitle(payload)).toBe("Need input");
    expect(structuredPrompt(payload)).toBe("Please provide answers");
    expect(structuredQuestions(payload)).toEqual([
      {
        id: "q1",
        label: "Workspace",
        prompt: "workspace name",
        kind: "single_select",
        required: true,
        helpText: "Pick one workspace.",
        defaultValue: "dev",
        defaultValues: undefined,
        minSelections: undefined,
        maxSelections: undefined,
        options: [
          { value: "dev", label: "Development" },
          { value: "prod", label: "Production", description: "Use with care" },
        ],
      },
      {
        id: "q2",
        label: "Branch",
        prompt: "branch name",
        kind: "text",
        required: undefined,
        placeholder: "feature/adaptive-yield-ui",
        helpText: undefined,
        defaultValue: undefined,
        defaultValues: undefined,
        minSelections: undefined,
        maxSelections: undefined,
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
      {
        id: "q2",
        label: "L2",
        prompt: "P2",
        kind: "text",
        required: undefined,
        placeholder: undefined,
        helpText: undefined,
        defaultValue: undefined,
        defaultValues: undefined,
        minSelections: undefined,
        maxSelections: undefined,
        options: undefined,
      },
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
        kind: "single_select",
        required: undefined,
        placeholder: undefined,
        helpText: undefined,
        defaultValue: undefined,
        defaultValues: undefined,
        minSelections: undefined,
        maxSelections: undefined,
        options: [
          { value: "a", label: "A" },
          { value: "b", label: "B" },
        ],
      },
    ]);
  });

  test("structuredQuestions supports multi-select defaults and constraints", () => {
    const payload = {
      questions: [
        {
          id: "q1",
          label: "Targets",
          prompt: "Pick deploy targets",
          kind: "multi_select",
          defaults: ["staging"],
          min_selected: 1,
          max_selected: 2,
          options: [
            { value: "staging", label: "Staging" },
            { value: "prod", label: "Production" },
          ],
        },
      ],
    };

    expect(structuredQuestions(payload)).toEqual([
      {
        id: "q1",
        label: "Targets",
        prompt: "Pick deploy targets",
        kind: "multi_select",
        required: undefined,
        placeholder: undefined,
        helpText: undefined,
        defaultValue: undefined,
        defaultValues: ["staging"],
        minSelections: 1,
        maxSelections: 2,
        options: [
          { value: "staging", label: "Staging" },
          { value: "prod", label: "Production" },
        ],
      },
    ]);
  });

  test("structuredQuestions rejects explicit select questions without options", () => {
    const payload = {
      questions: [
        {
          id: "q1",
          label: "Provider",
          prompt: "Pick a provider",
          kind: "single_select",
        },
      ],
    };

    expect(structuredQuestions(payload)).toEqual([]);
  });
});
