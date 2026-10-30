#!/usr/bin/env python3
"""Extract native follower topology and run one seed without a client."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

from freeze_follower_route import plan_route

ROOT = Path(__file__).resolve().parent.parent
NAVMESH = Path('/home/osso/Projects/world-of-osso/game-engine/data/navmesh')


def run_test(executable, name, env, log):
    command = [str(executable), name, '--exact', '--ignored', '--nocapture']
    with log.open('w') as output:
        result = subprocess.run(command, cwd=ROOT, env=env, stdout=output,
                                stderr=subprocess.STDOUT, check=False)
    if result.returncode:
        lines = log.read_text(errors='replace').splitlines()
        reason = next((line for line in lines if 'follower topology inputs:' in line),
                      f'{name} failed')
        raise RuntimeError(f'{reason}; see {log}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--test-executable', type=Path, required=True)
    parser.add_argument('--map', type=int, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--navmesh', type=Path, default=NAVMESH)
    parser.add_argument('--seed', type=int, default=1)
    args = parser.parse_args()
    executable = args.test_executable.resolve(strict=True)
    output = args.output.resolve()
    output.mkdir(parents=True, exist_ok=True)
    graph_path = output / f'map-{args.map}.json'
    env = os.environ | {'DUNGEON_SIMULATOR_MAP': str(args.map),
                        'DUNGEON_SIMULATOR_GRAPH': str(graph_path),
                        'DUNGEON_SIMULATOR_NAVMESH': str(args.navmesh.resolve()),
                        'DUNGEON_SIMULATOR_OUTPUT': str(output),
                        'DUNGEON_SIMULATOR_RUNS': '1',
                        'DUNGEON_SIMULATOR_SEED': str(args.seed)}
    prefix = 'dungeon_simulator::graph::'
    try:
        legacy = {34: 'stockade-34.json', 2515: 'azure-vault-2515.json'}
        if args.map in legacy:
            env['DUNGEON_SIMULATOR_GRAPH'] = str(ROOT / 'content/dungeons' / legacy[args.map])
        else:
            run_test(executable, prefix + 'extraction::extract_follower_topology', env, output / 'topology.log')
            run_test(executable, prefix + 'corridor_inputs::freeze_native_corridor_inputs', env, output / 'corridor.log')
            graph = json.loads(graph_path.read_text())
            inputs = json.loads((output / f'map-{args.map}-corridor-inputs.json').read_text())
            requirements = json.loads((output / f'map-{args.map}-requirements.json').read_text())[str(args.map)]
            graph.update(plan_route(graph, inputs, requirements))
            graph_path.write_text(json.dumps(graph, indent=2) + '\n')
        run_test(executable, 'dungeon_simulator::batch::run_stockade_batch', env, output / 'run.log')
    except (RuntimeError, OSError, ValueError, KeyError) as error:
        print(f'map{args.map}: {error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
