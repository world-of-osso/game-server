#!/usr/bin/env python3
"""Report per-map log observation, import and patrol evidence without inventing counts."""
import argparse
import json
from collections import Counter, defaultdict
from pathlib import Path


def coverage(populations, clusters, safe_clusters, preparation_gaps, imports):
    counts = defaultdict(Counter)
    patrols = defaultdict(list)
    for cluster in clusters:
        counts[cluster['map']]['positioned_clusters'] += 1
        counts[cluster['map']]['ambiguous_clusters'] += int(cluster['ambiguous_match'])
    for cluster in safe_clusters:
        if cluster['classification'] == 'stationary_candidate':
            counts[cluster['map']]['stationary_candidates'] += 1
        if cluster['classification'] == 'patrol_candidate' and len(cluster['observations']) >= 3:
            patrols[cluster['map']].append({'npc_id': cluster['npc_id'],
                'sample_count': len(cluster['observations']), 'runs': len(cluster['observations']),
                'spread_xy': cluster['spread_xy'], 'gap': cluster['gap']})
    report = []
    for population in populations:
        map_id = population['map']
        outcome = imports.get(str(map_id), {})
        import_gaps = Counter(gap['reason'] for gap in outcome.get('gaps', []))
        projected, absent = outcome.get('templates_from_tdb'), outcome.get('templates_absent')
        report.append({'map': map_id, 'runs': population['runs'], 'zone_segments_retained': population.get('zone_segments_retained', 0),
            'observed_npc_ids': len(population['npcs']),
            'tdb_rows': population['authored']['tdb_creature']['count'],
            'content_rows': population['authored']['content_creature']['count'],
            **counts[map_id], 'unique_spawns_proven': None,
            'unique_spawn_gap': 'CROSS_RUN_IDENTITY_UNPROVEN',
            'imported': outcome.get('imported'), 'already_present_nearby': outcome.get('already_present'),
            'template_ids_projected': projected, 'template_ids_projected_count': len(projected) if projected is not None else None,
            'template_ids_absent': absent, 'template_ids_absent_count': len(absent) if absent is not None else None,
            'preparation_gaps': preparation_gaps.get(str(map_id), {}),
            'import_gaps_by_reason': dict(import_gaps),
            'patrol_candidates': patrols[map_id], 'fitted_paths': 0})
    return report


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    for name in ('population', 'clusters', 'safe-clusters', 'preparation-gaps', 'import-report', 'output'):
        parser.add_argument('--' + name, required=True, type=Path)
    args = parser.parse_args()
    def load(path):
        with path.open(encoding='utf-8') as source:
            return json.load(source)
    result = coverage(load(args.population), load(args.clusters), load(args.safe_clusters),
                      load(args.preparation_gaps), load(args.import_report))
    args.output.write_text(json.dumps(result, indent=2) + '\n')


if __name__ == '__main__':
    main()
