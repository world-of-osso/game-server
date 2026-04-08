use super::*;

#[test]
fn base_xp_level_1() {
    assert_eq!(base_kill_xp(1), 50); // 5*1 + 45
}

#[test]
fn base_xp_level_80() {
    assert_eq!(base_kill_xp(80), 445); // 5*80 + 45
}

#[test]
fn grey_level_at_10() {
    assert_eq!(grey_level(10), 4); // 10 - 5 - 1 = 4
}

#[test]
fn grey_level_at_80() {
    assert_eq!(grey_level(80), 71); // 80 - 9 = 71
}

#[test]
fn grey_level_at_5() {
    assert_eq!(grey_level(5), 0);
}

#[test]
fn kill_xp_same_level() {
    let xp = kill_xp(80, 80);
    assert_eq!(xp, base_kill_xp(80)); // no reduction
}

#[test]
fn kill_xp_higher_creature_bonus() {
    let same = kill_xp(78, 78);
    let higher = kill_xp(78, 80);
    assert!(higher > same, "higher level creature should give more XP");
}

#[test]
fn kill_xp_lower_creature_reduced() {
    let same = kill_xp(80, 80);
    let lower = kill_xp(80, 75);
    assert!(lower < same, "lower level creature should give less XP");
    assert!(lower > 0, "non-grey should still give some XP");
}

#[test]
fn kill_xp_grey_is_zero() {
    // Level 80 player, grey threshold = 71
    assert_eq!(kill_xp(80, 70), 0);
    assert_eq!(kill_xp(80, 50), 0);
    assert_eq!(kill_xp(80, 1), 0);
}

#[test]
fn kill_xp_just_above_grey() {
    // Level 80, grey = 71, so 72 should give some XP
    let xp = kill_xp(80, 72);
    assert!(xp > 0);
}

#[test]
fn kill_xp_zero_levels() {
    assert_eq!(kill_xp(0, 10), 0);
    assert_eq!(kill_xp(10, 0), 0);
}

#[test]
fn kill_xp_low_level_no_grey() {
    // Level 3 player: grey = 0, so level 1 creature is NOT grey
    assert!(kill_xp(3, 1) > 0);
}

// --- Level scaling tests ---

#[test]
fn scaling_within_bracket() {
    let bracket = ZoneLevelBracket::new(10, 60);
    assert_eq!(bracket.scaled_creature_level(25), 25);
}

#[test]
fn scaling_clamps_below_min() {
    let bracket = ZoneLevelBracket::new(10, 60);
    assert_eq!(bracket.scaled_creature_level(5), 10);
}

#[test]
fn scaling_clamps_above_max() {
    let bracket = ZoneLevelBracket::new(10, 60);
    assert_eq!(bracket.scaled_creature_level(80), 60);
}

#[test]
fn scaling_at_boundaries() {
    let bracket = ZoneLevelBracket::new(10, 60);
    assert_eq!(bracket.scaled_creature_level(10), 10);
    assert_eq!(bracket.scaled_creature_level(60), 60);
}

#[test]
fn is_in_range_checks() {
    let bracket = ZoneLevelBracket::new(20, 40);
    assert!(!bracket.is_in_range(19));
    assert!(bracket.is_in_range(20));
    assert!(bracket.is_in_range(30));
    assert!(bracket.is_in_range(40));
    assert!(!bracket.is_in_range(41));
}

#[test]
fn is_outleveled_checks() {
    let bracket = ZoneLevelBracket::new(10, 60);
    assert!(!bracket.is_outleveled(50));
    assert!(!bracket.is_outleveled(60));
    assert!(bracket.is_outleveled(61));
}

// --- Quest XP tests ---

#[test]
fn quest_xp_same_level() {
    assert_eq!(quest_xp(1000, 50, 50), 1000);
}

#[test]
fn quest_xp_below_quest_level() {
    // Player lower than quest — full XP
    assert_eq!(quest_xp(1000, 50, 40), 1000);
}

#[test]
fn quest_xp_within_grace() {
    // 5 levels above quest — still full XP
    assert_eq!(quest_xp(1000, 50, 55), 1000);
}

#[test]
fn quest_xp_starts_decaying() {
    // 6 levels above: 1 into decay zone → 1000 * 9/10 = 900
    assert_eq!(quest_xp(1000, 50, 56), 900);
}

