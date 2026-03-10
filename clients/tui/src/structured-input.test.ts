import { describe, expect, test } from "bun:test";
import {
  buildStructuredResumePayload,
  createStructuredFormState,
  moveStructuredSingleSelection,
  moveStructuredOptionCursor,
  questionAnswerPreview,
  questionValidationError,
  selectStructuredSingleOption,
  shouldReuseStructuredFormState,
  structuredFormValidationError,
  toggleStructuredMultiOption,
} from "./structured-input";
import type { StructuredQuestion } from "./yield";

const QUESTIONS: StructuredQuestion[] = [
  {
    id: "branch",
    label: "Branch",
    prompt: "Git branch",
    kind: "text",
    required: true,
    placeholder: "feature/adaptive-yield-ui",
  },
  {
    id: "provider",
    label: "Provider",
    prompt: "Choose a provider",
    kind: "single_select",
    options: [
      { value: "openai", label: "OpenAI" },
      { value: "anthropic", label: "Anthropic" },
    ],
    defaultValue: "anthropic",
  },
  {
    id: "envs",
    label: "Environments",
    prompt: "Choose targets",
    kind: "multi_select",
    options: [
      { value: "staging", label: "Staging" },
      { value: "prod", label: "Production" },
      { value: "qa", label: "QA" },
    ],
    defaultValues: ["staging"],
    minSelections: 1,
    maxSelections: 2,
  },
];

const OPTIONAL_SINGLE_SELECT_QUESTION: StructuredQuestion = {
  id: "optional-provider",
  label: "Optional Provider",
  prompt: "Choose a provider if needed",
  kind: "single_select",
  options: [
    { value: "openai", label: "OpenAI" },
    { value: "anthropic", label: "Anthropic" },
  ],
  required: false,
};

describe("structured input helpers", () => {
  test("createStructuredFormState seeds defaults", () => {
    const state = createStructuredFormState("req-1", QUESTIONS);

    expect(state.answers).toEqual({
      branch: "",
      provider: "anthropic",
      envs: ["staging"],
    });
    expect(state.questionSignature).not.toBe("");
    expect(state.optionCursorByQuestionId.provider).toBe(1);
    expect(state.optionCursorByQuestionId.envs).toBe(0);
  });

  test("reuse helper rejects payload changes for same request id", () => {
    const state = createStructuredFormState("req-1", QUESTIONS);
    const changedQuestions: StructuredQuestion[] = [
      QUESTIONS[0],
      {
        ...QUESTIONS[1],
        defaultValue: "openai",
      },
      QUESTIONS[2],
    ];

    expect(shouldReuseStructuredFormState(state, "req-1", QUESTIONS)).toBe(
      true,
    );
    expect(
      shouldReuseStructuredFormState(state, "req-1", changedQuestions),
    ).toBe(false);
    expect(shouldReuseStructuredFormState(state, "req-2", QUESTIONS)).toBe(
      false,
    );
  });

  test("single-select and multi-select helpers update answers", () => {
    let state = createStructuredFormState("req-1", QUESTIONS);
    state = moveStructuredOptionCursor(state, QUESTIONS[1], -1);
    state = selectStructuredSingleOption(state, QUESTIONS[1], 0);
    state = toggleStructuredMultiOption(state, QUESTIONS[2], 1);

    expect(state.answers.provider).toBe("openai");
    expect(state.answers.envs).toEqual(["staging", "prod"]);
  });

  test("optional single-select can stay blank until user chooses", () => {
    const state = createStructuredFormState("req-1", [
      OPTIONAL_SINGLE_SELECT_QUESTION,
    ]);

    expect(state.answers[OPTIONAL_SINGLE_SELECT_QUESTION.id]).toBe("");
    expect(
      questionValidationError(state, OPTIONAL_SINGLE_SELECT_QUESTION),
    ).toBeNull();
  });

  test("single-select navigation updates the chosen answer", () => {
    let state = createStructuredFormState("req-1", [
      OPTIONAL_SINGLE_SELECT_QUESTION,
    ]);
    state = moveStructuredSingleSelection(
      state,
      OPTIONAL_SINGLE_SELECT_QUESTION,
      1,
    );

    expect(state.answers[OPTIONAL_SINGLE_SELECT_QUESTION.id]).toBe("anthropic");
    expect(
      state.optionCursorByQuestionId[OPTIONAL_SINGLE_SELECT_QUESTION.id],
    ).toBe(1);
  });

  test("multi-select helper respects maxSelections", () => {
    let state = createStructuredFormState("req-1", QUESTIONS);
    state = toggleStructuredMultiOption(state, QUESTIONS[2], 1);
    state = toggleStructuredMultiOption(state, QUESTIONS[2], 2);

    expect(state.answers.envs).toEqual(["staging", "prod"]);
  });

  test("validation reports the first blocking question", () => {
    const state = createStructuredFormState("req-1", QUESTIONS);
    expect(questionValidationError(state, QUESTIONS[0])).toBe(
      "Answer required.",
    );
    expect(structuredFormValidationError(state, QUESTIONS)).toBe(
      "Branch: Answer required.",
    );
  });

  test("validation rejects unknown select options", () => {
    const state = {
      ...createStructuredFormState("req-1", QUESTIONS),
      answers: {
        branch: "main",
        provider: "mistral",
        envs: ["staging", "preview"],
      },
    };

    expect(questionValidationError(state, QUESTIONS[1])).toBe(
      "Unknown option: mistral. Use one of: openai, anthropic.",
    );
    expect(questionValidationError(state, QUESTIONS[2])).toBe(
      "Unknown option: preview. Use one of: staging, prod, qa.",
    );
  });

  test("payload builder omits unanswered optional values", () => {
    let state = createStructuredFormState("req-1", QUESTIONS);
    state = {
      ...state,
      answers: {
        ...state.answers,
        branch: "main",
      },
    };

    expect(buildStructuredResumePayload(state, QUESTIONS)).toEqual({
      answers: [
        { question_id: "branch", value: "main" },
        { question_id: "provider", value: "anthropic" },
        { question_id: "envs", value: ["staging"] },
      ],
    });
  });

  test("questionAnswerPreview renders readable selections", () => {
    const state = createStructuredFormState("req-1", QUESTIONS);
    expect(questionAnswerPreview(state, QUESTIONS[0])).toBe(
      "feature/adaptive-yield-ui",
    );
    expect(questionAnswerPreview(state, QUESTIONS[1])).toBe("Anthropic");
    expect(questionAnswerPreview(state, QUESTIONS[2])).toBe("Staging");
  });
});
