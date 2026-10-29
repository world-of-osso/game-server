#!/usr/bin/env python3
"""Replay SimC tank-dummy pressure or a recorded native intake."""
import argparse
from concurrent.futures import ThreadPoolExecutor
from datetime import datetime
import json
import math
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


def run_replay(timeline, spec, source, duration, seed, profile_dir, executable, external_healing, mode='tank', party_trace=None, pressure_scale=1.0):
    environment = os.environ | {
        'HEALER_TANK_TIMELINE': str(timeline), 'HEALER_TANK_SPEC': str(spec),
        'HEALER_TANK_SOURCE': source, 'HEALER_TANK_MODE': mode, 'HEALER_TANK_DURATION': str(duration),
        'HEALER_TANK_SEED': str(seed), 'DPS_PARITY_WORLD_DB': str(WORLD_DB),
        'DPS_PARITY_OUTPUT': str(profile_dir), 'DPS_PARITY_ACTOR': 'player',
        'HEALER_TANK_EXTERNAL_HEALING': '1' if external_healing else '0',
        'HEALER_TANK_PRESSURE_SCALE': str(pressure_scale),
    }
    if party_trace is not None and mode == 'healer':
        environment['HEALER_TANK_PARTY_TRACE'] = str(party_trace)
    if executable:
        command = [str(executable), DRIVER, '--exact', '--ignored', '--nocapture']
    else:
        command = [str(AGENT_RUN), 'tank-pressure', 'cargo', 'test', '--locked',
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
        print('SimC dummy debuffs:', ', '.join(f'{buff["name"]} ({buff["spell_id"]})' for buff in report['dummy_debuffs']) or 'none')
        print('SimC dummy bleeding:', 'enabled' if report['dummy_bleeding'] else 'disabled')
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
    command = [str(AGENT_RUN), 'tank-pressure', 'cargo', 'test', '--locked',
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


def summarize(values):
    values = [value for value in values if value is not None]
    if not values:
        return None
    return {'mean': sum(values) / len(values), 'min': min(values), 'max': max(values)}


def format_range(values):
    result = summarize(values)
    if result is None:
        return 'N/A'
    return f'{result["mean"]:.2f} ({result["min"]:.2f}–{result["max"]:.2f})'


def print_tank_aggregate(reports, seeds):
    print(f'Tank {reports[0]["spec"]}; {len(seeds)} seeds {seeds[0]}–{seeds[-1]}; one canonical SimC trace.')
    print('External dummy healing:', 'enabled' if reports[0]['external_healing'] else 'disabled')
    print('Values are mean (min–max); absent spell casts and auras count as zero.')
    print('\n| Spell ID | Casts | Aura uptime % |')
    print('|---:|---:|---:|')
    spells = set().union(*(set(r['tank_casts']) | set(r['aura_uptime']) for r in reports))
    for spell in sorted(spells, key=int):
        casts = [r['tank_casts'].get(spell, 0) for r in reports]
        uptime = [100 * r['aura_uptime'].get(spell, 0) for r in reports]
        print(f'| {spell} | {format_range(casts)} | {format_range(uptime)} |')
    print('\n| Spec | Mitigation % | Active mitigation % | Deaths |')
    print('|---|---:|---:|---:|')
    mitigation = format_range([r['mitigation_pct'] for r in reports])
    uptime = format_range([None if r['active_mitigation_uptime'] is None else 100 * r['active_mitigation_uptime'] for r in reports])
    deaths = sum(r['time_to_death'] is not None for r in reports)
    print(f'| {reports[0]["spec"]} | {mitigation} | {uptime} | {deaths}/{len(reports)} |')


def run_all(trace_dir, executable, output_dir, jobs, profile_dir=None, party_trace=None, seeds=(70448,)):
    output_dir.mkdir(parents=True, exist_ok=True)
    def run(item):
        mode, spec, healed, seed = item
        profiles = profile_dir or ALL_PROFILES
        trace_spec = '66' if mode == 'healer' else spec
        timeline = (trace_dir / f'{trace_spec}.simc-trace.log').resolve(strict=True)
        report = run_replay(timeline, spec, 'simc', 300, seed, profiles, executable, healed, mode, party_trace)
        suffix = 'healed' if healed else 'unhealed'
        seed_suffix = f'-seed{seed}' if mode == 'tank' and len(seeds) > 1 else ''
        path = output_dir / f'{mode}-{spec}-{suffix}{seed_suffix}.json'
        path.write_text(json.dumps(report, indent=2) + '\n')
        return item, report

    tasks = [('healer', spec, False, 70448) for spec in HEALERS]
    tasks += [('tank', spec, healed, seed) for spec in TANKS for healed in (True, False) for seed in seeds]
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        reports = dict(pool.map(run, tasks))
    print('| Healer | Effective HPS | Raw HPS | Overheal % | Mana min | Mana end | First death (s) | Tank min health | Idle % |')
    print('|---|---:|---:|---:|---:|---:|---:|---:|---:|')
    for spec in HEALERS:
        m = reports[('healer', spec, False, 70448)]['metrics']
        print(f'| {spec} | {number(m["effective_hps"])} | {number(m["raw_hps"])} | '
              f'{number(m["overheal_pct"])} | {number(m["mana_min"])} | '
              f'{number(m["mana_curve"][-1]["current"])} | {number(m["first_death_seconds"])} | '
              f'{number(m["tank_health_min"])} | {number(m["idle_gcd_pct"])} |')
    if len(seeds) > 1:
        print(f'\nTank aggregates: {len(seeds)} seeds {seeds[0]}–{seeds[-1]}; values are mean (min–max). Healers use seed 70448.')
        print('| Tank | Healed mitigation % | Healed active mitigation % | Healed deaths | No-healer deaths |')
        print('|---|---:|---:|---:|---:|')
        for spec in TANKS:
            healed = [reports[('tank', spec, True, seed)] for seed in seeds]
            unhealed = [reports[('tank', spec, False, seed)] for seed in seeds]
            mitigation = format_range([r['mitigation_pct'] for r in healed])
            uptime = format_range([None if r['active_mitigation_uptime'] is None else 100 * r['active_mitigation_uptime'] for r in healed])
            deaths = lambda rows: sum(r['time_to_death'] is not None for r in rows)
            print(f'| {spec} | {mitigation} | {uptime} | {deaths(healed)}/{len(seeds)} | {deaths(unhealed)}/{len(seeds)} |')
        for spec in TANKS:
            for healed in (True, False):
                print_tank_aggregate([reports[('tank', spec, healed, seed)] for seed in seeds], seeds)
        print(f'\nDetails: {output_dir}')
        return
    print('\n| Tank | Mitigation % | Self-heal/s | Absorb/s | No-healer death (s) | Active mitigation % | 5s spike |')
    print('|---|---:|---:|---:|---:|---:|---:|')
    for spec in TANKS:
        healed = reports[('tank', spec, True, seeds[0])]
        unhealed = reports[('tank', spec, False, seeds[0])]
        uptime = healed['active_mitigation_uptime']
        print(f'| {spec} | {number(healed["mitigation_pct"])} | {number(healed["self_heal_per_second"])} | '
              f'{number(healed["absorb_per_second"])} | {number(unhealed["time_to_death"])} | '
              f'{number(None if uptime is None else uptime * 100)} | {number(healed["largest_spike"])} |')
    print(f'\nDetails: {output_dir}')


PRESSURE_GRID = tuple(1.0 + 0.25 * step for step in range(17))


def run_pressure_sweep(trace_dir, executable, output_dir, jobs, profiles, seeds):
    """Replay every tank at every grid scale and seed; no bisection, because the
    survivor count is not guaranteed to fall monotonically with scale."""
    output_dir.mkdir(parents=True, exist_ok=True)

    def replay(item):
        spec, scale, seed = item
        timeline = (trace_dir / f'{spec}.simc-trace.log').resolve(strict=True)
        report = run_replay(timeline, spec, 'simc', 300, seed, profiles, executable, False, pressure_scale=scale)
        (output_dir / f'tank-{spec}-scale{scale:.2f}-seed{seed}.json').write_text(json.dumps(report, indent=2) + '\n')
        return item, report

    tasks = [(spec, scale, seed) for spec in TANKS for scale in PRESSURE_GRID for seed in seeds]
    with ThreadPoolExecutor(max_workers=jobs) as pool:
        reports = dict(pool.map(replay, tasks))
    grid = {spec: [summarize_pressure(scale, [reports[(spec, scale, seed)] for seed in seeds])
                   for scale in PRESSURE_GRID] for spec in TANKS}
    results = []
    for spec in TANKS:
        first_failure = next((index for index, cell in enumerate(grid[spec]) if 2 * cell['survivors'] < len(seeds)),
                             len(PRESSURE_GRID))
        survival_scale = PRESSURE_GRID[first_failure - 1] if first_failure else None
        baseline = [reports[(spec, 1.0, seed)] for seed in seeds]
        net = sum(r['damage_taken'] / r['duration_seconds'] - r['self_heal_per_second'] for r in baseline) / len(baseline)
        results.append({'spec': spec, 'survival_scale': survival_scale, 'net_damage_per_second': net})
    results.sort(key=lambda row: row['survival_scale'] or 0.0, reverse=True)
    summary = {'seeds': list(seeds), 'scales': list(PRESSURE_GRID), 'grid': grid, 'results': results}
    (output_dir / 'summary.json').write_text(json.dumps(summary, indent=2) + '\n')
    print('| Scale | ' + ' | '.join(TANKS) + ' |')
    print('|---:|' + '---:|' * len(TANKS))
    for index, scale in enumerate(PRESSURE_GRID):
        cells = [f'{grid[spec][index]["survivors"]}/{len(seeds)}, {grid[spec][index]["mean_time"]:.0f} s' for spec in TANKS]
        print(f'| {scale:.2f} | ' + ' | '.join(cells) + ' |')
    print('\n| Tank | Survival scale | Net damage/s at 1.0× |')
    print('|---|---:|---:|')
    for row in results:
        print(f'| {row["spec"]} | {number(row["survival_scale"])} | {row["net_damage_per_second"]:.2f} |')


def summarize_pressure(scale, reports):
    survivors = sum(report['time_to_death'] is None for report in reports)
    mean_time = sum(report['time_to_death'] or 300.0 for report in reports) / len(reports)
    return {'scale': scale, 'survivors': survivors, 'mean_time': mean_time}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--timeline', type=Path, help='SimC debug trace or recorded intake JSON')
    parser.add_argument('--party-trace', type=Path, help='SimC raid-event party damage trace for healer replays')
    parser.add_argument('--all', action='store_true', help='run all healer and tank benchmarks')
    parser.add_argument('--pressure-scale', type=float, default=1.0, help='multiply SimC tank incoming raw damage')
    parser.add_argument('--pressure-sweep', action='store_true', help='rank all tanks over ten no-healer seeds')
    parser.add_argument('--trace-dir', type=Path, default=TRACE_DIR, help='directory of per-tank --all traces')
    parser.add_argument('--output-dir', type=Path, help='directory for --all detail JSON reports')
    parser.add_argument('--jobs', type=int, default=min(4, len(os.sched_getaffinity(0))), help='parallel --all replays')
    parser.add_argument('--source', choices=('simc', 'recorded'), default='simc')
    parser.add_argument('--mode', choices=('tank', 'healer'), default='tank')
    parser.add_argument('--spec', default='66')
    parser.add_argument('--duration', type=int, default=300, help='SimC replay duration in seconds')
    parser.add_argument('--seed', type=int, default=70448)
    parser.add_argument('--seeds', type=int, default=1, help='number of consecutive tank seeds starting at --seed')
    parser.add_argument('--simc-tank-heal', action='store_true', help='replay the SimC dummy 75000 heal every 5 seconds')
    parser.add_argument('--profile-dir', type=Path, help='directory containing profile and trait inputs')
    parser.add_argument('--executable', type=Path, help='existing server-simulation test executable')
    args = parser.parse_args()
    if not math.isfinite(args.pressure_scale) or args.pressure_scale <= 0:
        parser.error('--pressure-scale must be positive and finite')
    if args.pressure_sweep:
        if (args.all or args.timeline or args.simc_tank_heal or args.pressure_scale != 1.0
                or args.jobs <= 0 or args.seed < 0 or args.source != 'simc' or args.mode != 'tank'
                or args.party_trace):
            parser.error('--pressure-sweep requires SimC tank traces, no healer, a nonnegative seed, and positive jobs')
        executable = args.executable.resolve(strict=True) if args.executable else build_replay_executable()
        output = args.output_dir or TRACE_DIR / datetime.now().strftime('pressure-sweep-%Y-%m-%d-%H%M%S')
        profiles = (args.profile_dir or ALL_PROFILES).resolve(strict=True)
        run_pressure_sweep(args.trace_dir.resolve(strict=True), executable, output, args.jobs, profiles,
                           tuple(range(args.seed, args.seed + 10)))
        return 0
    if args.pressure_scale != 1.0 and (args.all or args.mode != 'tank' or args.source != 'simc'):
        parser.error('--pressure-scale requires a single SimC tank replay')
    if args.seeds <= 0 or args.seed < 0:
        parser.error('--seeds must be positive and --seed must be nonnegative')
    seeds = tuple(range(args.seed, args.seed + args.seeds))
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
        run_all(trace_dir, executable, output, args.jobs, profiles, party_trace, seeds)
        return 0
    if args.seeds > 1 and (args.mode != 'tank' or args.source != 'simc'):
        parser.error('--seeds greater than one requires SimC tank mode')
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
        reports = [run_replay(timeline, spec, args.source, args.duration, seed, profile_dir, executable, args.simc_tank_heal, args.mode, args.party_trace.resolve(strict=True) if args.party_trace else None, args.pressure_scale) for seed in seeds]
        report = reports[0]
        if len(seeds) > 1:
            print_tank_aggregate(reports, seeds)
        elif args.mode == 'healer' or (args.source == 'recorded' and spec == '65'):
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
