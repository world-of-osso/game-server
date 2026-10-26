#!/usr/bin/env python3
"""Format the workspace, including spell modules registered inside a macro."""
import argparse
import subprocess
import tomllib
from pathlib import Path


def format_workspace(root: Path, *, check: bool) -> int:
    command = ["cargo", "fmt", "--all"]
    if check:
        command.append("--check")
    result = subprocess.run(command, cwd=root)
    if result.returncode:
        return result.returncode
    modules = sorted((root / "crates/server-spell-scripts/src/modules").rglob("*.rs"))
    if not modules:
        raise RuntimeError(f"No spell-script Rust modules under {root}")
    manifest = tomllib.loads((root / "Cargo.toml").read_text())
    command = ["rustfmt", "--edition", manifest["workspace"]["package"]["edition"]]
    if check:
        command.append("--check")
    command.extend(str(path) for path in modules)
    return subprocess.run(command, cwd=root).returncode


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    return format_workspace(Path(__file__).resolve().parents[1], check=args.check)


if __name__ == "__main__":
    raise SystemExit(main())
