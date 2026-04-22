#!/usr/bin/env python3

import argparse
import json
import subprocess
import sys
from collections import Counter
from pathlib import Path

from swebench_lite_common import (
    build_row_index,
    load_dataset_rows,
    read_instance_ids,
    slug_repo_name,
)


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description=(
            "Prepare clean SWE-bench Lite git workspaces at each case's base_commit "
            "using official dataset rows."
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
        required=True,
        help="Output directory using the layout <workspace-root>/<instance_id>.",
    )
    parser.add_argument(
        "--repo-cache-root",
        help=(
            "Mirror cache directory used to avoid repeated remote clones. "
            "Default: <workspace-root>/.repo-cache."
        ),
    )
    parser.add_argument(
        "--github-root",
        default="https://github.com",
        help="Repository host prefix. Default: https://github.com.",
    )
    parser.add_argument(
        "--workspace-map-output",
        help="Optional JSON output path for instance_id -> workspace_dir mappings.",
    )
    parser.add_argument(
        "--skip-mirror-fetch",
        action="store_true",
        help="Reuse existing mirrors without fetching remote updates.",
    )
    parser.add_argument(
        "--reuse-existing-workspaces",
        action="store_true",
        help=(
            "Keep an existing workspace when it is already a clean git checkout "
            "at the requested base_commit."
        ),
    )
    return parser.parse_args()


def run_git(args: list[str], cwd: Path | None = None) -> str:
    result = subprocess.run(
        ["git", *args],
        check=True,
        cwd=str(cwd) if cwd else None,
        capture_output=True,
        text=True,
    )
    return result.stdout.strip()


def repo_clone_url(repo: str, github_root: str) -> str:
    return f"{github_root.rstrip('/')}/{repo}.git"


def ensure_repo_mirror(repo: str, mirror_path: Path, github_root: str, skip_fetch: bool) -> None:
    clone_url = repo_clone_url(repo, github_root)
    if mirror_path.exists():
        if not mirror_path.is_dir():
            raise ValueError(f"Mirror path exists but is not a directory: {mirror_path}")
        if not skip_fetch:
            run_git(["remote", "update", "--prune"], cwd=mirror_path)
        return

    mirror_path.parent.mkdir(parents=True, exist_ok=True)
    run_git(["clone", "--mirror", clone_url, str(mirror_path)])


def existing_workspace_matches(workspace_dir: Path, base_commit: str) -> bool:
    if not workspace_dir.exists():
        return False
    if not workspace_dir.is_dir():
        raise ValueError(f"Workspace path exists but is not a directory: {workspace_dir}")

    try:
        top_level = Path(run_git(["rev-parse", "--show-toplevel"], cwd=workspace_dir)).resolve()
    except subprocess.CalledProcessError as exc:
        raise ValueError(f"Workspace is not a git repository: {workspace_dir}") from exc

    if top_level != workspace_dir.resolve():
        raise ValueError(
            f"Workspace path must be the repository root: {workspace_dir} (resolved {top_level})"
        )

    head_commit = run_git(["rev-parse", "HEAD"], cwd=workspace_dir)
    status = run_git(["status", "--short", "--untracked-files=all"], cwd=workspace_dir)
    return head_commit == base_commit and status == ""


def prepare_workspace(
    workspace_dir: Path,
    mirror_path: Path,
    repo: str,
    base_commit: str,
    github_root: str,
) -> None:
    workspace_dir.parent.mkdir(parents=True, exist_ok=True)
    run_git(["clone", str(mirror_path), str(workspace_dir)])
    run_git(["remote", "set-url", "origin", repo_clone_url(repo, github_root)], cwd=workspace_dir)
    run_git(["checkout", "--detach", base_commit], cwd=workspace_dir)
    status = run_git(["status", "--short", "--untracked-files=all"], cwd=workspace_dir)
    if status:
        raise ValueError(f"Prepared workspace is not clean: {workspace_dir}")


