import importlib.util
from pathlib import Path
import unittest

ROOT = Path(__file__).resolve().parents[2]
spec = importlib.util.spec_from_file_location('follower_utility_metrics', ROOT / 'scripts/follower_utility_metrics.py')
metrics = importlib.util.module_from_spec(spec)
spec.loader.exec_module(metrics)


class UtilityMetricsTests(unittest.TestCase):
    def test_native_control_damage_and_dummy_have_distinct_danger(self):
        danger = metrics.NativeDanger(ROOT / 'data/world.db')
        self.assertEqual(danger(118), 4)
        self.assertEqual(danger(133), 2)
        self.assertEqual(danger(5782), 1)

    def test_counts_real_log_contract_not_misses_disappearances_or_instant_casts(self):
        def event(kind, source, target, spell=None, interrupted=None):
            return dict(kind=kind, source=source, target=target, spell_id=spell, extra_spell_id=interrupted)
        events = [event('Summon', 1, 3, 30146), event('CastStart', 2, 1, 133),
                  event('Interrupt', 3, 2, 89766, 133), event('CastStart', 2, 1, 118),
                  event('CastSuccess', 2, 1, 118), event('CastSuccess', 2, 1, 133),
                  event('Death', 2, 1), event('Death', 1, 2)]
        report = dict(seed=7, status='wiped', performance=dict(combat_events=events),
                      samples=[dict(members=[dict(entity=1, exists=False)])],
                      terminal_units=[dict(guid=20375720, entity=2)])
        result = metrics.measure(report, metrics.NativeDanger(ROOT / 'data/world.db'))
        self.assertEqual(result['interrupts_landed'], 1)
        self.assertEqual(result['dangerous_casts_through'], 1)
        self.assertEqual(result['deaths'], 1)
        self.assertEqual(result['completed_dangerous_spells'], {118: 1})


if __name__ == '__main__':
    unittest.main()
