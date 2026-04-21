#!/usr/bin/env python3

import argparse
import json
import sys
from collections import Counter
from pathlib import Path
from typing import Iterable


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Materialize Alan full-steward SWE-bench Lite case manifests, problem "
            "statement files, and a suite manifest from official dataset rows."
        )
    )
    parser.add_argument(
        "--instance-ids-file",
        required=True,
        help="Text file containing one SWE-bench instance_id per line.",
    )
    parser.add_argument(
        "--dataset-file",
        action="append",
        default=[],
        help=(
            "Local JSON/JSONL dataset export. Can be repeated. Supports raw row "
            "objects, datasets-server responses, or JSONL rows."
        ),
    )
    parser.add_argument(
        "--dataset-name",
        help=(
            "Official Hugging Face dataset name. Requires the optional "
            "`datasets` Python package when used."
        ),
    )
    parser.add_argument(
        "--split",
        default="test",
        help="Dataset split to load when --dataset-name is used. Default: test.",
    )
    parser.add_argument(
        "--workspace-root",
        help=(
            "Directory containing one prepared workspace per instance_id, using "
            "<workspace-root>/<instance_id>."
        ),
    )
    parser.add_argument(
        "--workspace-map-file",
        help="Optional JSON file mapping instance_id to workspace_dir.",
    )
    parser.add_argument(
        "--output-dir",
        required=True,
        help="Output directory for generated cases/, problem_statements/, and suite.json.",
    )
    parser.add_argument(
        "--suite-name",
        default="swebench_lite_pilot_v1",
        help="Suite name to write into suite.json. Default: swebench_lite_pilot_v1.",
    )
    parser.add_argument(
        "--dataset-label",
        default="SWE-bench Lite",
        help="Human-readable dataset label for case manifests. Default: SWE-bench Lite.",
    )
    parser.add_argument(
        "--scoring-dataset-name",
        default="princeton-nlp/SWE-bench_Lite",
        help=(
            "Official harness dataset name written into suite.json. "
            "Default: princeton-nlp/SWE-bench_Lite."
        ),
    )
    parser.add_argument(
        "--max-workers",
        type=int,
        default=4,
        help="Official harness max_workers hint written into suite.json. Default: 4.",
    )
    parser.add_argument(
        "--timeout-secs",
        type=int,
        default=1800,
        help="timeout_secs written into each case manifest. Default: 1800.",
    )
    parser.add_argument(
        "--allow-missing-workspaces",
        action="store_true",
        help="Allow generation even when a referenced workspace directory does not exist yet.",
    )
    return parser.parse_args()


def read_instance_ids(path: Path) -> list[str]:
    instance_ids: list[str] = []
    for raw_line in path.read_text(encoding="utf-8").splitlines():
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue
        instance_ids.append(line)
    if not instance_ids:
        raise ValueError(f"No instance ids found in {path}")
    return instance_ids


def normalize_row(item: object) -> dict:
    if not isinstance(item, dict):
        raise ValueError(f"Unsupported dataset row payload: {type(item).__name__}")
    if "row" in item and isinstance(item["row"], dict):
        return item["row"]
    return item


def load_rows_from_json_payload(payload: object) -> list[dict]:
    if isinstance(payload, dict):
        if "rows" in payload and isinstance(payload["rows"], list):
            return [normalize_row(item) for item in payload["rows"]]
        if "instance_id" in payload:
            return [normalize_row(payload)]
        raise ValueError("Unsupported JSON object payload; expected rows[] or an instance row")
    if isinstance(payload, list):
        return [normalize_row(item) for item in payload]
    raise ValueError("Unsupported JSON payload; expected an object or array")


def load_rows_from_dataset_file(path: Path) -> list[dict]:
    raw = path.read_text(encoding="utf-8").strip()
    if not raw:
        return []
    if raw[0] in "[{":
        try:
            return load_rows_from_json_payload(json.loads(raw))
        except json.JSONDecodeError:
            # Fall back to linewise parsing for JSONL exports that also begin
            # with "{" on the first line.
            pass
    rows: list[dict] = []
    for line in raw.splitlines():
        line = line.strip()
        if not line:
            continue
        rows.append(normalize_row(json.loads(line)))
    return rows


def load_rows_from_hf_dataset(dataset_name: str, split: str) -> list[dict]:
    try:
        from datasets import load_dataset
    except ImportError as exc:
        raise SystemExit(
            "The `datasets` package is required for --dataset-name. "
            "Install it or pass one or more --dataset-file exports instead."
        ) from exc

    dataset = load_dataset(dataset_name, split=split)
    return [dict(row) for row in dataset]


def build_row_index(dataset_rows: Iterable[dict]) -> dict[str, dict]:
    index: dict[str, dict] = {}
    for row in dataset_rows:
        instance_id = row.get("instance_id")
        if not instance_id:
            continue
        if instance_id not in index:
            index[instance_id] = row
    return index


