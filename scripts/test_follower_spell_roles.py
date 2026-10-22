"""Observable intrinsic-role evidence at native-effect and TC/DB2 chain boundaries."""
from contextlib import closing
from pathlib import Path
import sqlite3
import unittest

from scripts.follower_spell_roles import SpellRoleCatalog, read_tc_child_artifact

ROOT = Path(__file__).resolve().parents[1]
TC_EDGES = Path('/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/tc-script-child-edges-a352b1fa.json')


class IntrinsicSpellRoleTests(unittest.TestCase):
    def test_native_shield_charge_script_then_charge_trigger_reaches_damage(self):
        with closing(sqlite3.connect((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro', uri=True)) as db:
            facts = SpellRoleCatalog(db, read_tc_child_artifact(TC_EDGES)).read_role(385952)
        self.assertEqual(facts['intrinsic'], 'damage')
        self.assertIn(385954, {row['spell_id'] for row in facts['role_evidence']})

    def test_native_health_leech_keeps_damage_and_self_heal_roles(self):
        # Exact native direct/periodic effects isolate the hit+heal contract from scripts.
        for spell, effect in ((3108, 927), (234153, 353731)):
            with self.subTest(spell=spell, effect=effect):
                with closing(sqlite3.connect(':memory:', uri=True)) as db:
                    db.execute('ATTACH DATABASE ? AS native', ((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro',))
                    db.execute('CREATE TABLE spell_effect AS SELECT * FROM native.spell_effect WHERE ID=?', (effect,))
                    facts = SpellRoleCatalog(db, dict(edges=[], unresolved_calls=[])).read_role(spell)
                self.assertEqual(facts['intrinsic'], 'mixed', 'TC leech also heals its caster')
                self.assertEqual({row['role'] for row in facts['role_evidence']}, {'damage', 'heal'})


if __name__ == '__main__':
    unittest.main()
