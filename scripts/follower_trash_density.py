"""Deterministic count-capped selection; never create or average a position."""
from collections import defaultdict

DENSITY_POLICY = 'FOLLOWER_TRASH_DENSITY_FROM_MDT_COUNT'


def marker_key(point):
    return point['entry'], point['marker_index']


def squared_distance(left, right):
    return sum((a - b) ** 2 for a, b in zip(left['wow_xyz'], right['wow_xyz']))


def select_farthest_points(points, count):
    if count < 0:
        raise ValueError('MDT clone count cannot be negative')
    ordered = sorted(points, key=marker_key)
    count = min(count, len(ordered))
    if count == 0:
        return []
    if count == len(ordered):
        return ordered
    selected = {0}
    nearest = [squared_distance(point, ordered[0]) for point in ordered]
    while len(selected) < count:
        index = max((i for i in range(len(ordered)) if i not in selected), key=lambda i: (nearest[i], -i))
        selected.add(index)
        for i, point in enumerate(ordered):
            nearest[i] = min(nearest[i], squared_distance(point, ordered[index]))
    return [point for i, point in enumerate(ordered) if i in selected]


def apply_mdt_density(points, enemies, source):
    by_entry = defaultdict(list)
    for point in points:
        by_entry[point['entry']].append(point)
    selected_keys, reports, gaps = set(), [], []
    for entry, enemy in sorted(enemies.items()):
        if enemy['is_boss']:
            continue
        available = by_entry.get(entry, [])
        selected = select_farthest_points(available, enemy['clone_count'])
        selected_keys.update(marker_key(point) for point in selected)
        reports.append(dict(entry=entry, mdt_enemy=enemy, qualified_count=len(available),
                            selected_count=len(selected), shortfall=max(0, enemy['clone_count'] - len(available))))
    for entry, candidates in sorted(by_entry.items()):
        if entry not in enemies or enemies[entry]['is_boss']:
            gaps.append(dict(entry=entry, status='FOLLOWER_TRASH_MDT_TYPE_ABSENT', qualified_count=len(candidates), selected_count=0))
    selected = [point for point in points if marker_key(point) in selected_keys]
    excluded = [point for point in points if marker_key(point) not in selected_keys]
    report = dict(policy=DENSITY_POLICY, method='GREEDY_FARTHEST_POINT', seed=0,
                  seed_definition='index0 in immutable(entry,marker_index) order; tie chooses lowest such key',
                  metric='squared Euclidean WorldXYZ distance in yards; no coordinate averaging',
                  source=source, per_npc=reports, excluded_points=excluded,
                  qualified_count=len(points), selected_count=len(selected), excluded_count=len(excluded))
    return selected, report, gaps
