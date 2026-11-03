#!/usr/bin/env python3
"""Check WCL versus native effective intake per recipient and 15-second window."""
import argparse
import collections
import json
import statistics
from pathlib import Path


def evaluate(trace, report):
    duration_ms = report['duration_seconds'] * 1000
    if duration_ms % 15000:
        raise ValueError('duration must contain complete 15-second windows')
    first_death = report['metrics']['first_death_seconds']
    cutoff_ms = min(duration_ms, int(first_death * 1000)) if first_death is not None else duration_ms
    fatal_hits = [hit['time_ms'] for hit in report['wcl_intake']['health_samples']
                  if hit['time_ms'] <= cutoff_ms and hit['native_hp_pct'] <= 0]
    if fatal_hits:
        cutoff_ms = min(cutoff_ms, min(fatal_hits))
    expected = collections.defaultdict(float)
    actual = collections.defaultdict(float)
    for hit in trace['hits']:
        if hit['time_ms'] < cutoff_ms:
            expected[hit['slot'], hit['time_ms'] // 15000] += (
                100 * (hit['amount'] + hit['absorbed']) / hit['max_hp']
            )
    for hit in report['wcl_intake']['health_samples']:
        if hit['time_ms'] < cutoff_ms:
            actual[hit['slot'], hit['time_ms'] // 15000] += (
                hit['native_loss_pct'] + hit['native_absorbed_pct']
            )
    rows = []
    for slot in range(5):
        for window in range((cutoff_ms + 14999) // 15000):
            wcl = expected[slot, window]
            native = actual[slot, window]
            error = 100 * (native / wcl - 1) if wcl else None
            passed = abs(error) <= 10 if error is not None else native == 0
            rows.append({'slot': slot, 'start_ms': window * 15000,
                         'end_ms': min((window + 1) * 15000, cutoff_ms),
                         'wcl_pct_hp': wcl, 'native_pct_hp': native,
                         'relative_error_pct': error, 'passed': passed})
    paired = [h for h in report['wcl_intake']['health_samples']
              if h['slot'] == 0 and h['wcl_hp_pct'] is not None]
    hp_mae = statistics.mean(abs(h['wcl_hp_pct'] - h['native_hp_pct']) for h in paired) if paired else None
    missed = report['wcl_intake']['events_on_dead_targets_not_replayed']
    return {'passed': bool(rows) and all(row['passed'] for row in rows) and (first_death is not None or missed == 0),
            'tolerance_pct': 10, 'cutoff_ms': cutoff_ms, 'rows': rows, 'dead_target_events': missed,
            'tank_hp_paired_samples': len(paired), 'tank_hp_mae_pp': hp_mae,
            'first_death_seconds': first_death}


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument('trace', type=Path)
    parser.add_argument('report', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    result = evaluate(json.loads(args.trace.read_text()), json.loads(args.report.read_text()))
    args.output.write_text(json.dumps(result, indent=2) + '\n')
    print(f"intake gate {'PASS' if result['passed'] else 'FAIL'}: "
          f"{sum(not row['passed'] for row in result['rows'])}/{len(result['rows'])} windows; "
          f"{result['dead_target_events']} dead-target events")
    raise SystemExit(0 if result['passed'] else 1)


if __name__ == '__main__':
    main()
