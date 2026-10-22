"""Behavioral source extraction and stable-key projection controls."""
import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import follower_trash_ai as trash

LW = Path('/home/osso/Repos/LittleWigs')


class TrashSources(unittest.TestCase):
    def test_lua_cast_events_belong_to_registered_named_actor(self):
        kits = trash.read_lua_kits(LW)
        self.assertEqual(kits[196102]['spells'], [387564])
        self.assertEqual(kits[187159]['spells'], [370225, 397726])
        self.assertEqual(kits[187155]['spells'], [377488])
        self.assertNotIn(194602, kits, 'transport books have no combat kit')
        self.assertNotIn(375649, kits[191164]['spells'], 'periodic damage is not a cast')

    def test_wowhead_abilities_only_not_loot_or_related_spells(self):
        page = """new Listview({template: 'item', id: 'drops', data: [{"id":7}]});
new Listview({template: 'spell', id: 'abilities', data: [{"id":387564,"name":"Mystic Vapors",modes:{"mode":[1,2,23]}},{"id":317898,"name":"Blinding Sleet",modes:{"mode":[1]}}]});"""
        self.assertEqual(trash.read_wowhead_abilities(page), [317898, 387564])
        self.assertEqual(trash.read_wowhead_abilities(json.dumps(page)), [317898, 387564])
        self.assertEqual(trash.read_wowhead_abilities('<html>No abilities</html>'), [])

    def test_similar_kit_is_named_policy_with_donor_not_new_spell(self):
        actors = [
            dict(entry=187160, name='Crystal Fury', family=0, models=[106135], maps=[2515], spells=[], classification='none'),
            dict(entry=196116, name='Crystal Fury', family=0, models=[106135], maps=[2515], spells=[370764], classification='Lua-only'),
        ]
        trash.infer_similar_kits(actors)
        self.assertEqual(actors[0]['spells'], [370764])
        self.assertEqual(actors[0]['donor_entry'], 196116)
        self.assertEqual(actors[0]['policy'], 'FOLLOWER_TRASH_SIMILAR_CREATURE_KIT')
        self.assertEqual(actors[0]['classification'], 'inferred')

    def test_no_cross_dungeon_or_zero_family_match(self):
        actors = [
            dict(entry=187160, name='Crystal Fury', family=0, models=[], maps=[2515], spells=[], classification='none'),
            dict(entry=196116, name='Unrelated', family=0, models=[], maps=[2521], spells=[370764], classification='Lua-only'),
        ]
        trash.infer_similar_kits(actors)
        self.assertEqual(actors[0]['classification'], 'none')


if __name__ == '__main__':
    unittest.main()
