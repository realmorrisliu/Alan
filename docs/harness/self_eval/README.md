# Self-Eval Suite

Self-eval compares `baseline` vs `candidate` profile behavior and emits a promotion report.

## Run Modes

1. `local`
   - Command:
     - `bash scripts/harness/run_self_eval_suite.sh --mode local`
   - Uses deterministic blocking autonomy scenarios.
   - Always emits report; does not fail on gate mismatch.
2. `ci`
   - Command:
     - `bash scripts/harness/run_self_eval_suite.sh --mode ci`
   - Uses deterministic blocking autonomy scenarios.
   - Baseline runs against `HEAD~1`; candidate runs against current `HEAD`.
   - Fails with non-zero exit code when promotion gate checks fail.
3. `nightly`
   - Command:
     - `bash scripts/harness/run_self_eval_suite.sh --mode nightly`
   - Uses full autonomy scenario set (`run_autonomy_suite.sh` without `--ci-blocking`).
   - Baseline runs against `HEAD~1`; candidate runs against current `HEAD`.
   - Intended for broader trend monitoring.

## Execution Isolation

To avoid cache-order bias in duration comparisons, baseline and candidate runs use isolated
`CARGO_TARGET_DIR` directories under each profile artifact directory.

## Artifacts

Generated under:

- `target/harness/self_eval/latest/`

Key files:

1. `input_script.json` (scenario fixture snapshot)
2. `promotion_thresholds.env` (resolved threshold config)
3. `baseline/profile_metrics.json`
4. `candidate/profile_metrics.json`
5. `profile_regression_report.json` (comparison + gate checks)

## Threshold Configuration

Versioned config file:

- `docs/harness/self_eval/promotion_thresholds.v1.env`