def load_workspace_map(args: argparse.Namespace, instance_ids: Iterable[str]) -> tuple[dict[str, Path], list[str]]:
    mapping: dict[str, Path] = {}
    missing: list[str] = []

    if args.workspace_map_file:
        payload = json.loads(Path(args.workspace_map_file).read_text(encoding="utf-8"))
        if not isinstance(payload, dict):
            raise ValueError("--workspace-map-file must contain a JSON object")
        for instance_id, raw_path in payload.items():
            mapping[instance_id] = Path(str(raw_path)).expanduser().resolve()

    if args.workspace_root:
        root = Path(args.workspace_root).expanduser().resolve()
        for instance_id in instance_ids:
            mapping.setdefault(instance_id, root / instance_id)

    if not mapping:
        raise ValueError("Provide --workspace-root or --workspace-map-file")

    for instance_id in instance_ids:
        if instance_id not in mapping:
            missing.append(instance_id)

    return mapping, missing


def main() -> int:
    args = parse_args()
    instance_ids_path = Path(args.instance_ids_file).expanduser().resolve()
    output_dir = Path(args.output_dir).expanduser().resolve()

    if not args.dataset_file and not args.dataset_name:
        raise SystemExit("Provide at least one --dataset-file or --dataset-name.")

    instance_ids = read_instance_ids(instance_ids_path)

    dataset_rows: list[dict] = []
    for dataset_file in args.dataset_file:
        dataset_rows.extend(load_rows_from_dataset_file(Path(dataset_file).expanduser().resolve()))
    if args.dataset_name:
        dataset_rows.extend(load_rows_from_hf_dataset(args.dataset_name, args.split))

    row_index = build_row_index(dataset_rows)
    missing_rows = [instance_id for instance_id in instance_ids if instance_id not in row_index]
    if missing_rows:
        raise SystemExit(
            "Missing instance rows in dataset input: " + ", ".join(sorted(missing_rows))
        )

    workspace_map, missing_workspace_mappings = load_workspace_map(args, instance_ids)
    if missing_workspace_mappings:
        raise SystemExit(
            "Missing workspace mapping for instances: "
            + ", ".join(sorted(missing_workspace_mappings))
        )

    output_dir.mkdir(parents=True, exist_ok=True)
    cases_dir = output_dir / "cases"
    statements_dir = output_dir / "problem_statements"
    cases_dir.mkdir(parents=True, exist_ok=True)
    statements_dir.mkdir(parents=True, exist_ok=True)

    missing_workspace_dirs: list[str] = []
    repo_counts: Counter[str] = Counter()
    case_files: list[str] = []

    for instance_id in instance_ids:
        row = row_index[instance_id]
        workspace_dir = workspace_map[instance_id]
        if not workspace_dir.exists():
            missing_workspace_dirs.append(instance_id)
            if not args.allow_missing_workspaces:
                raise SystemExit(
                    f"Workspace directory does not exist for {instance_id}: {workspace_dir}"
                )

        repo = row.get("repo", "unknown")
        repo_counts[repo] += 1

        problem_statement_path = statements_dir / f"{instance_id}.txt"
        problem_statement_path.write_text(
            row["problem_statement"].rstrip() + "\n",
            encoding="utf-8",
        )

        case_path = cases_dir / f"{instance_id}.json"
        case_payload = {
            "instance_id": instance_id,
            "dataset": args.dataset_label,
            "workspace_dir": str(workspace_dir),
            "problem_statement_file": str(problem_statement_path),
            "timeout_secs": args.timeout_secs,
        }
        case_path.write_text(
            json.dumps(case_payload, indent=2) + "\n",
            encoding="utf-8",
        )
        case_files.append(str(case_path.relative_to(output_dir)))

    suite_payload = {
        "suite": args.suite_name,
        "dataset": args.dataset_label,
        "dataset_name": args.scoring_dataset_name,
        "max_workers": args.max_workers,
        "cases": case_files,
    }
    suite_path = output_dir / "suite.json"
    suite_path.write_text(json.dumps(suite_payload, indent=2) + "\n", encoding="utf-8")

    report_payload = {
        "suite": args.suite_name,
        "dataset_name": args.scoring_dataset_name,
        "instance_ids_file": str(instance_ids_path),
        "instance_count": len(instance_ids),
        "repos": dict(sorted(repo_counts.items())),
        "dataset_files": [str(Path(path).expanduser().resolve()) for path in args.dataset_file],
        "dataset_source": args.dataset_name,
        "workspace_root": (
            str(Path(args.workspace_root).expanduser().resolve())
            if args.workspace_root
            else None
        ),
        "workspace_map_file": (
            str(Path(args.workspace_map_file).expanduser().resolve())
            if args.workspace_map_file
            else None
        ),
        "allow_missing_workspaces": args.allow_missing_workspaces,
        "missing_workspace_dirs": missing_workspace_dirs,
        "suite_json": str(suite_path),
    }
    report_path = output_dir / "materialization_report.json"
    report_path.write_text(json.dumps(report_payload, indent=2) + "\n", encoding="utf-8")

    print(f"suite_json\t{suite_path}")
    print(f"instance_count\t{len(instance_ids)}")
    for repo, count in sorted(repo_counts.items()):
        print(f"repo_count\t{repo}\t{count}")
    if missing_workspace_dirs:
        print(
            "warning\tmissing_workspaces\t" + ",".join(sorted(missing_workspace_dirs)),
            file=sys.stderr,
        )
    return 0


if __name__ == "__main__":
    sys.exit(main())
