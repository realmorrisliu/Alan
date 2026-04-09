import { currentStructuredQuestion, type StructuredFormState } from "../structured-input.js";
import {
  parseSchemaDrivenYieldForm,
  type SchemaDrivenYieldForm,
} from "../schema-driven-yield.js";
import { confirmationActionOptions, structuredQuestions, type StructuredQuestion } from "../yield.js";
import { getAdaptiveSurface } from "./registry.js";
import type { AdaptiveSurfaceDefinition, AdaptiveSurfaceRenderContext } from "./types.js";
import type { PendingYield } from "./yield-state.js";

export interface AdaptiveSurfaceViewModel {
  pendingStructuredQuestions: StructuredQuestion[];
  pendingSchemaForm: SchemaDrivenYieldForm | null;
  activeStructuredQuestion: StructuredQuestion | null;
  activeSchemaQuestion: StructuredQuestion | null;
  activeSurface: AdaptiveSurfaceDefinition | null;
  adaptiveSurfaceContext: AdaptiveSurfaceRenderContext | null;
}

export interface BuildAdaptiveSurfaceViewModelInput {
  pendingYield: PendingYield | null;
  confirmationActionIndex: number;
  structuredFormState: StructuredFormState | null;
  schemaFormState: StructuredFormState | null;
}

export function buildAdaptiveSurfaceViewModel({
  pendingYield,
  confirmationActionIndex,
  structuredFormState,
  schemaFormState,
}: BuildAdaptiveSurfaceViewModelInput): AdaptiveSurfaceViewModel {
  const pendingStructuredQuestions =
    pendingYield?.kind === "structured_input"
      ? structuredQuestions(pendingYield.payload)
      : [];
  const pendingSchemaForm =
    pendingYield &&
    (pendingYield.kind === "dynamic_tool" || pendingYield.kind === "custom")
      ? parseSchemaDrivenYieldForm(pendingYield.payload)
      : null;
  const activeStructuredQuestion =
    structuredFormState && pendingYield?.kind === "structured_input"
      ? currentStructuredQuestion(
          structuredFormState,
          pendingStructuredQuestions,
        )
      : null;
  const activeSchemaQuestion =
    schemaFormState && pendingSchemaForm
      ? currentStructuredQuestion(schemaFormState, pendingSchemaForm.questions)
      : null;
  const activeSurface = getAdaptiveSurface(pendingYield);
  const adaptiveSurfaceContext = pendingYield
    ? {
        pendingYield,
        confirmation:
          pendingYield.kind === "confirmation"
            ? {
                actionIndex: confirmationActionIndex,
                options: confirmationActionOptions(pendingYield.payload),
              }
            : undefined,
        schemaForm: pendingSchemaForm
          ? {
              title: pendingSchemaForm.title,
              prompt: pendingSchemaForm.prompt,
              formState: schemaFormState,
              questions: pendingSchemaForm.questions,
              activeQuestion: activeSchemaQuestion,
            }
          : undefined,
        structuredInput:
          pendingYield.kind === "structured_input"
            ? {
                formState: structuredFormState,
                questions: pendingStructuredQuestions,
                activeQuestion: activeStructuredQuestion,
              }
            : undefined,
      }
    : null;

  return {
    pendingStructuredQuestions,
    pendingSchemaForm,
    activeStructuredQuestion,
    activeSchemaQuestion,
    activeSurface,
    adaptiveSurfaceContext,
  };
}

export function isAdaptiveSurfaceReadyForInput(
  viewModel: AdaptiveSurfaceViewModel,
): boolean {
  const context = viewModel.adaptiveSurfaceContext;
  if (!context || !viewModel.activeSurface?.handleInputKey) {
    return false;
  }

  const { pendingYield } = context;
  if (
    pendingYield.kind === "structured_input" &&
    (!context.structuredInput?.formState ||
      !context.structuredInput.activeQuestion)
  ) {
    return false;
  }

  if (
    (pendingYield.kind === "dynamic_tool" || pendingYield.kind === "custom") &&
    context.schemaForm &&
    (!context.schemaForm.formState || !context.schemaForm.activeQuestion)
  ) {
    return false;
  }

  return true;
}
