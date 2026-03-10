import type React from "react";
import type { Dispatch, SetStateAction } from "react";
import type { Key } from "ink";
import type { StructuredFormState } from "../structured-input.js";
import type { StructuredQuestion } from "../yield.js";
import type { PendingYield, PendingYieldKind } from "./yield-state.js";

export type SystemEventType =
  | "system_message"
  | "system_warning"
  | "system_error";

export interface AdaptiveSurfaceEventMessage {
  type: SystemEventType;
  message: string;
}

export interface StructuredInputSurfaceState {
  formState: StructuredFormState | null;
  questions: StructuredQuestion[];
  activeQuestion: StructuredQuestion | null;
}

export interface ConfirmationSurfaceState {
  actionIndex: number;
  options: string[];
}

export interface AdaptiveSurfaceRenderContext {
  pendingYield: PendingYield;
  confirmation?: ConfirmationSurfaceState;
  structuredInput?: StructuredInputSurfaceState;
}

export interface AdaptiveSurfaceInputContext extends AdaptiveSurfaceRenderContext {
  inputValue: string;
}

export interface AdaptiveSurfaceKeyContext {
  pendingYield: PendingYield;
  input: string;
  key: Key;
  inputValue: string;
  confirmation?: ConfirmationSurfaceState;
  structuredInput?: StructuredInputSurfaceState;
  setInputValue: Dispatch<SetStateAction<string>>;
  addSystemEvent: (type: SystemEventType, message: string) => void;
  submitPendingYield: (content: unknown) => void;
  confirmationControls?: {
    setActionIndex: Dispatch<SetStateAction<number>>;
  };
  structuredInputControls?: {
    setFormState: Dispatch<SetStateAction<StructuredFormState | null>>;
    submitStructuredForm: () => void;
    confirmActiveQuestion: () => void;
  };
}

export interface AdaptiveSurfaceDefinition {
  kind: PendingYieldKind;
  buildAnnouncement: (
    pendingYield: PendingYield,
  ) => AdaptiveSurfaceEventMessage[];
  render: (context: AdaptiveSurfaceRenderContext) => React.ReactNode;
  footerHint: (context: AdaptiveSurfaceRenderContext) => string;
  inputLabel: (context: AdaptiveSurfaceInputContext) => string;
  inputPlaceholder: (context: AdaptiveSurfaceInputContext) => string;
  inputFocus?: (context: AdaptiveSurfaceInputContext) => boolean;
  handleInputKey?: (context: AdaptiveSurfaceKeyContext) => boolean;
}
