#!/usr/bin/env python3
"""
Generate a repo-local Rust-focused code-index triage report.

The upstream tool is still the source of truth for indexing, but this script
filters out the obvious Rust false positives we see in this repository:

- inline test symbols that live below `#[cfg(test)]` inside production files
- symbols in dedicated test/example/bin files
- dead-code candidates that are textually referenced even when code-index
  failed to resolve the caller edge
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from collections import Counter, defaultdict
from dataclasses import dataclass
from pathlib import Path
from typing import Any


@dataclass(frozen=True)
class SymbolEntry:
    id: int
    file_path: str
    name: str
    kind: str
    line_start: int
    line_end: int
    visibility: str | None
    signature: str | None

    @classmethod
    def from_json(cls, raw: dict[str, Any]) -> "SymbolEntry":
        return cls(
            id=int(raw["id"]),
            file_path=str(raw["file_path"]),
            name=str(raw["name"]),
            kind=str(raw["kind"]),
            line_start=int(raw["line_start"]),
            line_end=int(raw["line_end"]),
            visibility=raw.get("visibility"),
            signature=raw.get("signature"),
        )


def run_command(args: list[str], cwd: Path) -> str:
    result = subprocess.run(
        args,
        cwd=cwd,
        check=True,
        capture_output=True,
        text=True,
    )
    return result.stdout


def parse_index_resolution(output: str) -> tuple[int, int, int, int] | None:
    match = re.search(
        r"Resolution:\s+(\d+)/(\d+)\s+resolved,\s+(\d+)\s+ambiguous,\s+(\d+)\s+unresolved",
        output,
    )
    if not match:
        return None
    return tuple(int(value) for value in match.groups())


def parse_status_counts(output: str) -> tuple[int, int, int] | None:
    match = re.search(
        r"Files:\s+(\d+),\s+Symbols:\s+(\d+),\s+References:\s+(\d+)", output
    )
    if not match:
        return None
    return tuple(int(value) for value in match.groups())


def load_entries(output: str) -> list[SymbolEntry]:
    return [SymbolEntry.from_json(item) for item in json.loads(output)]


def is_rust_file(path: str) -> bool:
    return path.endswith(".rs")


def is_test_path(path: str) -> bool:
    return (
        path.endswith("_tests.rs")
        or "/tests/" in path
        or "/bin/" in path
        or "/examples/" in path
    )


def relative_path(root: Path, file_path: str) -> str:
    try:
        return str(Path(file_path).resolve().relative_to(root.resolve()))
    except ValueError:
        return file_path


def first_cfg_test_line(path: Path) -> int | None:
    try:
        for index, line in enumerate(path.read_text().splitlines(), start=1):
            if "#[cfg(test)]" in line:
                return index
    except FileNotFoundError:
        return None
    return None


def is_inline_test_symbol(
    entry: SymbolEntry, root: Path, cfg_test_cache: dict[Path, int | None]
) -> bool:
    if is_test_path(entry.file_path):
        return True
    if entry.visibility is not None:
        return False
    path = Path(entry.file_path)
    marker = cfg_test_cache.setdefault(
        path, first_cfg_test_line(root / relative_path(root, entry.file_path))
    )
    return marker is not None and entry.line_start >= marker


def has_textual_reference(
    entry: SymbolEntry,
    function_hits: dict[str, list[tuple[Path, int]]],
    method_hits: dict[str, list[tuple[Path, int]]],
) -> bool:
    definition_path = Path(entry.file_path).resolve()
    hits = (
        method_hits.get(entry.name, [])
        if entry.kind == "method"
        else function_hits.get(entry.name, [])
    )
    for rust_file, line_number in hits:
        if rust_file == definition_path and line_number == entry.line_start:
            continue
        return True
    return False


def build_token_hits(
    file_text_cache: dict[Path, list[str]],
) -> tuple[dict[str, list[tuple[Path, int]]], dict[str, list[tuple[Path, int]]]]:
    function_hits: dict[str, list[tuple[Path, int]]] = defaultdict(list)
    method_hits: dict[str, list[tuple[Path, int]]] = defaultdict(list)
    function_pattern = re.compile(
        r"(?:^|[^.\w])(?:[A-Za-z_][A-Za-z0-9_]*::)*([A-Za-z_][A-Za-z0-9_]*)\s*\("
    )
    method_pattern = re.compile(r"\.([A-Za-z_][A-Za-z0-9_]*)\s*\(")
    for rust_file, lines in file_text_cache.items():
        for line_number, line in enumerate(lines, start=1):
            for match in function_pattern.finditer(line):
                function_hits[match.group(1)].append((rust_file, line_number))
            for match in method_pattern.finditer(line):
                method_hits[match.group(1)].append((rust_file, line_number))
    return function_hits, method_hits


def top_file_counts(
    entries: list[SymbolEntry], root: Path, limit: int = 10
) -> list[tuple[str, int]]:
    counts = Counter(relative_path(root, entry.file_path) for entry in entries)
    return counts.most_common(limit)


def public_dead_shortlist(
    entries: list[SymbolEntry], root: Path, limit: int = 10
) -> list[tuple[str, list[str]]]:
    grouped: dict[str, list[str]] = defaultdict(list)
    for entry in entries:
        rel = relative_path(root, entry.file_path)
        grouped[rel].append(entry.name)
    ranked = sorted(grouped.items(), key=lambda item: (-len(item[1]), item[0]))
    return [(path, names[:5]) for path, names in ranked[:limit]]


def render_report(
    root: Path,
    rust_file_count: int,
    indexed_files: int | None,
    indexed_symbols: int | None,
    indexed_refs: int | None,
    resolution: tuple[int, int, int, int] | None,
    raw_dead: list[SymbolEntry],
    raw_untested: list[SymbolEntry],
    filtered_dead: list[SymbolEntry],
    filtered_untested: list[SymbolEntry],
    textually_referenced_dead: list[SymbolEntry],
    public_dead: list[SymbolEntry],
) -> str:
    lines: list[str] = []
    lines.append("# Code-Index Rust Triage")
    lines.append("")
    lines.append("Generated by `python3 scripts/triage-code-index-rust.py`.")
    lines.append(
        "Uses a full `code-index index --full` refresh unless `--skip-index` is passed."
    )
    lines.append("")
    lines.append("## Summary")
    lines.append("")
    lines.append(f"- Rust files under `crates/`: `{rust_file_count}`")
    if indexed_files is not None:
        lines.append(f"- Indexed files: `{indexed_files}`")
    if indexed_symbols is not None:
        lines.append(f"- Symbols: `{indexed_symbols}`")
    if indexed_refs is not None:
        lines.append(f"- References: `{indexed_refs}`")
    if resolution is not None:
        resolved, total, ambiguous, unresolved = resolution
        lines.append(
            f"- Resolution: `{resolved} / {total}` resolved, `{ambiguous}` ambiguous, `{unresolved}` unresolved"
        )
    lines.append(f"- Raw `dead-code` output: `{len(raw_dead)}`")
    lines.append(f"- Raw `untested` output: `{len(raw_untested)}`")
    lines.append(
        f"- Filtered production `dead-code` candidates: `{len(filtered_dead)}`"
    )
    lines.append(
        f"- Filtered production `untested` candidates: `{len(filtered_untested)}`"
    )
    lines.append(
        f"- Production dead-code entries contradicted by textual references: `{len(textually_referenced_dead)}`"
    )
    lines.append("")
    lines.append("## Interpretation")
    lines.append("")
    lines.append(
        "The raw Rust `dead-code` output is still too noisy for deletion work. The filtered `untested` report is a better next-step shortlist."
    )
    lines.append("")

    if textually_referenced_dead:
        lines.append("## Contradicted Dead-Code Examples")
        lines.append("")
        for entry in textually_referenced_dead[:10]:
            rel = relative_path(root, entry.file_path)
            lines.append(f"- `{entry.name}` in `{rel}:{entry.line_start}`")
        lines.append("")

    lines.append("## Top Untested Production Hotspots")
    lines.append("")
    for path, count in top_file_counts(filtered_untested, root):
        lines.append(f"- `{path}` — `{count}`")
    lines.append("")

    lines.append("## Public Dead-Code Shortlist For Manual Review")
    lines.append("")
    for path, names in public_dead_shortlist(public_dead, root):
        sample = ", ".join(f"`{name}`" for name in names)
        lines.append(f"- `{path}` — {sample}")
    lines.append("")

    lines.append("## Recommendation")
    lines.append("")
    lines.append(
        "1. Use the filtered `untested` hotspots for the next server-focused test pass."
    )
    lines.append("2. Do not delete from the raw `dead-code` report.")
    lines.append(
        "3. Improve `code-index` Rust resolution before trusting dead-code counts for cleanup work."
    )
    lines.append("")
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("project_root", nargs="?", default=".")
    parser.add_argument("--skip-index", action="store_true")
    parser.add_argument("--output", default="CODE_INDEX_TRIAGE.md")
    args = parser.parse_args()

    root = Path(args.project_root).resolve()
    rust_files = sorted(root.glob("crates/**/*.rs"))
    rust_file_count = len(rust_files)
    file_text_cache = {
        path.resolve(): path.read_text().splitlines() for path in rust_files
    }
    function_hits, method_hits = build_token_hits(file_text_cache)

    index_output = ""
    if not args.skip_index:
        index_output = run_command(["code-index", "index", "--full"], root)
    status_output = run_command(["code-index", "status"], root)
    dead_output = run_command(["code-index", "dead-code"], root)
    untested_output = run_command(["code-index", "untested"], root)

    status_counts = parse_status_counts(status_output)
    resolution = parse_index_resolution(index_output)

    raw_dead = [
        entry for entry in load_entries(dead_output) if is_rust_file(entry.file_path)
    ]
    raw_untested = [
        entry
        for entry in load_entries(untested_output)
        if is_rust_file(entry.file_path)
    ]

    cfg_test_cache: dict[Path, int | None] = {}

    filtered_dead: list[SymbolEntry] = []
    textually_referenced_dead: list[SymbolEntry] = []
    public_dead: list[SymbolEntry] = []
    for entry in raw_dead:
        if is_inline_test_symbol(entry, root, cfg_test_cache):
            continue
        if is_test_path(entry.file_path):
            continue
        if entry.visibility in {"pub", "pub(crate)"} and entry.kind in {
            "function",
            "method",
        }:
            if has_textual_reference(entry, function_hits, method_hits):
                textually_referenced_dead.append(entry)
                continue
        filtered_dead.append(entry)
        if entry.visibility in {"pub", "pub(crate)"}:
            public_dead.append(entry)

    filtered_untested = [
        entry
        for entry in raw_untested
        if not is_inline_test_symbol(entry, root, cfg_test_cache)
        and not is_test_path(entry.file_path)
    ]

    indexed_files = indexed_symbols = indexed_refs = None
    if status_counts is not None:
        indexed_files, indexed_symbols, indexed_refs = status_counts

    report = render_report(
        root=root,
        rust_file_count=rust_file_count,
        indexed_files=indexed_files,
        indexed_symbols=indexed_symbols,
        indexed_refs=indexed_refs,
        resolution=resolution,
        raw_dead=raw_dead,
        raw_untested=raw_untested,
        filtered_dead=filtered_dead,
        filtered_untested=filtered_untested,
        textually_referenced_dead=textually_referenced_dead,
        public_dead=public_dead,
    )
    output_path = root / args.output
    output_path.write_text(report + "\n")
    print(f"Wrote {output_path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
