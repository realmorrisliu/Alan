import { describe, expect, test } from "bun:test";
import { createStructuredFormState } from "../structured-input";
import {
  buildAdaptiveSurfaceViewModel,
  isAdaptiveSurfaceReadyForInput,
} from "./view-model";
import type { PendingYield } from "./yield-state";

describe("adaptive surface view model", () => {
  test("builds structured input context with the active question", () => {
    const pendingYield: PendingYield = {
      requestId: "req-1",
      kind: "structured_input",
      payload: {
        title: "Collect input",
        questions: [
          {
            id: "branch",
            label: "Branch",
            prompt: "Git branch",
            kind: "text",
            required: true,
          },
        ],
      },
    };

    const viewModel = buildAdaptiveSurfaceViewModel({
      pendingYield,
      confirmationActionIndex: 0,
      confirmationActionRequestId: null,
      structuredFormState: createStructuredFormState("req-1", [
        {
          id: "branch",
          label: "Branch",
          prompt: "Git branch",
          kind: "text",
          required: true,
        },
      ]),
      schemaFormState: null,
    });

    expect(viewModel.pendingStructuredQuestions).toHaveLength(1);
    expect(viewModel.activeStructuredQuestion?.id).toBe("branch");
    expect(
      viewModel.adaptiveSurfaceContext?.structuredInput?.activeQuestion?.id,
    ).toBe("branch");
    expect(isAdaptiveSurfaceReadyForInput(viewModel)).toBe(true);
  });

  test("blocks structured input keyboard handling until form state exists", () => {
    const viewModel = buildAdaptiveSurfaceViewModel({
      pendingYield: {
        requestId: "req-1",
        kind: "structured_input",
        payload: {
          title: "Collect input",
          questions: [
            {
              id: "branch",
              label: "Branch",
              prompt: "Git branch",
              kind: "text",
              required: true,
            },
          ],
        },
      },
      confirmationActionIndex: 0,
      confirmationActionRequestId: null,
      structuredFormState: null,
      schemaFormState: null,
    });

    expect(isAdaptiveSurfaceReadyForInput(viewModel)).toBe(false);
  });

  test("parses schema-driven form context for dynamic-tool yields", () => {
    const pendingYield: PendingYield = {
      requestId: "req-2",
      kind: "dynamic_tool",
      payload: {
        tool_name: "custom_tool",
        arguments: {},
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
          ],
        },
      },
    };

    const viewModel = buildAdaptiveSurfaceViewModel({
      pendingYield,
      confirmationActionIndex: 0,
      confirmationActionRequestId: null,
      structuredFormState: null,
      schemaFormState: createStructuredFormState("req-2", [
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
        },
      ]),
    });

    expect(viewModel.pendingSchemaForm?.title).toBe("Return tool result");
    expect(viewModel.activeSchemaQuestion?.id).toBe("success");
    expect(isAdaptiveSurfaceReadyForInput(viewModel)).toBe(true);
  });

  test("allows fallback command mode for custom yields without schema forms", () => {
    const viewModel = buildAdaptiveSurfaceViewModel({
      pendingYield: {
        requestId: "req-3",
        kind: "custom",
        payload: {
          title: "Raw custom yield",
        },
      },
      confirmationActionIndex: 0,
      confirmationActionRequestId: null,
      structuredFormState: null,
      schemaFormState: null,
    });

    expect(viewModel.pendingSchemaForm).toBeNull();
    expect(isAdaptiveSurfaceReadyForInput(viewModel)).toBe(true);
  });

  test("uses the payload default for a new confirmation request before local state syncs", () => {
    const viewModel = buildAdaptiveSurfaceViewModel({
      pendingYield: {
        requestId: "req-4",
        kind: "confirmation",
        payload: {
          summary: "Proceed?",
          options: ["approve", "reject"],
          default_option: "reject",
        },
      },
      confirmationActionIndex: 0,
      confirmationActionRequestId: "req-3",
      structuredFormState: null,
      schemaFormState: null,
    });

    expect(viewModel.adaptiveSurfaceContext?.confirmation).toEqual({
      actionIndex: 1,
      options: ["approve", "reject"],
    });
  });
});
