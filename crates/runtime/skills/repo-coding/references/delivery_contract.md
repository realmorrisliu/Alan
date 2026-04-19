# Repo-Worker Delivery Contract

Repo-worker runs should end with a bounded delivery artifact that the parent
steward can inspect without replaying the full child transcript.

## Required fields

The contract is expressed as JSON with these required fields:

1. `status`: one of `completed`, `blocked`, or `failed`
2. `summary`: concise explanation of what happened
3. `changed_files`: array of repo-relative file paths
4. `verification`: non-empty array of verification entries
5. `residual_risks`: array of remaining risks or blockers
6. `evaluator`: conditional evaluator decision and reason

## Verification entry shape

Each verification entry should include:

1. `command`
2. `scope`: `targeted` or `broader`
3. `status`: `passed`, `failed`, or `not_run`
4. `exit_code`
5. `summary`

## Evaluator field

`evaluator` should record:

1. `mode`: `not_needed`, `recommended`, or `used`
2. `reason`: concise explanation tied to the task state

This keeps the repo-worker output explicit about whether the run stayed inside
the stable solo loop or crossed into a case where evaluator support should have
been considered.
