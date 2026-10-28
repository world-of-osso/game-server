#!/usr/bin/env python3
"""Replay SimC tank-dummy pressure or a recorded native intake."""
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
WORLD_DB = Path('/home/osso/Projects/world-of-osso/game-server/data/world.db')
PROFILE_DIR = Path('/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/dps-parity-2026-10-21')


def run_replay(timeline, spec, source, duration, seed, profile_dir, executable):
    environment = os.environ | {
        'HEALER_TANK_TIMELINE': str(timeline), 'HEALER_TANK_SPEC': str(spec),
        'HEALER_TANK_SOURCE': source, 'HEALER_TANK_DURATION': str(duration),
        'HEALER_TANK_SEED': str(seed), 'DPS_PARITY_WORLD_DB': str(WORLD_DB),
        'DPS_PARITY_OUTPUT': str(profile_dir),
    }
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
    if report.get('timeline') == 'simc_tank_dummy':
        print(f'SimC tank dummy, MID2 level {report["level"]}; rotation: {report["rotation_status"]}.')
    print('| Spec | Raw damage | Damage taken | Mitigation % | Self-heal/s | Absorb/s | Death (s) | Active mitigation % | 5s spike |')
    print('|---|---:|---:|---:|---:|---:|---:|---:|---:|')
    uptime = report['active_mitigation_uptime']
    print(f'| {report["spec"]} | {number(report["raw_damage"])} | {number(report["damage_taken"])} | '
          f'{number(report["mitigation_pct"])} | {number(report["self_heal_per_second"])} | '
          f'{number(report["absorb_per_second"])} | {number(report["time_to_death"])} | '
          f'{number(None if uptime is None else 100 * uptime)} | {number(report["largest_spike"])} |')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--timeline', type=Path, required=True, help='SimC debug trace or recorded intake JSON')
    parser.add_argument('--source', choices=('simc', 'recorded'), default='simc')
    parser.add_argument('--spec', choices=('65', '66', 'all'), default='66')
    parser.add_argument('--duration', type=int, default=300, help='SimC replay duration in seconds')
    parser.add_argument('--seed', type=int, default=70448)
    parser.add_argument('--profile-dir', type=Path, default=PROFILE_DIR)
    parser.add_argument('--executable', type=Path, help='existing server-simulation test executable')
    args = parser.parse_args()
    if args.source == 'simc' and args.spec != '66':
        parser.error('SimC timeline playback currently supports only spec 66')
    if args.duration <= 0 or args.seed < 0:
        parser.error('duration must be positive and seed must be nonnegative')
    timeline = args.timeline.resolve(strict=True)
    profile_dir = args.profile_dir.resolve(strict=True)
    executable = args.executable.resolve(strict=True) if args.executable else None
    for spec in (('65', '66') if args.spec == 'all' else (args.spec,)):
        report = run_replay(timeline, spec, args.source, args.duration, args.seed, profile_dir, executable)
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
