import sqlite3
import unittest

from hostile_packmate_factions import POLICY, prepare


class HostilePackmateFactionTest(unittest.TestCase):
    def database(self):
        db = sqlite3.connect(':memory:')
        db.execute('CREATE TABLE content_creature_template (entry INTEGER PRIMARY KEY, faction INTEGER NOT NULL)')
        db.executemany('INSERT INTO content_creature_template VALUES (?,?)', [(234359, 0), (236073, 16), (234852, 0)])
        return db

    def test_hostile_stalker_uses_packmate_and_replays(self):
        flags = {'0x40': {'example': {'file': 'WoWCombatLog-102626_222725.txt', 'line': 76, 'flags': '0xa48'}, 'player_event_example': None}}
        with self.database() as db:
            audit = {'flags': {'234359': flags}}
            self.assertEqual(prepare(db, audit)['hostile'], [234359])
            self.assertEqual(prepare(db, audit)['hostile'], [234359])
            self.assertEqual(db.execute('SELECT faction FROM content_creature_template WHERE entry=234359').fetchone(), (16,))
            self.assertEqual(db.execute('SELECT policy,packmate_entry,faction,source_line FROM log_derived_hostile_packmate').fetchone(), (POLICY, 236073, 16, 76))
            self.assertEqual(db.execute('SELECT faction FROM content_creature_template WHERE entry=234852').fetchone(), (0,))

    def test_mixed_or_missing_flags_do_not_change_faction(self):
        with self.database() as db:
            report = prepare(db, {'flags': {'234359': {'0x40': {}, '0x20': {}}}})
            self.assertEqual(report['mixed_reactions'], [234359])
            self.assertEqual(db.execute('SELECT faction FROM content_creature_template WHERE entry=234359').fetchone(), (0,))
        with self.database() as db:
            self.assertEqual(prepare(db, {'flags': {'234359': {}}})['no_reaction_observed'], [234359])

    def test_changed_packmate_is_rejected(self):
        with self.database() as db:
            db.execute('UPDATE content_creature_template SET faction=35 WHERE entry=236073')
            flags = {'0x40': {'example': {'file': 'combat.txt', 'line': 1, 'flags': '0xa48'}, 'player_event_example': None}}
            with self.assertRaisesRegex(ValueError, 'pack-mate 236073'):
                prepare(db, {'flags': {'234359': flags}})
            self.assertEqual(db.execute('SELECT faction FROM content_creature_template WHERE entry=234359').fetchone(), (0,))


if __name__ == '__main__':
    unittest.main()
