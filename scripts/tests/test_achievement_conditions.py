import copy
import sqlite3
import sys
import unittest
from contextlib import closing
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))
import import_achievement_conditions as rules  # noqa: E402

# Pinned Difficulty.db2 OldEnumValue 0 = Normal1, 10-player3, Normal38, Lorewalking236.
LEGACY = {0: {1, 3, 38, 236}, 1: {2, 4, 39}}


def node(parent, operator, kind=0, asset=0, amount=0):
    return {'Parent': str(parent), 'Operator': str(operator), 'Amount': str(amount),
            'Type': str(kind), 'Asset': str(asset), 'SecondaryAsset': '0', 'TertiaryAsset': '0'}


class Conditions(unittest.TestCase):
    def setUp(self):
        self.criteria = {1: {'Type': '0', 'ModifierTreeId': '10', 'StartEvent': '0',
                             'StartTimer': '0', 'FailEvent': '0', 'EligibilityWorldStateID': '0'}}

    def compile(self, modifiers, criteria=None):
        compiled = rules.qualified_conditions(criteria or self.criteria, modifiers, LEGACY)
        return {row: rest for row, (_, *rest) in compiled.items()}

    def test_all_single_true_mythic_requires_that_difficulty(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 68, 23)}
        self.assertEqual(self.compile(modifiers), {1: [(True, (23,)), ()]})

    def test_some_one_of_normal_or_heroic_allows_either(self):
        modifiers = {10: node(0, 8), 11: node(10, 2, 68, 1), 12: node(10, 2, 68, 2)}
        self.assertEqual(self.compile(modifiers), {1: [(True, (1, 2)), ()]})

    def test_all_single_false_excludes_listed_difficulties(self):
        modifiers = {10: node(0, 4), 11: node(10, 3, 68, 7), 12: node(10, 3, 68, 17)}
        self.assertEqual(self.compile(modifiers), {1: [(False, (7, 17)), ()]})

    def test_all_single_true_and_false_admits_the_difference(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 20, 1), 12: node(10, 3, 68, 39)}
        self.assertEqual(self.compile(modifiers), {1: [(True, (2, 4)), ()]})

    def test_legacy_value_admits_every_difficulty_with_that_old_enum(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 20, 0)}
        self.assertEqual(self.compile(modifiers), {1: [(True, (1, 3, 38, 236)), ()]})

    def test_legacy_and_current_difficulty_leaves_intersect(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 20, 1), 12: node(10, 2, 68, 4)}
        self.assertEqual(self.compile(modifiers), {1: [(True, (4,)), ()]})

    def test_player_facts_alone_or_with_difficulty(self):
        alone = {10: node(0, 4), 11: node(10, 2, 32, 1), 12: node(10, 2, 69, 60), 13: node(10, 2, 17, 12)}
        self.assertEqual(self.compile(alone), {1: [None, ((17, 12), (32, 1), (69, 60))]})
        mixed = {10: node(0, 4), 11: node(10, 2, 8, 46598), 12: node(10, 0), 13: node(12, 2, 68, 1), 14: node(12, 2, 68, 2)}
        mixed[12] = node(10, 8)
        self.assertEqual(self.compile(mixed), {1: [(True, (1, 2)), ((8, 46598),)]})

    def test_untimestamped_time_event_always_passed(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 289, 600), 12: node(10, 2, 8, 46598)}
        self.assertEqual(self.compile(modifiers), {1: [None, ((8, 46598),)]})
        timestamped = {10: node(0, 4), 11: node(10, 2, 289, 350)}
        self.assertEqual(self.compile(timestamped), {})
        window = {10: node(0, 4), 11: node(10, 2, 289, 350), 12: node(10, 3, 289, 353)}
        self.assertEqual(self.compile(window), {})

    def test_unsatisfiable_foreign_nested_or_negated_facts_stay_disabled(self):
        cases = {
            'all of two different difficulties': {10: node(0, 4), 11: node(10, 2, 68, 1), 12: node(10, 2, 68, 2)},
            'difference empties': {10: node(0, 4), 11: node(10, 2, 68, 1), 12: node(10, 3, 68, 1)},
            'unknown legacy value': {10: node(0, 4), 11: node(10, 2, 20, 7)},
            'some needing two': {10: node(0, 8, amount=2), 11: node(10, 2, 68, 1), 12: node(10, 2, 68, 2)},
            'some of facts': {10: node(0, 8), 11: node(10, 2, 8, 1)},
            'negated fact': {10: node(0, 4), 11: node(10, 3, 8, 46598)},
            'target leaf': {10: node(0, 4), 11: node(10, 2, 4, 448)},
            'nested all': {10: node(0, 4), 11: node(10, 4), 12: node(11, 2, 8, 1)},
            'zero fact asset': {10: node(0, 4), 11: node(10, 2, 32, 0)},
            'empty tree': {10: node(0, 4)},
        }
        for name, modifiers in cases.items():
            with self.subTest(name):
                self.assertEqual(self.compile(modifiers), {})

    def test_target_creature_requires_a_per_request_emote_producer(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 4, 1412)}
        emote = copy.deepcopy(self.criteria)
        emote[1]['Type'] = '54'
        self.assertEqual(self.compile(modifiers, emote), {1: [None, ((4, 1412),)]})
        self.assertEqual(self.compile(modifiers), {}, 'other producers have no target snapshot')
        modifiers[11] = node(10, 2, 5, 0)
        self.assertEqual(self.compile(modifiers, emote), {}, 'player/dead/faction targets remain unsupported')

    def test_arena_player_facts_on_all_gated_events(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 24, 2), 12: node(10, 2, 60, 0)}
        for kind in (0, 17, 32, 33, 56, 70):
            criteria = copy.deepcopy(self.criteria)
            criteria[1]['Type'] = str(kind)
            self.assertEqual(self.compile(modifiers, criteria), {1: [None, ((24, 2), (60, 0))]})
            modifiers[11]['Asset'] = '5'
            self.assertEqual(self.compile(modifiers, criteria), {}, 'no native5v5')
            modifiers[11]['Asset'] = '2'
        derived = copy.deepcopy(self.criteria)
        derived[1]['Type'] = '8'
        self.assertEqual(self.compile(modifiers, derived), {}, 'structural recomputation has no player facts')

    def test_player_zone_preserves_the_authored_id_and_other_conditions(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 41, 1519), 12: node(10, 2, 69, 10)}
        self.assertEqual(self.compile(modifiers), {1: [None, ((41, 1519), (69, 10))]})
        modifiers[11]['Operator'] = '3'
        self.assertEqual(self.compile(modifiers), {}, 'negated zone remains unsupported')

    def test_known_solo_condition_admits_the_pinned_zero_asset(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 35, 0)}
        self.assertEqual(self.compile(modifiers), {1: [None, ((35, 0),)]})
        modifiers[11]['Operator'] = '3'
        self.assertEqual(self.compile(modifiers), {}, 'negated party conditions remain unsupported')

    def test_group_roster_limit_preserves_strict_threshold(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 20, 0), 12: node(10, 2, 29, 9)}
        self.assertEqual(self.compile(modifiers), {1: [(True, (1, 3, 38, 236)), ((29, 9),)]})

    def test_map_instance_type_preserves_exact_type_including_world_zero(self):
        for instance_type in (0, 1, 2, 3, 4, 5):
            modifiers = {10: node(0, 4), 11: node(10, 2, 122, instance_type)}
            self.assertEqual(self.compile(modifiers), {1: [None, ((122, instance_type),)]})
        modifiers[11]['Asset'] = '128'
        self.assertEqual(self.compile(modifiers), {}, 'unknown out-of-range type stays disabled')

    def test_native_identity_loader_rejects_extra_fields_flags_and_source34(self):
        with closing(sqlite3.connect(':memory:')) as db:
            db.executescript('''
                CREATE TABLE tdb_conditions(SourceTypeOrReferenceId INTEGER, SourceEntry INTEGER);
                CREATE TABLE player_condition(
                    ID INTEGER, Flags INTEGER, ClassMask INTEGER,
                    RaceMasks_0 INTEGER, RaceMasks_1 INTEGER,
                    Gender INTEGER, NativeGender INTEGER, PrevQuestID_0 INTEGER,
                    Failure_description_lang TEXT);
                INSERT INTO player_condition VALUES
                    (923,1,0,1309324210,-1440044374,-1,-1,0,''),
                    (62,0,8,0,0,-1,-1,0,''),
                    (100,0,8,0,0,-1,-1,77,''),
                    (101,128,8,0,0,-1,-1,0,''),
                    (102,0,8,0,0,0,-1,0,''),
                    (103,0,8,0,0,-1,-1,0,'');
                INSERT INTO tdb_conditions VALUES(34,103);
            ''')
            self.assertEqual(rules.load_identity_condition_ids(db), {923, 62})

    def test_player_condition_leaf_preserves_the_authored_id(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 2, 923)}
        self.assertEqual(self.compile(modifiers), {1: [None, ((2, 923),)]})

    def test_ungated_types_and_timed_criteria_stay_disabled(self):
        modifiers = {10: node(0, 4), 11: node(10, 2, 8, 46598)}
        for kind in sorted(rules.UNGATED_TYPES):
            with self.subTest(kind):
                criteria = copy.deepcopy(self.criteria)
                criteria[1]['Type'] = str(kind)
                self.assertEqual(self.compile(modifiers, criteria), {})
        for key in rules.CONDITIONS:
            with self.subTest(key):
                criteria = copy.deepcopy(self.criteria)
                criteria[1][key] = '5'
                self.assertEqual(self.compile(modifiers, criteria), {})


if __name__ == '__main__':
    unittest.main()
