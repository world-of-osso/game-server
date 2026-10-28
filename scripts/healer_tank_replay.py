#!/usr/bin/env python3
"""Replay a recorded native intake against Holy and Protection Paladins."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parent.parent
AGENT_RUN = Path('/home/osso/Projects/world-of-osso/game-engine/scripts/agent/agent-run')
DRIVER = 'dungeon_simulator::replay_cli_tests::native_replay_cli_driver'
MARKER = 'HEALER_TANK_REPORT='


def run_replay(timeline, spec, executable):
    environment = os.environ | {'HEALER_TANK_TIMELINE': str(timeline), 'HEALER_TANK_SPEC': str(spec)}
    if executable:
        command = [str(executable), DRIVER, '--exact', '--ignored', '--nocapture']
    else:
        command = [str(AGENT_RUN), 'healer-tank-replay-cli', 'cargo', 'test', '--locked',
                   '-p', 'server-simulation', '--lib', DRIVER, '--', '--exact', '--ignored', '--nocapture']
    result = subprocess.run(command, cwd=ROOT, env=environment, text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(f'native spec {spec} replay failed:\n{result.stdout}\n{result.stderr}')
    lines = [line.split(MARKER, 1)[1] for line in result.stdout.splitlines() if MARKER in line]
    if len(lines) != 1:
        raise RuntimeError(f'native spec {spec} returned {len(lines)} reports')
    return json.loads(lines[0])


def number(value):
    return 'N/A' if value is None else f'{value:.2f}'


def print_healer(report):
    metrics = report['metrics']
    print('| Spec | Effective HPS | Raw HPS | Overheal % | First death (s) |')
    print('|---|---:|---:|---:|---:|')
    print(f'| {report["spec"]} | {number(metrics["effective_hps"])} | {number(metrics["raw_hps"])} | '
          f'{number(metrics["overheal_pct"])} | {number(metrics["first_death_seconds"])} |')
    print('\n| Spell ID | Effective HPS | Raw HPS |')
    print('|---|---:|---:|')
    for spell, values in metrics['spell_healing'].items():
        print(f'| {spell} | {number(values["effective_hps"])} | {number(values["raw_hps"])} |')
    print('\n| Time (s) | Mana | Max mana |')
    print('|---:|---:|---:|')
    for sample in metrics['mana_curve']:
        print(f'| {number(sample["time_seconds"])} | {number(sample["current"])} | {number(sample["max"])} |')
    print('\nCoverage gaps:', ', '.join(report['gaps']))


def print_tank(report):
    print('| Spec | Raw damage | Damage taken | Mitigation % | Self-heal/s | Absorb/s | Death (s) | Active mitigation % | 5s spike |')
    print('|---|---:|---:|---:|---:|---:|---:|---:|---:|')
    uptime = report['active_mitigation_uptime']
    print(f'| {report["spec"]} | {number(report["raw_damage"])} | {number(report["damage_taken"])} | '
          f'{number(report["mitigation_pct"])} | {number(report["self_heal_per_second"])} | '
          f'{number(report["absorb_per_second"])} | {number(report["time_to_death"])} | '
          f'{number(None if uptime is None else 100 * uptime)} | {number(report["largest_spike"])} |')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--timeline', type=Path, required=True, help='recorded native intake JSON')
    parser.add_argument('--spec', choices=('65', '66', 'all'), default='all')
    parser.add_argument('--executable', type=Path, help='existing server-simulation test executable')
    args = parser.parse_args()
    timeline = args.timeline.resolve(strict=True)
    executable = args.executable.resolve(strict=True) if args.executable else None
    for spec in (('65', '66') if args.spec == 'all' else (args.spec,)):
        report = run_replay(timeline, spec, executable)
        if spec == '65':
            print_healer(report)
        else:
            print_tank(report)
    return 0


if __name__ == '__main__':
    try:
        raise SystemExit(main())
    except (OSError, ValueError, KeyError, RuntimeError) as error:
        print(f'healer/tank replay: {error}', file=sys.stderr)
        raise SystemExit(1)
