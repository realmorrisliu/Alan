# Provider Reasoning Effort Migration

Alan now treats named reasoning effort as the primary cross-provider control:

```toml
model_reasoning_effort = "medium"
```

Use one of `none`, `minimal`, `low`, `medium`, `high`, or `xhigh` when the
selected model declares support for that effort. Omit the field to use Alan's
model-catalog default or, when Alan has no catalog metadata for the provider,
the provider default.

`thinking_budget_tokens` remains available, but it is a provider-specific
compatibility setting rather than Alan's canonical reasoning control:

```toml
# Keep only for budget-native providers or temporary compatibility.
thinking_budget_tokens = 2048
```

Do not set `model_reasoning_effort` and `thinking_budget_tokens` in the same
agent config. Alan rejects that combination because a named effort and a raw
budget are two different control models.

## OpenAI Migration

Older OpenAI configs sometimes used `thinking_budget_tokens` only to make Alan
infer a request `reasoning_effort`. Replace those budgets with the closest
named effort:

| Old budget intent | New config |
| --- | --- |
| very small reasoning budget | `model_reasoning_effort = "minimal"` |
| shallow reasoning | `model_reasoning_effort = "low"` |
| default reasoning | `model_reasoning_effort = "medium"` |
| deeper reasoning | `model_reasoning_effort = "high"` |
| maximum model effort, when supported | `model_reasoning_effort = "xhigh"` |

For OpenAI Responses Alan sends the selected effort as `reasoning.effort`. For
OpenAI Chat Completions Alan sends it as `reasoning_effort`.

## When To Keep Budgets

Keep `thinking_budget_tokens` when the target provider is budget-native, such
as Anthropic extended thinking or Gemini 2.5 thinking budgets, and you need a
specific provider token budget rather than Alan's named effort presets.

Alan normalizes the selected named effort or legacy budget in the runtime before
provider dispatch. Provider-specific `extra_params` may still carry temporary
compatibility fields when no canonical control is set, but they must not
override a resolved runtime effort or budget.

If a provider/model supports both in the future, prefer named effort in shared
agent configuration and reserve raw budgets for provider-specific profiles or
temporary experiments.
