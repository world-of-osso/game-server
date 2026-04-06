// Gated rewards, tabard championing, tracked/watched, and DR tests
// (extracted from reputation_tests.rs)

// --- Gated rewards ---

fn sample_rewards() -> Vec<GatedReward> {
    vec![
        GatedReward {
            item_id: 5001, faction_id: 72,
            required_standing: Standing::Friendly,
            reward_type: GatedRewardType::VendorItem, cost: 10_000,
        },
        GatedReward {
            item_id: 5002, faction_id: 72,
            required_standing: Standing::Honored,
            reward_type: GatedRewardType::Recipe, cost: 50_000,
        },
        GatedReward {
            item_id: 5003, faction_id: 72,
            required_standing: Standing::Revered,
            reward_type: GatedRewardType::Tabard, cost: 100_000,
        },
        GatedReward {
            item_id: 5004, faction_id: 72,
            required_standing: Standing::Exalted,
            reward_type: GatedRewardType::Mount, cost: 500_000,
        },
        GatedReward {
            item_id: 6001, faction_id: 76,
            required_standing: Standing::Honored,
            reward_type: GatedRewardType::Enchant, cost: 30_000,
        },
    ]
}

#[test]
fn can_access_at_required_standing() {
    let reg = sample_registry();
    let rep = CharacterReputation::new_for_race(&reg, HUMAN);
    let rewards = sample_rewards();
    assert!(can_access_reward(&rewards[0], &rep));
    assert!(!can_access_reward(&rewards[1], &rep));
}

#[test]
fn can_access_above_required_standing() {
    let reg = sample_registry();
    let mut rep = CharacterReputation::new_for_race(&reg, HUMAN);
    rep.set_rep(72, 42_000);
    let rewards = sample_rewards();
    assert!(can_access_reward(&rewards[0], &rep));
    assert!(can_access_reward(&rewards[1], &rep));
    assert!(can_access_reward(&rewards[2], &rep));
    assert!(!can_access_reward(&rewards[3], &rep));
}

#[test]
fn available_rewards_filters_by_standing() {
    let reg = sample_registry();
    let mut rep = CharacterReputation::new_for_race(&reg, HUMAN);
    rep.set_rep(72, 30_000);
    let rewards = sample_rewards();
    let available = available_rewards(&rewards, 72, &rep);
    assert_eq!(available.len(), 2);
    assert_eq!(available[0].item_id, 5001);
    assert_eq!(available[1].item_id, 5002);
}

#[test]
fn available_rewards_filters_by_faction() {
    let reg = sample_registry();
    let mut rep = CharacterReputation::new_for_race(&reg, HUMAN);
    rep.set_rep(76, 30_000);
    let rewards = sample_rewards();
    let available = available_rewards(&rewards, 76, &rep);
    assert_eq!(available.len(), 1);
    assert_eq!(available[0].item_id, 6001);
}

#[test]
fn upcoming_rewards_shows_next_unlocks() {
    let reg = sample_registry();
    let rep = CharacterReputation::new_for_race(&reg, HUMAN);
    let rewards = sample_rewards();
    let upcoming = upcoming_rewards(&rewards, 72, &rep);
    assert_eq!(upcoming.len(), 3);
    assert_eq!(upcoming[0].required_standing, Standing::Honored);
}

#[test]
fn upcoming_rewards_empty_at_exalted() {
    let mut rep = CharacterReputation::default();
    rep.set_rep(72, 63_000);
    let rewards = sample_rewards();
    assert!(upcoming_rewards(&rewards, 72, &rep).is_empty());
}

#[test]
fn no_rewards_for_unknown_faction() {
    let rep = CharacterReputation::default();
    let rewards = sample_rewards();
    assert!(available_rewards(&rewards, 999, &rep).is_empty());
}

// --- Tabard championing ---

const STORMWIND_TABARD: FactionTabard = FactionTabard { item_id: 45574, faction_id: 72 };

#[test]
fn champion_redirects_rep_in_dungeon() {
    assert_eq!(resolve_champion_faction(Some(999), Some(&STORMWIND_TABARD), true), Some(72));
}

#[test]
fn champion_ignored_outside_dungeon() {
    assert_eq!(resolve_champion_faction(Some(999), Some(&STORMWIND_TABARD), false), Some(999));
}

#[test]
fn no_tabard_uses_original_faction() {
    assert_eq!(resolve_champion_faction(Some(72), None, true), Some(72));
}

#[test]
fn no_tabard_no_faction() {
    assert_eq!(resolve_champion_faction(None, None, true), None);
}

#[test]
fn champion_overrides_mob_faction() {
    assert_eq!(resolve_champion_faction(Some(999), Some(&STORMWIND_TABARD), true), Some(72));
}

#[test]
fn gain_dungeon_rep_with_tabard() {
    let reg = sample_registry();
    let mut rep = CharacterReputation::new_for_race(&reg, HUMAN);
    let sw_before = rep.get_value(72);
    let results = gain_dungeon_rep(&mut rep, Some(999), 100, Some(&STORMWIND_TABARD), true, &reg);
    assert!(!results.is_empty());
    assert_eq!(results[0].faction_id, 72);
    assert_eq!(rep.get_value(72), sw_before + 100);
}

