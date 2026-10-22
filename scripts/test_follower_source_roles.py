"""Native source books must publish damaging scripted roots as combat actions."""
from contextlib import closing
import json
from pathlib import Path
import sqlite3
import unittest

from scripts.follower_spec_kits import kit_rows

ROOT = Path(__file__).resolve().parents[1]
EXPORT = Path('/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/follower-kits-2026-10-18/native-source-v2.json')


def native_rows(spec):
    source = json.loads(EXPORT.read_text())
    selected = [row for row in source['loadouts'] if row['spec_id'] == spec]
    with closing(sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True)) as db:
        rows, _, _ = kit_rows(db, dict(loadouts=selected))
    return rows


def action_at(rows, spell, level):
    return next((row for row in rows if row[1] == spell and row[2] <= level <= row[3]), None)


class NativeSourceDamageRoleTests(unittest.TestCase):
    def test_shield_charge_scripted_damage_is_a_combat_action_at_native_levels(self):
        rows = native_rows(73)
        self.assertIsNone(action_at(rows, 385952, 60))
        for level in (61, 90):
            with self.subTest(level=level):
                action = action_at(rows, 385952, level)
                self.assertIsNotNone(action)
                self.assertEqual(action[4], 'attack', 'TC shield-charge Dummy emits native385954 damage')
                self.assertLess(action[5], 1000)

    def test_feral_frenzy_native_override_family_keeps_the_base_action_to_max_level(self):
        rows = native_rows(103)
        self.assertIsNone(action_at(rows, 274837, 54))
        for level in (55, 56, 57, 90):
            with self.subTest(level=level):
                action = action_at(rows, 274837, level)
                self.assertIsNotNone(action, 'native1243807 overrides274837; family availability must not end at56')
                self.assertEqual(action[4], 'attack', 'nativeFrenzy child274838 is damage, not utility')
                self.assertLess(action[5], 1000)

    def test_self_damage_and_fire_apl_exclusions_are_not_enemy_fillers(self):
        for spec, spell in ((266, 111400), (63, 116), (63, 1449)):
            with self.subTest(spec=spec, spell=spell):
                row = action_at(native_rows(spec), spell, 90)
                self.assertIsNotNone(row)
                self.assertEqual(row[4], 'utility')

    def test_established_damage_heal_and_utility_examples_keep_their_roles(self):
        cases = ((63, 133, 'attack'), (105, 8936, 'heal'), (63, 1953, 'utility'))
        for spec, spell, expected in cases:
            with self.subTest(spec=spec, spell=spell):
                row = action_at(native_rows(spec), spell, 90)
                self.assertIsNotNone(row)
                self.assertEqual(row[4], expected)


if __name__ == '__main__':
    unittest.main()
