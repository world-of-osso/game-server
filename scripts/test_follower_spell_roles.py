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
        # One exact native effect isolates the effect9 hit+heal contract from scripts.
        with closing(sqlite3.connect(':memory:', uri=True)) as db:
            db.execute('ATTACH DATABASE ? AS native', ((ROOT / 'data/world.db').resolve().as_uri() + '?mode=ro',))
            spell, effect = db.execute('SELECT SpellID,ID FROM native.spell_effect WHERE DifficultyID=0 AND Effect=9 AND EffectTriggerSpell=0 ORDER BY ID LIMIT 1').fetchone()
            db.execute('CREATE TABLE spell_effect AS SELECT * FROM native.spell_effect WHERE ID=?', (effect,))
            facts = SpellRoleCatalog(db, dict(edges=[], unresolved_calls=[])).read_role(spell)
        self.assertEqual(facts['intrinsic'], 'mixed', 'TC EffectHealthLeech also HealBySpell to caster')
        self.assertEqual({row['role'] for row in facts['role_evidence']}, {'damage', 'heal'})


if __name__ == '__main__':
    unittest.main()