#[test]
fn quest_xp_mid_decay() {
    // 10 levels above: 5 into decay → 1000 * 5/10 = 500
    assert_eq!(quest_xp(1000, 50, 60), 500);
}

#[test]
fn quest_xp_fully_decayed() {
    // 15 levels above: 10 into decay → 0
    assert_eq!(quest_xp(1000, 50, 65), 0);
}

#[test]
fn quest_xp_way_over_level() {
    assert_eq!(quest_xp(1000, 10, 80), 0);
}

#[test]
fn quest_xp_zero_base() {
    assert_eq!(quest_xp(0, 50, 50), 0);
}

// --- Level-up tests ---

#[test]
fn xp_to_level_1() {
    assert_eq!(xp_to_level(1), 400); // AzerothCore table
}

#[test]
fn xp_to_level_at_max() {
    assert_eq!(xp_to_level(80), 0); // max level
}

#[test]
fn xp_curve_key_values() {
    // Verify specific AzerothCore table values
    assert_eq!(xp_to_level(10), 7600);
    assert_eq!(xp_to_level(20), 20800);
    assert_eq!(xp_to_level(40), 74300);
    assert_eq!(xp_to_level(60), 290000); // TBC jump
    assert_eq!(xp_to_level(70), 1523800); // WotLK jump
    assert_eq!(xp_to_level(79), 1670800);
}

#[test]
fn xp_curve_monotonically_increasing() {
    for level in 1..79u8 {
        assert!(
            xp_to_level(level + 1) >= xp_to_level(level),
            "XP should increase: L{} ({}) < L{} ({})",
            level,
            xp_to_level(level),
            level + 1,
            xp_to_level(level + 1)
        );
    }
}

#[test]
fn level_up_single() {
    let mut xp = PlayerXp::new(1);
    let result = xp.add_xp(400); // exactly L1→L2
    assert_eq!(result, XpGainResult::LeveledUp { new_level: 2 });
    assert_eq!(xp.level, 2);
    assert_eq!(xp.current_xp, 0);
}

#[test]
fn level_up_with_overflow() {
    let mut xp = PlayerXp::new(1);
    let result = xp.add_xp(500); // 400 needed, 100 overflow
    assert_eq!(result, XpGainResult::LeveledUp { new_level: 2 });
    assert_eq!(xp.current_xp, 100);
}

#[test]
fn level_up_multiple() {
    let mut xp = PlayerXp::new(1);
    // L1→2: 400, L2→3: 900, total = 1300
    let result = xp.add_xp(1300);
    assert_eq!(result, XpGainResult::LeveledUp { new_level: 3 });
    assert_eq!(xp.level, 3);
    assert_eq!(xp.current_xp, 0);
}

#[test]
fn no_level_up() {
    let mut xp = PlayerXp::new(1);
    let result = xp.add_xp(100);
    assert_eq!(result, XpGainResult::Gained);
    assert_eq!(xp.level, 1);
    assert_eq!(xp.current_xp, 100);
}

#[test]
fn max_level_rejects_xp() {
    let mut xp = PlayerXp::new(80);
    let result = xp.add_xp(10000);
    assert_eq!(result, XpGainResult::AtMaxLevel);
    assert_eq!(xp.current_xp, 0);
}

#[test]
fn level_up_caps_at_max() {
    let mut xp = PlayerXp::new(79);
    let result = xp.add_xp(2000000); // L79→80 needs 1670800
    assert_eq!(result, XpGainResult::LeveledUp { new_level: 80 });
    assert_eq!(xp.current_xp, 0);
}

#[test]
fn progress_fraction() {
    let mut xp = PlayerXp::new(1);
    xp.current_xp = 200;
    assert!((xp.progress() - 0.5).abs() < 0.01); // 200/400
}

#[test]
fn progress_at_max() {
    let xp = PlayerXp::new(80);
    assert_eq!(xp.progress(), 1.0);
}

// --- Rested XP tests ---

#[test]
fn rested_new_starts_empty() {
    let rested = RestedXp::new(10000);
    assert_eq!(rested.amount, 0);
    assert_eq!(rested.max, 15000); // 1.5 * 10000
    assert!(!rested.is_rested());
}

#[test]
fn rested_accumulate_in_rest_area() {
    let mut rested = RestedXp::new(10000);
    rested.accumulate(8.0, 10000); // 8 hours = 5% of level
    assert_eq!(rested.amount, 500); // 10000 * 0.05
    assert!(rested.is_rested());
}

