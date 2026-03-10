import { describe, expect, test } from "bun:test";
import { createStructuredFormState } from "./structured-input";
import {
  buildSchemaDrivenYieldPayload,
  parseSchemaDrivenYieldForm,
} from "./schema-driven-yield";

describe("schema driven yield helpers", () => {
  test("parses a flat object schema into structured questions", () => {
    const form = parseSchemaDrivenYieldForm({
      title: "Return tool result",
      prompt: "Provide a simple response payload",
      resume_schema: {
        type: "object",
        required: ["success", "result"],
        properties: {
          success: {
            type: "boolean",
            title: "Success",
            default: true,
          },
          result: {
            type: "string",
            title: "Result",
            description: "Return a result string",
          },
          mode: {
            enum: ["quick", "full"],
            title: "Mode",
          },
        },
      },
    });

    expect(form).toEqual({
      title: "Return tool result",
      prompt: "Provide a simple response payload",
      questions: [
        {
          id: "success",
          label: "Success",
          prompt: "Success",
          kind: "single_select",
          required: true,
          defaultValue: "true",
          options: [
            { value: "true", label: "Yes" },
            { value: "false", label: "No" },
          ],
        },
        {
          id: "result",
          label: "Result",
          prompt: "Return a result string",
          kind: "text",
          required: true,
          placeholder: undefined,
          defaultValue: undefined,
          helpText: undefined,
        },
        {
          id: "mode",
          label: "Mode",
          prompt: "Mode",
          kind: "single_select",
          required: false,
          defaultValue: undefined,
          options: [
            { value: "quick", label: "Quick" },
            { value: "full", label: "Full" },
          ],
        },
      ],
      fields: expect.any(Array),
    });
  });

  test("falls back when the schema contains unsupported nested objects", () => {
    expect(
      parseSchemaDrivenYieldForm({
        resume_schema: {
          type: "object",
          properties: {
            nested: {
              type: "object",
              properties: {
                key: { type: "string" },
              },
            },
          },
        },
      }),
    ).toBeNull();
  });

  test("serializes answered values back into a payload object", () => {
    const form = parseSchemaDrivenYieldForm({
      resume_schema: {
        type: "object",
        required: ["success", "attempts"],
        properties: {
          success: {
            type: "boolean",
            default: true,
          },
          attempts: {
            type: "integer",
          },
          note: {
            type: "string",
          },
        },
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
      resume_schema: {
        type: "object",
        required: ["attempts"],
        properties: {
          attempts: {
            type: "integer",
            title: "Attempts",
          },
        },
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
