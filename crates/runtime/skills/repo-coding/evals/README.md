# Repo-Worker Evals

Package-local repo-coding eval fixtures and review assets live here.

Current surfaces:

1. `evals.json` as the manifest-first entrypoint for `alan skills eval`.
2. `files/benchmark_cases.json` as deterministic steward-vs-legacy routing fixtures.
3. `../scripts/run_benchmark_fixture.sh` and `../scripts/grade_benchmark_case.sh`
   as package-local benchmark helpers.

Run the scaffold locally with:

```bash
cargo run -p alan -- skills eval crates/runtime/skills/repo-coding --output-dir target/skills-eval/repo-coding/latest
```

Cross-package harness scenarios remain under:

- `docs/harness/scenarios/repo_worker/`
- `docs/harness/scenarios/coding_steward/`

## Lite-First Full-Steward Bring-Up

`run_swebench_full_steward_case.sh` is the M1 single-case external benchmark
entrypoint for this package.

It does not bypass Alan's orchestration layer. Instead it:

1. starts a real root/steward session,
2. submits one benchmark problem to that steward,
3. requires repo-local coding work to run through child repo-worker launch,
4. reads rollout/session artifacts after completion,
5. exports:
   - `model.patch`
   - `prediction.json`
   - `predictions.jsonl`
   - `run.json`
   - `assertion_report.json`
   - `kpi.json`

The case-level `run.json` also records Alan-native orchestration metadata such
as:

1. `spawn_count`
2. `escalation_count`
3. `child_runs`
4. `duration_secs`

Use the template case file as a starting point:

- `evals/files/swebench_lite_case.template.json`

The case file should point at:

1. one prepared `SWE-bench Lite` workspace checkout,
2. one problem-statement text file for that instance.

Run one full-steward case locally with:

```bash
bash crates/runtime/skills/repo-coding/scripts/run_swebench_full_steward_case.sh \
  crates/runtime/skills/repo-coding/evals/files/swebench_lite_case.template.json
```

This is intentionally operator-run and non-CI-blocking.

Recommended rollout order:

1. one Lite case,
2. a curated Lite subset,
3. full Lite,
4. curated Pro subsets.

For the M2 curated-subset step, use:

- `evals/files/swebench_lite_subset.template.json`
- `evals/files/swebench_lite_pilot_v1.ids.txt`

To materialize a real pilot subset from official Lite rows into Alan case/suite
manifests, use:

```bash
python3 crates/runtime/skills/repo-coding/scripts/prepare_swebench_lite_subset.py \
  --instance-ids-file crates/runtime/skills/repo-coding/evals/files/swebench_lite_pilot_v1.ids.txt \
  --dataset-name princeton-nlp/SWE-bench_Lite \
  --workspace-root /absolute/path/to/prepared/swebench-lite/workspaces \
  --output-dir target/benchmarks/swebench_lite/manifests/pilot_v1
```

If you do not want to install the optional `datasets` package, the same script
also accepts one or more local dataset exports via repeated `--dataset-file`
arguments. It supports:

1. JSONL rows with `instance_id` and `problem_statement`
2. JSON arrays of row objects
3. Hugging Face datasets-server JSON responses with `rows[].row`

The materializer writes:

1. `cases/<instance_id>.json`
2. `problem_statements/<instance_id>.txt`
3. `suite.json`
4. `materialization_report.json`

For an official rows fallback without extra Python packages:

```bash
curl -L -o /tmp/swebench-lite.rows-0.json \
  'https://datasets-server.huggingface.co/rows?dataset=princeton-nlp/SWE-bench_Lite&config=default&split=test&offset=0&length=100'
curl -L -o /tmp/swebench-lite.rows-100.json \
  'https://datasets-server.huggingface.co/rows?dataset=princeton-nlp/SWE-bench_Lite&config=default&split=test&offset=100&length=100'
curl -L -o /tmp/swebench-lite.rows-200.json \
  'https://datasets-server.huggingface.co/rows?dataset=princeton-nlp/SWE-bench_Lite&config=default&split=test&offset=200&length=100'

python3 crates/runtime/skills/repo-coding/scripts/prepare_swebench_lite_subset.py \
  --instance-ids-file crates/runtime/skills/repo-coding/evals/files/swebench_lite_pilot_v1.ids.txt \
  --dataset-file /tmp/swebench-lite.rows-0.json \
  --dataset-file /tmp/swebench-lite.rows-100.json \
  --dataset-file /tmp/swebench-lite.rows-200.json \
  --workspace-root /absolute/path/to/prepared/swebench-lite/workspaces \
  --output-dir target/benchmarks/swebench_lite/manifests/pilot_v1
```

```bash
bash crates/runtime/skills/repo-coding/scripts/run_swebench_full_steward_subset.sh \
  crates/runtime/skills/repo-coding/evals/files/swebench_lite_subset.template.json
```

Or run the generated pilot suite directly:

```bash
bash crates/runtime/skills/repo-coding/scripts/run_swebench_full_steward_subset.sh \
  target/benchmarks/swebench_lite/manifests/pilot_v1/suite.json
```

That suite runner aggregates per-case artifacts into one suite directory and
generates:

1. `predictions.jsonl`
2. `case_results.jsonl`
3. `run.json`
4. `benchmark.json`
5. `kpi.json`
6. `score_with_official_harness.sh`

Suite-level `run.json` and `benchmark.json` now also summarize
`total_escalation_count` across all executed cases.

Official harness scoring still happens outside Alan's runtime loop, but the
package now provides a thin wrapper so operators do not need to remember the
raw Python module entrypoint:

```bash
bash crates/runtime/skills/repo-coding/scripts/score_swebench_predictions.sh \
  target/benchmarks/swebench_lite/suites/swebench_lite_curated/predictions.jsonl
```