#[test]
fn rested_caps_at_max() {
    let mut rested = RestedXp::new(10000);
    rested.accumulate(1000.0, 10000); // way more than needed
    assert_eq!(rested.amount, 15000); // capped at 1.5 levels
}

#[test]
fn rested_apply_bonus_doubles_xp() {
    let mut rested = RestedXp::new(10000);
    rested.amount = 500;
    let (total, consumed) = rested.apply_bonus(200);
    assert_eq!(total, 400); // 200 base + 200 bonus
    assert_eq!(consumed, 200);
    assert_eq!(rested.amount, 300); // 500 - 200
}

#[test]
fn rested_bonus_limited_by_pool() {
    let mut rested = RestedXp::new(10000);
    rested.amount = 100;
    let (total, consumed) = rested.apply_bonus(500);
    assert_eq!(total, 600); // 500 + 100 (only 100 available)
    assert_eq!(consumed, 100);
    assert_eq!(rested.amount, 0);
}

#[test]
fn rested_no_bonus_when_empty() {
    let mut rested = RestedXp::new(10000);
    let (total, consumed) = rested.apply_bonus(500);
    assert_eq!(total, 500);
    assert_eq!(consumed, 0);
}

#[test]
fn rested_levels_fraction() {
    let mut rested = RestedXp::new(10000);
    rested.amount = 5000;
    assert!((rested.rested_levels(10000) - 0.5).abs() < 0.001);
}

#[test]
fn rested_update_max_on_level_up() {
    let mut rested = RestedXp::new(10000);
    rested.amount = 15000; // at cap
    rested.update_max(20000); // level up
    assert_eq!(rested.max, 30000);
    assert_eq!(rested.amount, 15000); // still within new cap
}

// --- Group XP tests ---

#[test]
fn group_xp_solo_gets_full() {
    let members = vec![GroupMemberXp {
        level: 80,
        distance: 5.0,
    }];
    let shares = group_kill_xp(80, &members);
    assert_eq!(shares.len(), 1);
    assert_eq!(shares[0].xp, kill_xp(80, 80)); // no bonus, no split
}

#[test]
fn group_xp_two_equal_split() {
    let members = vec![
        GroupMemberXp {
            level: 80,
            distance: 5.0,
        },
        GroupMemberXp {
            level: 80,
            distance: 10.0,
        },
    ];
    let shares = group_kill_xp(80, &members);
    // 2 members, 1.0x bonus, equal levels → each gets half
    let base = kill_xp(80, 80);
    assert_eq!(shares[0].xp, base / 2);
    assert_eq!(shares[1].xp, base / 2);
}

#[test]
fn group_xp_three_member_bonus() {
    let members = vec![
        GroupMemberXp {
            level: 80,
            distance: 5.0,
        },
        GroupMemberXp {
            level: 80,
            distance: 5.0,
        },
        GroupMemberXp {
            level: 80,
            distance: 5.0,
        },
    ];
    let shares = group_kill_xp(80, &members);
    let base = kill_xp(80, 80);
    // 3 members, 1.166x bonus → pool = base * 1.166, each gets pool/3
    let pool = (base as f32 * 1.166) as u32;
    let expected_each = pool / 3;
    assert_eq!(shares[0].xp, expected_each);
}

#[test]
fn group_xp_level_weighted() {
    let members = vec![
        GroupMemberXp {
            level: 80,
            distance: 5.0,
        },
        GroupMemberXp {
            level: 40,
            distance: 5.0,
        },
    ];
    let shares = group_kill_xp(80, &members);
    // Level 80 should get more than level 40
    assert!(shares[0].xp > shares[1].xp);
    // 80/(80+40) = 2/3, 40/(80+40) = 1/3
    assert_eq!(shares[0].xp, shares[1].xp * 2);
}

#[test]
fn group_xp_out_of_range_gets_zero() {
    let members = vec![
        GroupMemberXp {
            level: 80,
            distance: 5.0,
        },
        GroupMemberXp {
            level: 80,
            distance: 200.0,
        }, // out of range
    ];
    let shares = group_kill_xp(80, &members);
    assert!(shares[0].xp > 0);
    assert_eq!(shares[1].xp, 0);
}

#[test]
fn group_xp_all_out_of_range() {
    let members = vec![GroupMemberXp {
        level: 80,
        distance: 200.0,
    }];
    let shares = group_kill_xp(80, &members);
    assert_eq!(shares[0].xp, 0);
}