#[test]
fn gain_dungeon_rep_without_tabard() {
    let reg = sample_registry();
    let mut rep = CharacterReputation::new_for_race(&reg, HUMAN);
    let results = gain_dungeon_rep(&mut rep, Some(72), 100, None, true, &reg);
    assert_eq!(results[0].faction_id, 72);
}

#[test]
fn gain_dungeon_rep_no_faction_mob() {
    let reg = sample_registry();
    let mut rep = CharacterReputation::new_for_race(&reg, HUMAN);
    assert!(gain_dungeon_rep(&mut rep, None, 100, None, true, &reg).is_empty());
}

#[test]
fn gain_dungeon_rep_tabard_with_spillover() {
    let reg = sample_registry();
    let mut rep = CharacterReputation::new_for_race(&reg, HUMAN);
    let if_before = rep.get_value(IRONFORGE_ID);
    let results = gain_dungeon_rep(&mut rep, Some(999), 1000, Some(&STORMWIND_TABARD), true, &reg);
    assert!(results.len() > 1);
    let if_result = results.iter().find(|r| r.faction_id == IRONFORGE_ID).unwrap();
    assert_eq!(rep.get_value(IRONFORGE_ID), if_before + 250);
    assert_eq!(if_result.actual_change(), 250);
}

#[test]
fn gain_dungeon_rep_outside_dungeon_no_redirect() {
    let reg = sample_registry();
    let mut rep = CharacterReputation::new_for_race(&reg, HUMAN);
    let results = gain_dungeon_rep(&mut rep, Some(76), 100, Some(&STORMWIND_TABARD), false, &reg);
    assert_eq!(results[0].faction_id, 76);
}

// --- Tracked / watched factions ---

#[test]
fn track_and_untrack() {
    let mut rep = CharacterReputation::default();
    assert!(rep.tracked_factions().is_empty());
    rep.track(72);
    rep.track(76);
    assert_eq!(rep.tracked_factions().len(), 2);
    assert!(rep.is_tracked(72));
    rep.untrack(72);
    assert!(!rep.is_tracked(72));
    assert_eq!(rep.tracked_factions().len(), 1);
}

#[test]
fn track_duplicate_is_noop() {
    let mut rep = CharacterReputation::default();
    rep.track(72);
    rep.track(72);
    assert_eq!(rep.tracked_factions().len(), 1);
}

#[test]
fn watch_faction() {
    let mut rep = CharacterReputation::default();
    rep.watch(72);
    assert_eq!(rep.watched_faction(), Some(72));
    assert!(rep.is_tracked(72));
}

#[test]
fn watch_replaces_previous() {
    let mut rep = CharacterReputation::default();
    rep.watch(72);
    rep.watch(76);
    assert_eq!(rep.watched_faction(), Some(76));
    assert!(rep.is_tracked(72));
    assert!(rep.is_tracked(76));
}

#[test]
fn unwatch_clears() {
    let mut rep = CharacterReputation::default();
    rep.watch(72);
    rep.unwatch();
    assert_eq!(rep.watched_faction(), None);
    assert!(rep.is_tracked(72));
}

#[test]
fn untrack_watched_clears_watch() {
    let mut rep = CharacterReputation::default();
    rep.watch(72);
    rep.untrack(72);
    assert_eq!(rep.watched_faction(), None);
    assert!(!rep.is_tracked(72));
}

#[test]
fn display_entry_values() {
    let reg = sample_registry();
    let rep = CharacterReputation::new_for_race(&reg, HUMAN);
    let entry = rep.display_entry(72);
    assert_eq!(entry.faction_id, 72);
    assert_eq!(entry.standing, Standing::Friendly);
    assert_eq!(entry.value, 21_000);
    assert_eq!(entry.progress, (0, 9_000));
}

#[test]
fn display_tracked_list() {
    let reg = sample_registry();
    let mut rep = CharacterReputation::new_for_race(&reg, HUMAN);
    rep.track(72);
    rep.track(76);
    let entries = rep.display_tracked();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].faction_id, 72);
    assert_eq!(entries[1].standing, Standing::Hated);
}

#[test]
fn display_tracked_empty() {
    let rep = CharacterReputation::default();
    assert!(rep.display_tracked().is_empty());
}

// --- Mob kill rep diminishing returns ---

#[test]
fn dr_same_level_full_rep() {
    assert_eq!(diminished_mob_rep(100, 60, 60), 100);
}

#[test]
fn dr_higher_mob_full_rep() {
    assert_eq!(diminished_mob_rep(100, 60, 65), 100);
}

#[test]
fn dr_one_level_below() {
    assert_eq!(diminished_mob_rep(100, 60, 59), 87);
}

#[test]
fn dr_four_levels_below() {
    assert_eq!(diminished_mob_rep(100, 60, 56), 50);
}

#[test]
fn dr_seven_levels_below() {
    assert_eq!(diminished_mob_rep(100, 60, 53), 12);
}

#[test]
fn dr_eight_levels_below_grey() {
    assert_eq!(diminished_mob_rep(100, 60, 52), 0);
}

#[test]
fn dr_far_below_grey() {
    assert_eq!(diminished_mob_rep(100, 60, 10), 0);
}

#[test]
fn dr_small_rep_rounds_down() {
    assert_eq!(diminished_mob_rep(7, 60, 59), 6);
}

#[test]
fn dr_level_one_mob_vs_high_player() {
    assert_eq!(diminished_mob_rep(100, 70, 1), 0);
}
