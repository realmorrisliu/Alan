#!/usr/bin/env python3

import json
from pathlib import Path
from typing import Iterable


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


def load_dataset_rows(
    dataset_files: Iterable[str],
    dataset_name: str | None,
    split: str,
) -> list[dict]:
    dataset_rows: list[dict] = []
    for dataset_file in dataset_files:
        dataset_rows.extend(load_rows_from_dataset_file(Path(dataset_file).expanduser().resolve()))
    if dataset_name:
        dataset_rows.extend(load_rows_from_hf_dataset(dataset_name, split))
    return dataset_rows


def build_row_index(dataset_rows: Iterable[dict]) -> dict[str, dict]:
    index: dict[str, dict] = {}
    for row in dataset_rows:
        instance_id = row.get("instance_id")
        if not instance_id:
            continue
        if instance_id not in index:
            index[instance_id] = row
    return index


def slug_repo_name(repo: str) -> str:
    return repo.replace("/", "__")
