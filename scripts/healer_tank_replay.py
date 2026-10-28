#!/usr/bin/env python3
"""Replay SimC tank-dummy pressure or a recorded native intake."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime
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
TRACE_DIR = Path('/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/healer-tank-replay')
ALL_PROFILES = PROFILE_DIR / 'healer-tank-all-inputs'
HEALERS = ('65', '105', '256', '257', '264', '270', '1468')
TANKS = ('66', '73', '104', '250', '268', '581')


def run_replay(timeline, spec, source, duration, seed, profile_dir, executable, external_healing, mode='tank', party_trace=None):
    environment = os.environ | {
        'HEALER_TANK_TIMELINE': str(timeline), 'HEALER_TANK_SPEC': str(spec),
        'HEALER_TANK_SOURCE': source, 'HEALER_TANK_MODE': mode, 'HEALER_TANK_DURATION': str(duration),
        'HEALER_TANK_SEED': str(seed), 'DPS_PARITY_WORLD_DB': str(WORLD_DB),
        'DPS_PARITY_OUTPUT': str(profile_dir), 'DPS_PARITY_ACTOR': 'player',
        'HEALER_TANK_EXTERNAL_HEALING': '1' if external_healing else '0',
    }
    if party_trace is not None and mode == 'healer':
        environment['HEALER_TANK_PARTY_TRACE'] = str(party_trace)
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
    if report.get('timeline') == 'simc_tank_dummy':
        print('\n| Minimum mana | Tank DTPS | Minimum tank health |')
        print('|---:|---:|---:|')
        print(f'| {number(metrics["mana_min"])} | {number(metrics["tank_dtps"])} | {number(metrics["tank_health_min"])} |')
    print('\n| Spell ID | Effective HPS | Raw HPS |')
    print('|---|---:|---:|')
    for spell, values in metrics['spell_healing'].items():
        print(f'| {spell} | {number(values["effective_hps"])} | {number(values["raw_hps"])} |')
    print('\n| Time (s) | Mana | Max mana |')
    print('|---:|---:|---:|')
    for sample in metrics['mana_curve']:
        print(f'| {number(sample["time_seconds"])} | {number(sample["current"])} | {number(sample["max"])} |')
    if report.get('timeline') != 'simc_tank_dummy':
        print('\nCoverage gaps:', ', '.join(report['gaps']))


def print_tank(report):
    if report.get('timeline') == 'simc_tank_dummy':
        print(f'SimC tank dummy, MID2 level {report["level"]}; rotation: {report["rotation_status"]}.')
        print('External dummy healing:', 'enabled' if report['external_healing'] else 'disabled')
        print('SimC Bloodlust start (s):', number(report['bloodlust_start_seconds']))
        print('SimC raid buffs:', ', '.join(f'{buff["name"]} ({buff["spell_id"]})' for buff in report['raid_buffs']) or 'none')
        print('\n| Spell ID | Casts | Periodic damage ticks | Raw self-heal | Effective self-heal | Consumed absorb | Aura uptime % |')
        print('|---|---:|---:|---:|---:|---:|---:|')
        spells = (set(report['tank_casts']) | set(report['periodic_damage_ticks_by_spell']) |
                  set(report['raw_self_healing_by_spell']) | set(report['self_healing_by_spell']) |
                  set(report['absorbs_by_spell']) | set(report['aura_uptime']))
        for spell in sorted(spells, key=int):
            print(f'| {spell} | {report["tank_casts"].get(spell, 0)} | '
                  f'{report["periodic_damage_ticks_by_spell"].get(spell, 0)} | '
                  f'{number(report["raw_self_healing_by_spell"].get(spell, 0))} | '
                  f'{number(report["self_healing_by_spell"].get(spell, 0))} | '
                  f'{number(report["absorbs_by_spell"].get(spell, 0))} | '
                  f'{number(100 * report["aura_uptime"].get(spell, 0))} |')
        print('\n| Energize spell ID | Attempted | Effective |')
        print('|---:|---:|---:|')
        for spell, values in sorted(report['energize_by_spell'].items(), key=lambda item: int(item[0])):
            print(f'| {spell} | {values["attempted"]} | {values["effective"]} |')
        print('\n| Resource counter | Count |')
        print('|---|---:|')
        for name, count in report['resource_counters'].items():
            print(f'| {name} | {count} |')
        print('\n| Target debuff ID | Uptime % |')
        print('|---:|---:|')
        for spell, uptime in report['target_debuff_uptime'].items():
            print(f'| {spell} | {number(100 * uptime)} |')
    print('| Spec | Raw damage | Damage taken | Mitigation % | Self-heal/s | Absorb/s | Death (s) | Active mitigation % | 5s spike |')
    print('|---|---:|---:|---:|---:|---:|---:|---:|---:|')
    uptime = report['active_mitigation_uptime']
    print(f'| {report["spec"]} | {number(report["raw_damage"])} | {number(report["damage_taken"])} | '
          f'{number(report["mitigation_pct"])} | {number(report["self_heal_per_second"])} | '
          f'{number(report["absorb_per_second"])} | {number(report["time_to_death"])} | '
          f'{number(None if uptime is None else 100 * uptime)} | {number(report["largest_spike"])} |')


def build_replay_executable():
    command = [str(AGENT_RUN), 'healer-tank-replay-cli', 'cargo', 'test', '--locked',
               '-p', 'server-simulation', '--lib', '--no-run', '--message-format=json']
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True)
    if result.returncode:
        raise RuntimeError(f'replay executable build failed:\n{result.stderr}')
    artifacts = [json.loads(line) for line in result.stdout.splitlines() if line.startswith('{')]
    executables = [item['executable'] for item in artifacts
                   if item.get('reason') == 'compiler-artifact' and item.get('executable')
                   and item.get('profile', {}).get('test') and item['target']['name'] == 'server_simulation']
    if len(executables) != 1:
        raise RuntimeError(f'expected one replay executable, found {len(executables)}')
    return Path(executables[0])


def run_all(trace_dir, executable, output_dir, jobs, profile_dir=None, party_trace=None):
    output_dir.mkdir(parents=True, exist_ok=True)
    def run(item):
        mode, spec, healed = item
        profiles = profile_dir or ALL_PROFILES
        trace_spec = '66' if mode == 'healer' else spec
        timeline = (trace_dir / f'{trace_spec}.simc-trace.log').resolve(strict=True)
        report = run_replay(timeline, spec, 'simc', 300, 70448, profiles, executable, healed, mode, party_trace)
        suffix = 'healed' if healed else 'unhealed'
        path = output_dir / f'{mode}-{spec}-{suffix}.json'
        path.write_text(json.dumps(report, indent=2) + '\n')
        return item, report

    tasks = [('healer', spec, False) for spec in HEALERS]
    tasks += [('tank', spec, healed) for spec in TANKS for healed in (True, False)]
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        reports = dict(pool.map(run, tasks))
    print('| Healer | Effective HPS | Raw HPS | Overheal % | Mana min | Mana end | First death (s) | Tank min health | Idle % |')
    print('|---|---:|---:|---:|---:|---:|---:|---:|---:|')
    for spec in HEALERS:
        m = reports[('healer', spec, False)]['metrics']
        print(f'| {spec} | {number(m["effective_hps"])} | {number(m["raw_hps"])} | '
              f'{number(m["overheal_pct"])} | {number(m["mana_min"])} | '
              f'{number(m["mana_curve"][-1]["current"])} | {number(m["first_death_seconds"])} | '
              f'{number(m["tank_health_min"])} | {number(m["idle_gcd_pct"])} |')
    print('\n| Tank | Mitigation % | Self-heal/s | Absorb/s | No-healer death (s) | Active mitigation % | 5s spike |')
    print('|---|---:|---:|---:|---:|---:|---:|')
    for spec in TANKS:
        healed = reports[('tank', spec, True)]
        unhealed = reports[('tank', spec, False)]
        uptime = healed['active_mitigation_uptime']
        print(f'| {spec} | {number(healed["mitigation_pct"])} | {number(healed["self_heal_per_second"])} | '
              f'{number(healed["absorb_per_second"])} | {number(unhealed["time_to_death"])} | '
              f'{number(None if uptime is None else uptime * 100)} | {number(healed["largest_spike"])} |')
    print(f'\nDetails: {output_dir}')


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--timeline', type=Path, help='SimC debug trace or recorded intake JSON')
    parser.add_argument('--party-trace', type=Path, help='SimC raid-event party damage trace for healer replays')
    parser.add_argument('--all', action='store_true', help='run all healer and tank benchmarks')
    parser.add_argument('--trace-dir', type=Path, default=TRACE_DIR, help='directory of per-tank --all traces')
    parser.add_argument('--output-dir', type=Path, help='directory for --all detail JSON reports')
    parser.add_argument('--jobs', type=int, default=min(4, len(os.sched_getaffinity(0))), help='parallel --all replays')
    parser.add_argument('--source', choices=('simc', 'recorded'), default='simc')
    parser.add_argument('--mode', choices=('tank', 'healer'), default='tank')
    parser.add_argument('--spec', default='66')
    parser.add_argument('--duration', type=int, default=300, help='SimC replay duration in seconds')
    parser.add_argument('--seed', type=int, default=70448)
    parser.add_argument('--simc-tank-heal', action='store_true', help='replay the SimC dummy 75000 heal every 5 seconds')
    parser.add_argument('--profile-dir', type=Path, help='directory containing profile and trait inputs')
    parser.add_argument('--executable', type=Path, help='existing server-simulation test executable')
    args = parser.parse_args()
    if args.all:
        if args.jobs <= 0:
            parser.error('--jobs must be positive')
        if args.timeline:
            parser.error('--all uses per-tank traces from --trace-dir, not --timeline')
        trace_dir = args.trace_dir.resolve(strict=True)
        executable = args.executable.resolve(strict=True) if args.executable else build_replay_executable()
        output = args.output_dir or TRACE_DIR / datetime.now().strftime('all-%Y-%m-%d-%H%M%S')
        profiles = args.profile_dir.resolve(strict=True) if args.profile_dir else None
        party_trace = args.party_trace.resolve(strict=True) if args.party_trace else None
        run_all(trace_dir, executable, output, args.jobs, profiles, party_trace)
        return 0
    if args.party_trace and (args.mode != 'healer' or args.source != 'simc'):
        parser.error('--party-trace requires SimC healer mode or --all')
    if args.timeline is None:
        parser.error('--timeline is required unless --all is used')
    if args.simc_tank_heal and args.source != 'simc':
        parser.error('--simc-tank-heal requires --source simc')
    if args.mode == 'healer' and (args.source != 'simc' or args.simc_tank_heal or not args.spec.isdigit()):
        parser.error('healer mode requires a numeric spec, SimC source, and no external dummy healing')
    if args.mode == 'tank' and args.source == 'simc' and args.spec not in ('66', '73', '104', '250', '268', '581', 'all'):
        parser.error('unsupported SimC tank specialization')
    if args.source == 'recorded' and args.spec not in ('65', '66', 'all'):
        parser.error('recorded intake playback currently supports specs 65 and 66')
    if args.duration <= 0 or args.seed < 0:
        parser.error('duration must be positive and seed must be nonnegative')
    timeline = args.timeline.resolve(strict=True)
    profile_dir = (args.profile_dir or PROFILE_DIR).resolve(strict=True)
    executable = args.executable.resolve(strict=True) if args.executable else None
    all_specs = ('66', '73', '104', '250', '268', '581') if args.source == 'simc' else ('65', '66')
    for spec in (all_specs if args.spec == 'all' else (args.spec,)):
        report = run_replay(timeline, spec, args.source, args.duration, args.seed, profile_dir, executable, args.simc_tank_heal, args.mode, args.party_trace.resolve(strict=True) if args.party_trace else None)
        if args.mode == 'healer' or (args.source == 'recorded' and spec == '65'):
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
