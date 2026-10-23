"""Count-capped, deterministic selection of actual observed coordinates only."""
import copy
import sys
from pathlib import Path
import unittest

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
from follower_trash_density import apply_mdt_density, select_farthest_points


def point(index, xyz, entry=232369):
    return dict(entry=entry, marker_index=index, wow_xyz=xyz)


class TrashDensity(unittest.TestCase):
    def test_fixed_seed_farthest_points_keep_exact_observations_and_stable_ties(self):
        points = [point(0, [0, 0, 0]), point(1, [1, 0, 0]), point(2, [10, 0, 0]),
                  point(3, [0, 10, 0]), point(4, [5, 5, 0])]
        before = copy.deepcopy(points)
        selected = select_farthest_points(points, 3)
        self.assertEqual([p['marker_index'] for p in selected], [0, 2, 3])
        self.assertEqual([p['wow_xyz'] for p in selected], [[0, 0, 0], [10, 0, 0], [0, 10, 0]])
        self.assertEqual(select_farthest_points(list(reversed(points)), 3), selected)
        self.assertEqual(points, before)
        self.assertEqual(select_farthest_points(points, 0), [])
        self.assertEqual(select_farthest_points(points, 10), points)

    def test_world_height_participates_without_creating_new_positions(self):
        points = [point(0, [0, 0, 0]), point(1, [0, 0, 10]), point(2, [0, 0, 20])]
        self.assertEqual([p['marker_index'] for p in select_farthest_points(points, 2)], [0, 2])

    def test_per_type_caps_shortfalls_and_absent_mdt_types_are_explicit(self):
        points = [point(0, [0, 0, 0]), point(1, [10, 0, 0]), point(2, [20, 0, 0]),
                  point(0, [1, 1, 1], 234089), point(0, [2, 2, 2], 251861)]
        enemies = {
            232369: dict(entry=232369, name='Arcane Magister', clone_count=2, clone_count_source_line=78, clone_lines=[79, 85], is_boss=False),
            234089: dict(entry=234089, name='Animated Codex', clone_count=3, clone_count_source_line=160, clone_lines=[161, 167, 173], is_boss=False),
            234067: dict(entry=234067, name='Vigilant Librarian', clone_count=1, clone_count_source_line=1700, clone_lines=[1701], is_boss=False),
        }
        selected, report, gaps = apply_mdt_density(points, enemies, dict(path='pinned-MDT-fixture'))
        self.assertEqual(len([p for p in selected if p['entry'] == 232369]), 2)
        self.assertEqual(len([p for p in selected if p['entry'] == 234089]), 1)
        self.assertEqual(len([p for p in selected if p['entry'] == 251861]), 0)
        counts = {row['entry']: row for row in report['per_npc']}
        self.assertEqual((counts[234089]['selected_count'], counts[234089]['shortfall']), (1, 2))
        self.assertEqual((counts[234067]['selected_count'], counts[234067]['shortfall']), (0, 1))
        self.assertEqual([(g['entry'], g['status']) for g in gaps], [(251861, 'FOLLOWER_TRASH_MDT_TYPE_ABSENT')])
        self.assertEqual(len(report['excluded_points']), 2)
        self.assertEqual(report['seed'], 0)


if __name__ == '__main__':
    unittest.main()
