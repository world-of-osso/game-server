#!/usr/bin/env python3
"""Fail deployment if an existing navigation map would omit its portal graph."""
import argparse
from pathlib import Path

from ground_files import selected_map_sources


def check_graphs(nav_root: Path, selected_maps: set[int]) -> None:
    if not nav_root.is_dir():
        raise ValueError(f"navigation directory missing: {nav_root}")
    for directory in sorted(nav_root.iterdir()):
        if not directory.is_dir() or not any(directory.glob("*.nav")):
            continue
        try:
            map_id = int(directory.name)
        except ValueError as error:
            raise ValueError(f"invalid navigation map directory: {directory}") from error
        if map_id not in selected_maps:
            continue
        graph = directory / "portal-graph.json"
        if not graph.is_file() or graph.stat().st_size == 0:
            raise ValueError(f"navigation graph missing or empty: {graph}")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--world-db", type=Path, required=True)
    parser.add_argument("--nav-root", type=Path, required=True)
    args = parser.parse_args()
    selected = {map_id for map_id, _, _ in selected_map_sources(args.world_db)}
    check_graphs(args.nav_root, selected)


if __name__ == "__main__":
    main()
