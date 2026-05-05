# Provider Reasoning Effort Migration

Alan now treats named reasoning effort as the primary cross-provider control:

```toml
model_reasoning_effort = "medium"
```

Use one of `none`, `minimal`, `low`, `medium`, `high`, or `xhigh` when the
selected model declares support for that effort. Omit the field to use Alan's
model-catalog default or, when Alan has no catalog metadata for the provider,
the provider default.

`thinking_budget_tokens` has been removed as a public config and request field.
If an agent config or API payload still contains it, Alan rejects the input with
guidance to use `model_reasoning_effort`.

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

## Provider-Native Budgets

Some provider APIs still require budget-shaped wire fields, such as Anthropic
extended thinking or Gemini 2.5 thinking budgets. Those budgets are now derived
inside Alan from named effort presets and model/provider metadata. They are not a
public config or API control.

If a provider/model needs a new budget mapping, add it to Alan's provider
projection logic and tests rather than reintroducing a user-facing raw budget
field.
