import sqlite3
import unittest

from follower_reaction_flags import POLICY, prepare


class FollowerReactionFlagsTest(unittest.TestCase):
    def test_only_unambiguous_friendly_entries_change_and_replay(self):
        friendly = [235841, 236089, 237903, 240289, 243249, 251538, 272104]
        with sqlite3.connect(':memory:') as db:
            db.execute('CREATE TABLE content_creature_template (entry INTEGER PRIMARY KEY, faction INTEGER NOT NULL)')
            db.executemany('INSERT INTO content_creature_template VALUES (?,0)', [(entry,) for entry in friendly + [234852, 190051, 234359]])
            observation = {'count': 1, 'example': {'file': 'WoWCombatLog-100126_203256.txt', 'line': 737940, 'flags': '0xa18'}, 'player_event_example': None}
            audit = {'scope': [[2813, entry, '', 1] for entry in friendly + [234852,190051,234359]],
                     'flags': {str(entry): {'0x10': observation} for entry in friendly}}
            audit['flags'].update({'234852': {'0x40': observation, '0x20': observation},
                                   '190051': {}, '234359': {'0x40': observation}})
            expected = {'friendly': friendly, 'conflicting_reactions': [234852], 'no_reaction_observed': [190051]}
            self.assertEqual(prepare(db, audit), expected)
            self.assertEqual(prepare(db, audit), expected)
            self.assertEqual(db.execute('SELECT entry FROM content_creature_template WHERE faction=35 ORDER BY entry').fetchall(), [(entry,) for entry in friendly])
            self.assertEqual(db.execute('SELECT entry FROM content_creature_template WHERE faction=0 ORDER BY entry').fetchall(), [(190051,), (234359,), (234852,)])
            self.assertEqual(db.execute('SELECT count(*) FROM log_derived_faction_reaction WHERE policy=?', (POLICY,)).fetchone(), (7,))


if __name__ == '__main__':
    unittest.main()
