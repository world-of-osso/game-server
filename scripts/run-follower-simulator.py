#!/usr/bin/env python3
"""Extract native follower topology and run one seed without a client."""
import argparse
import json
import os
from pathlib import Path
import sqlite3
import subprocess
import sys

from freeze_follower_route import plan_route
from follower_encounter_actors import encounter_actor_bits

ROOT = Path(__file__).resolve().parent.parent
NAVMESH = Path('/home/osso/Projects/world-of-osso/game-engine/data/navmesh')


def read_catalog_level(database, map_id):
    with sqlite3.connect(f'file:{database}?mode=ro', uri=True) as db:
        row = db.execute('''SELECT t.MaxLevelSquish FROM follower_lfg_dungeons d
                            JOIN content_tuning t ON t.ID=d.content_tuning_id
                            WHERE d.map_id=? ORDER BY d.id LIMIT 1''', (map_id,)).fetchone()
    if row is None or not 1 <= row[0] <= 90:
        raise ValueError(f'map{map_id} missing valid catalog level')
    return row[0]


def write_blocked_report(output, map_id, seed, reason):
    report = {'map_id': map_id, 'seed': seed, 'status': 'blocked', 'detail': str(reason),
              'encounters_killed': 0, 'encounters_total': None, 'deaths': 0,
              'simulated_seconds': 0.0}
    if map_id == 2662:
        report['deck_population_unsourced'] = {
            'report': 'x3HzDRn6yVjqCbt8', 'fight': 7, 'replay_executed': False,
            'transport_start_ms': 86000, 'commander_death_ms': 162380,
            'radiant_light_ms': 166807,
        }
    (output / f'run-{seed}.json').write_text(json.dumps(report, indent=2) + '\n')


def run_test(executable, name, env, log):
    command = [str(executable), name, '--exact', '--ignored', '--nocapture']
    with log.open('w') as output:
        result = subprocess.run(command, cwd=ROOT, env=env, stdout=output,
                                stderr=subprocess.STDOUT, check=False)
    if result.returncode:
        lines = log.read_text(errors='replace').splitlines()
        reason = next((line for line in lines if 'follower topology inputs:' in line), None)
        if reason is None:
            reason = next((line for line in lines if 'called `Result::unwrap()` on an `Err`' in line),
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
    report_path = output / f'run-{args.seed}.json'
    report_path.unlink(missing_ok=True)
    try:
        legacy = {34: 'stockade-34.json', 2515: 'azure-vault-2515.json'}
        if args.map not in legacy and 'DUNGEON_SIMULATOR_LEVEL' not in env:
            env['DUNGEON_SIMULATOR_LEVEL'] = str(read_catalog_level(ROOT / 'data/world.db', args.map))
        print(f"map{args.map} seed={args.seed} level={env.get('DUNGEON_SIMULATOR_LEVEL', '32')}", flush=True)
        if args.map in legacy:
            env['DUNGEON_SIMULATOR_GRAPH'] = str(ROOT / 'content/dungeons' / legacy[args.map])
        else:
            run_test(executable, prefix + 'extraction::extract_follower_topology', env, output / 'topology.log')
            run_test(executable, prefix + 'corridor_inputs::freeze_native_corridor_inputs', env, output / 'corridor.log')
            graph = json.loads(graph_path.read_text())
            inputs = json.loads((output / f'map-{args.map}-corridor-inputs.json').read_text())
            requirements = json.loads((output / f'map-{args.map}-requirements.json').read_text())[str(args.map)]
            graph.update(plan_route(graph, inputs, requirements))
            with sqlite3.connect(f'file:{ROOT / "data/world.db"}?mode=ro', uri=True) as db:
                graph['follower_encounter_actor_bits'] = encounter_actor_bits(
                    graph, db, ROOT / 'data/db2/active/JournalEncounterCreature.csv')
            graph_path.write_text(json.dumps(graph, indent=2) + '\n')
        run_test(executable, 'dungeon_simulator::batch::run_stockade_batch', env, output / 'run.log')
    except (RuntimeError, OSError, ValueError, KeyError, sqlite3.Error) as error:
        if not report_path.exists():
            write_blocked_report(output, args.map, args.seed, error)
        print(f'map{args.map}: {error}', file=sys.stderr)
        return 1
    return 0


if __name__ == '__main__':
    raise SystemExit(main())
