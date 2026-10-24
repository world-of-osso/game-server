"""Behavior of conservative pinned APL active-enemy admission projection."""
import unittest

from scripts.follower_spec_kits import active_enemy_lower_bound, derive_active_enemy_minima, read_active_enemy_sources


class ActiveEnemyProjectionTests(unittest.TestCase):
    def test_called_list_and_inline_guards_both_constrain_the_action(self):
        source = '''actions=call_action_list,name=cleave,if=active_enemies>2
        actions.cleave=area_blast,if=active_enemies>1&buff.ready.up'''
        self.assertEqual(derive_active_enemy_minima(source), {'area_blast': 3})

    def test_unknown_disjunction_keeps_an_unrestricted_alternate_use(self):
        self.assertEqual(active_enemy_lower_bound('active_enemies>3|howl_summon.ready'), 1)
        self.assertEqual(active_enemy_lower_bound('(active_enemies>3|howl_summon.ready)&active_enemies>1'), 2)

    def test_pinned_nested_call_guard_projects_its_weaker_viable_branch(self):
        condition = 'talent.black_arrow&(active_enemies>2|talent.beast_cleave&active_enemies>1)'
        self.assertEqual(active_enemy_lower_bound(condition), 2)
        self.assertEqual(active_enemy_lower_bound('active_enemies<2|active_enemies>3'), 1)

    def test_unrestricted_action_use_wins_over_a_guarded_list_use(self):
        source = '''actions=strike
        actions+=/call_action_list,name=aoe,if=active_enemies>2
        actions.aoe=strike
        actions.aoe+=/other_area'''
        self.assertEqual(derive_active_enemy_minima(source), {'other_area': 3})

    def test_precombat_and_unreachable_lists_do_not_gate_combat_actions(self):
        source = '''actions.precombat=strike,if=active_enemies>7
        actions=strike
        actions.unreachable=other_area,if=active_enemies>9'''
        self.assertEqual(derive_active_enemy_minima(source), {})

    def test_source_projection_is_not_wild_thrash_specific(self):
        source = '''actions=first_area,if=active_enemies>1
        actions+=/second_area,if=active_enemies>=4
        actions+=/strike,if=buff.ready.up'''
        self.assertEqual(derive_active_enemy_minima(source), {'first_area': 2, 'second_area': 4})

    def test_unsupported_count_arithmetic_is_not_silently_dropped(self):
        for predicate in ('active_enemies>gcd', '!(active_enemies>1)', 'active_enemies+1>3', 'variable.area_ready'):
            with self.subTest(predicate=predicate):
                with self.assertRaises(ValueError):
                    active_enemy_lower_bound(predicate)

    def test_missing_and_recursive_called_lists_fail_explicitly(self):
        for source in ('actions=call_action_list,name=missing',
                       'actions=call_action_list,name=loop\nactions.loop=call_action_list,name=default'):
            with self.subTest(source=source):
                with self.assertRaises(ValueError):
                    derive_active_enemy_minima(source)

    def test_pinned_bm_projects_every_action_without_gating_single_target_alternates(self):
        admissions, evidence = read_active_enemy_sources({253})
        self.assertEqual(admissions, {253: {'wild_thrash': 2}})
        self.assertEqual(evidence[253]['minima'], admissions[253])
        self.assertEqual(len(evidence[253]['sha256']), 64)
        self.assertEqual(read_active_enemy_sources({262}), ({}, {}))


if __name__ == '__main__':
    unittest.main()
