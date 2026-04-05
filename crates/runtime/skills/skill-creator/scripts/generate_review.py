#!/usr/bin/env python3

import argparse
import shutil
import subprocess
import sys


def resolve_command() -> list[str]:
    binary = shutil.which("alan-skill-tools")
    if binary:
        return [binary]
    return ["cargo", "run", "-p", "alan-skill-tools", "--"]


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Regenerate the static review bundle for an eval run."
    )
    parser.add_argument("run_dir", help="Eval run directory containing run.json")
    args = parser.parse_args()

    command = [*resolve_command(), "generate-review", args.run_dir]
    return subprocess.run(command, check=False).returncode


if __name__ == "__main__":
    sys.exit(main())