def main() -> int:
    args = parse_args()
    instance_ids_path = Path(args.instance_ids_file).expanduser().resolve()
    workspace_root = Path(args.workspace_root).expanduser().resolve()
    repo_cache_root = (
        Path(args.repo_cache_root).expanduser().resolve()
        if args.repo_cache_root
        else workspace_root / ".repo-cache"
    )
    workspace_map_output = (
        Path(args.workspace_map_output).expanduser().resolve()
        if args.workspace_map_output
        else workspace_root / "workspace_map.json"
    )

    if not args.dataset_file and not args.dataset_name:
        raise SystemExit("Provide at least one --dataset-file or --dataset-name.")

    instance_ids = read_instance_ids(instance_ids_path)
    dataset_rows = load_dataset_rows(args.dataset_file, args.dataset_name, args.split)
    row_index = build_row_index(dataset_rows)

    missing_rows = [instance_id for instance_id in instance_ids if instance_id not in row_index]
    if missing_rows:
        raise SystemExit(
            "Missing instance rows in dataset input: " + ", ".join(sorted(missing_rows))
        )

    workspace_root.mkdir(parents=True, exist_ok=True)
    repo_cache_root.mkdir(parents=True, exist_ok=True)

    repo_counts: Counter[str] = Counter()
    row_by_instance: dict[str, dict] = {}
    for instance_id in instance_ids:
        row = row_index[instance_id]
        repo = str(row.get("repo", "")).strip()
        base_commit = str(row.get("base_commit", "")).strip()
        if not repo or not base_commit:
            raise SystemExit(
                f"Dataset row for {instance_id} must include non-empty repo and base_commit"
            )
        row_by_instance[instance_id] = row
        repo_counts[repo] += 1

    mirror_errors: dict[str, str] = {}
    mirror_by_repo: dict[str, Path] = {}
    for repo in sorted(repo_counts):
        mirror_path = repo_cache_root / f"{slug_repo_name(repo)}.git"
        mirror_by_repo[repo] = mirror_path
        try:
            ensure_repo_mirror(repo, mirror_path, args.github_root, args.skip_mirror_fetch)
        except (subprocess.CalledProcessError, ValueError) as exc:
            mirror_errors[repo] = str(exc)

    workspace_map: dict[str, str] = {}
    prepared: list[dict[str, str]] = []
    reused: list[dict[str, str]] = []
    failures: list[dict[str, str]] = []

    for instance_id in instance_ids:
        row = row_by_instance[instance_id]
        repo = str(row["repo"])
        base_commit = str(row["base_commit"])
        environment_setup_commit = str(row.get("environment_setup_commit", "")).strip()
        workspace_dir = workspace_root / instance_id
        workspace_map[instance_id] = str(workspace_dir)

        if repo in mirror_errors:
            failures.append(
                {
                    "instance_id": instance_id,
                    "repo": repo,
                    "workspace_dir": str(workspace_dir),
                    "reason": f"mirror_error: {mirror_errors[repo]}",
                }
            )
            continue

        try:
            if workspace_dir.exists():
                if not args.reuse_existing_workspaces:
                    raise ValueError(
                        "workspace already exists; rerun with --reuse-existing-workspaces "
                        "when it is already clean at the requested base_commit"
                    )
                if existing_workspace_matches(workspace_dir, base_commit):
                    reused.append(
                        {
                            "instance_id": instance_id,
                            "repo": repo,
                            "base_commit": base_commit,
                            "environment_setup_commit": environment_setup_commit,
                            "workspace_dir": str(workspace_dir),
                        }
                    )
                    continue
                raise ValueError(
                    f"existing workspace does not match clean base_commit {base_commit}"
                )

            prepare_workspace(
                workspace_dir=workspace_dir,
                mirror_path=mirror_by_repo[repo],
                repo=repo,
                base_commit=base_commit,
                github_root=args.github_root,
            )
            prepared.append(
                {
                    "instance_id": instance_id,
                    "repo": repo,
                    "base_commit": base_commit,
                    "environment_setup_commit": environment_setup_commit,
                    "workspace_dir": str(workspace_dir),
                }
            )
        except (subprocess.CalledProcessError, ValueError) as exc:
            failures.append(
                {
                    "instance_id": instance_id,
                    "repo": repo,
                    "base_commit": base_commit,
                    "environment_setup_commit": environment_setup_commit,
                    "workspace_dir": str(workspace_dir),
                    "reason": str(exc),
                }
            )

    workspace_map_output.parent.mkdir(parents=True, exist_ok=True)
    workspace_map_output.write_text(
        json.dumps(workspace_map, indent=2) + "\n",
        encoding="utf-8",
    )

    report_payload = {
        "instance_ids_file": str(instance_ids_path),
        "instance_count": len(instance_ids),
        "dataset_files": [str(Path(path).expanduser().resolve()) for path in args.dataset_file],
        "dataset_source": args.dataset_name,
        "workspace_root": str(workspace_root),
        "repo_cache_root": str(repo_cache_root),
        "workspace_map_file": str(workspace_map_output),
        "github_root": args.github_root,
        "reuse_existing_workspaces": args.reuse_existing_workspaces,
        "repos": dict(sorted(repo_counts.items())),
        "prepared_count": len(prepared),
        "reused_count": len(reused),
        "failed_count": len(failures),
        "prepared": prepared,
        "reused": reused,
        "failures": failures,
    }
    report_path = workspace_root / "preparation_report.json"
    report_path.write_text(json.dumps(report_payload, indent=2) + "\n", encoding="utf-8")

    print(f"workspace_root\t{workspace_root}")
    print(f"workspace_map\t{workspace_map_output}")
    print(f"report\t{report_path}")
    print(f"prepared_count\t{len(prepared)}")
    print(f"reused_count\t{len(reused)}")
    print(f"failed_count\t{len(failures)}")
    for repo, count in sorted(repo_counts.items()):
        print(f"repo_count\t{repo}\t{count}")

    if failures:
        for failure in failures:
            print(
                "failure\t{instance_id}\t{reason}".format(**failure),
                file=sys.stderr,
            )
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
