import { describe, expect, test } from "bun:test";
import { createStructuredFormState } from "./structured-input";
import {
  buildSchemaDrivenYieldPayload,
  parseSchemaDrivenYieldForm,
} from "./schema-driven-yield";

describe("schema driven yield helpers", () => {
  test("parses a typed adaptive form contract from dynamic tool payloads", () => {
    const form = parseSchemaDrivenYieldForm({
      tool_name: "custom_tool",
      title: "Return tool result",
      prompt: "Provide a simple response payload",
      form: {
        fields: [
          {
            id: "success",
            label: "Success",
            prompt: "Did it work?",
            kind: "boolean",
            required: true,
            default: "true",
            options: [
              { value: "true", label: "Yes" },
              { value: "false", label: "No" },
            ],
            presentation_hints: ["toggle"],
          },
          {
            id: "result",
            label: "Result",
            prompt: "Return a result string",
            kind: "text",
          },
          {
            id: "mode",
            label: "Mode",
            prompt: "Mode",
            kind: "single_select",
            options: [
              { value: "quick", label: "Quick" },
              { value: "full", label: "Full" },
            ],
          },
        ],
      },
    });

    expect(form).toEqual({
      title: "Return tool result",
      prompt: "Provide a simple response payload",
      questions: [
        {
          id: "success",
          label: "Success",
          prompt: "Did it work?",
          kind: "boolean",
          required: true,
          defaultValue: "true",
          options: [
            { value: "true", label: "Yes" },
            { value: "false", label: "No" },
          ],
          presentationHints: ["toggle"],
        },
        {
          id: "result",
          label: "Result",
          prompt: "Return a result string",
          kind: "text",
          required: undefined,
          placeholder: undefined,
          helpText: undefined,
          defaultValue: undefined,
          defaultValues: undefined,
          minSelections: undefined,
          maxSelections: undefined,
          options: undefined,
          presentationHints: undefined,
        },
        {
          id: "mode",
          label: "Mode",
          prompt: "Mode",
          kind: "single_select",
          required: undefined,
          placeholder: undefined,
          helpText: undefined,
          defaultValue: undefined,
          defaultValues: undefined,
          minSelections: undefined,
          maxSelections: undefined,
          options: [
            { value: "quick", label: "Quick" },
            { value: "full", label: "Full" },
          ],
          presentationHints: undefined,
        },
      ],
    });
  });

  test("falls back when the form contains unsupported entries", () => {
    expect(
      parseSchemaDrivenYieldForm({
        title: "Bad form",
        form: {
          fields: [
            {
              id: "nested",
              label: "Nested",
              prompt: "Nested",
              kind: "object",
            },
          ],
        },
      }),
    ).toBeNull();
  });

  test("serializes answered values back into a payload object", () => {
    const form = parseSchemaDrivenYieldForm({
      title: "Return tool result",
      form: {
        fields: [
          {
            id: "success",
            label: "Success",
            prompt: "Did it work?",
            kind: "boolean",
            required: true,
            default: "true",
            options: [
              { value: "true", label: "Yes" },
              { value: "false", label: "No" },
            ],
          },
          {
            id: "attempts",
            label: "Attempts",
            prompt: "How many attempts?",
            kind: "integer",
            required: true,
          },
          {
            id: "note",
            label: "Note",
            prompt: "Additional note",
            kind: "text",
          },
        ],
      },
    });
    expect(form).not.toBeNull();

    const state = {
      ...createStructuredFormState("req-1", form!.questions),
      answers: {
        success: "false",
        attempts: "3",
        note: "failed after retries",
      },
    };

    expect(buildSchemaDrivenYieldPayload(state, form!)).toEqual({
      payload: {
        success: false,
        attempts: 3,
        note: "failed after retries",
      },
    });
  });

  test("returns a validation error for invalid numeric fields", () => {
    const form = parseSchemaDrivenYieldForm({
      title: "Numeric form",
      form: {
        fields: [
          {
            id: "attempts",
            label: "Attempts",
            prompt: "Attempts",
            kind: "integer",
            required: true,
          },
        ],
      },
    });
    const state = {
      ...createStructuredFormState("req-1", form!.questions),
      answers: {
        attempts: "not-a-number",
      },
    };

    expect(buildSchemaDrivenYieldPayload(state, form!)).toEqual({
      error: "Attempts: enter a valid number.",
    });
  });
});
