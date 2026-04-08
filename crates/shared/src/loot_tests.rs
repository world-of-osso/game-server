use super::*;

fn sample_table() -> LootTable {
    LootTable {
        entries: vec![
            LootEntry {
                item_id: 100,
                chance: 1.0,
                min_count: 1,
                max_count: 1,
                entry_type: LootEntryType::Normal,
            },
            LootEntry {
                item_id: 200,
                chance: 0.5,
                min_count: 1,
                max_count: 3,
                entry_type: LootEntryType::Normal,
            },
            LootEntry {
                item_id: 300,
                chance: 0.01,
                min_count: 1,
                max_count: 1,
                entry_type: LootEntryType::Rare,
            },
        ],
    }
}

#[test]
fn guaranteed_item_always_drops() {
    let table = sample_table();
    let rolls = vec![0.5, 0.99, 0.99]; // only first entry passes
    let count_rolls = vec![0.0, 0.0, 0.0];
    let drops = table.roll(&rolls, &count_rolls, &[]);
    assert_eq!(drops.len(), 1);
    assert_eq!(drops[0].item_id, 100);
}

#[test]
fn multiple_items_can_drop() {
    let table = sample_table();
    let rolls = vec![0.5, 0.3, 0.005]; // all three pass
    let count_rolls = vec![0.0, 0.5, 0.0];
    let drops = table.roll(&rolls, &count_rolls, &[]);
    assert_eq!(drops.len(), 3);
}

#[test]
fn nothing_drops_on_bad_rolls() {
    let table = sample_table();
    // Only first is guaranteed; others roll > chance
    let rolls = vec![1.1, 0.9, 0.5]; // first > 1.0 technically, but 1.0 chance means <= check
    let count_rolls = vec![0.0; 3];
    // roll=1.1 > chance=1.0 → even guaranteed fails if roll > chance
    let drops = table.roll(&rolls, &count_rolls, &[]);
    assert!(drops.is_empty());
}

#[test]
fn count_rolls_within_range() {
    let table = LootTable {
        entries: vec![LootEntry {
            item_id: 500,
            chance: 1.0,
            min_count: 2,
            max_count: 10,
            entry_type: LootEntryType::Normal,
        }],
    };
    let drops_min = table.roll(&[0.0], &[0.0], &[]);
    assert_eq!(drops_min[0].count, 2);

    let drops_mid = table.roll(&[0.0], &[0.5], &[]);
    assert_eq!(drops_mid[0].count, 6);

    let drops_max = table.roll(&[0.0], &[1.0], &[]);
    assert_eq!(drops_max[0].count, 10);
}

#[test]
fn empty_table_produces_no_drops() {
    let table = LootTable::default();
    let drops = table.roll(&[], &[], &[]);
    assert!(drops.is_empty());
}

#[test]
fn single_count_item() {
    let table = LootTable {
        entries: vec![LootEntry {
            item_id: 100,
            chance: 1.0,
            min_count: 1,
            max_count: 1,
            entry_type: LootEntryType::Normal,
        }],
    };
    let drops = table.roll(&[0.0], &[0.99], &[]);
    assert_eq!(drops[0].count, 1);
}

#[test]
fn rare_item_drops_on_low_roll() {
    let table = LootTable {
        entries: vec![
            LootEntry {
                item_id: 200,
                chance: 0.5,
                min_count: 1,
                max_count: 1,
                entry_type: LootEntryType::Normal,
            },
            LootEntry {
                item_id: 300,
                chance: 0.01,
                min_count: 1,
                max_count: 1,
                entry_type: LootEntryType::Rare,
            },
        ],
    };
    let rolls = vec![0.99, 0.005]; // only rare passes
    let count_rolls = vec![0.0; 2];
    let drops = table.roll(&rolls, &count_rolls, &[]);
    assert_eq!(drops.len(), 1);
    assert_eq!(drops[0].item_id, 300);
}

// --- Loot window tests ---

#[test]
fn generate_loot_window() {
    let table = sample_table();
    let rolls = vec![0.5, 0.3, 0.005]; // all three pass
    let count_rolls = vec![0.0, 0.0, 0.0];
    let window = LootWindow::generate(42, &table, &rolls, &count_rolls, &[], 500);
    assert_eq!(window.source, 42);
    assert_eq!(window.slots.len(), 3);
    assert_eq!(window.gold, 500);
    assert!(!window.gold_looted);
    assert!(!window.is_fully_looted());
}

#[test]
fn loot_item_from_window() {
    let table = sample_table();
    let rolls = vec![0.0; 3];
    let count_rolls = vec![0.0; 3];
    let mut window = LootWindow::generate(1, &table, &rolls, &count_rolls, &[], 0);

    let item = window.loot_item(0).unwrap();
    assert_eq!(item.item_id, 100);
    assert_eq!(window.available_count(), 2);

    // Can't loot same slot twice
    assert!(window.loot_item(0).is_none());
}

#[test]
fn loot_gold_from_window() {
    let mut window = LootWindow::generate(1, &LootTable::default(), &[], &[], &[], 1000);
    let gold = window.loot_gold().unwrap();
    assert_eq!(gold, 1000);
    assert!(window.loot_gold().is_none()); // already looted
}

#[test]
fn fully_looted_check() {
    let table = LootTable {
        entries: vec![LootEntry {
            item_id: 100,
            chance: 1.0,
            min_count: 1,
            max_count: 1,
            entry_type: LootEntryType::Normal,
        }],
    };
    let mut window = LootWindow::generate(1, &table, &[0.0], &[0.0], &[], 50);
    assert!(!window.is_fully_looted());

    window.loot_item(0);
    assert!(!window.is_fully_looted()); // gold still

    window.loot_gold();
    assert!(window.is_fully_looted());
}

#[test]
fn empty_loot_window_is_fully_looted() {
    let window = LootWindow::generate(1, &LootTable::default(), &[], &[], &[], 0);
    assert!(window.is_fully_looted());
}

#[test]
fn loot_item_out_of_bounds() {
    let mut window = LootWindow::generate(1, &LootTable::default(), &[], &[], &[], 0);
    assert!(window.loot_item(99).is_none());
}

// --- Loot sparkle tests ---

#[test]
fn sparkle_on_unlooted_corpse() {
    let w = make_simple_window(1, 100, 50);
    let windows: Vec<(u64, &LootWindow)> = vec![(1, &w)];
    let sparkles = sparkle_corpses(&windows);
    assert_eq!(sparkles.len(), 1);
    assert_eq!(sparkles[0].corpse, 1);
}

#[test]
fn no_sparkle_on_fully_looted() {
    let mut w = make_simple_window(1, 100, 50);
    w.loot_item(0);
    w.loot_gold();
    let windows: Vec<(u64, &LootWindow)> = vec![(1, &w)];
    let sparkles = sparkle_corpses(&windows);
    assert!(sparkles.is_empty());
}

#[test]
fn sparkle_mixed_looted_and_not() {
    let w1 = make_simple_window(1, 100, 50);
    let mut w2 = make_simple_window(2, 200, 30);
    w2.loot_item(0);
    w2.loot_gold();
    let windows: Vec<(u64, &LootWindow)> = vec![(1, &w1), (2, &w2)];
    let sparkles = sparkle_corpses(&windows);
    assert_eq!(sparkles.len(), 1);
    assert_eq!(sparkles[0].corpse, 1);
}

#[test]
fn sparkle_gold_only_corpse() {
    let w = LootWindow::generate(1, &LootTable::default(), &[], &[], &[], 100);
    let windows: Vec<(u64, &LootWindow)> = vec![(1, &w)];
    let sparkles = sparkle_corpses(&windows);
    assert_eq!(sparkles.len(), 1); // gold still available
}

// --- AoE loot tests ---

fn make_simple_window(entity: u64, item_id: u32, gold: u32) -> LootWindow {
    let table = LootTable {
        entries: vec![LootEntry {
            item_id,
            chance: 1.0,
            min_count: 1,
            max_count: 1,
            entry_type: LootEntryType::Normal,
        }],
    };
    LootWindow::generate(entity, &table, &[0.0], &[0.0], &[], gold)
}

#[test]
fn aoe_loot_collects_nearby() {
    let mut w1 = make_simple_window(1, 100, 50);
    let mut w2 = make_simple_window(2, 200, 30);
    let mut windows: Vec<(u64, &mut LootWindow)> = vec![(1, &mut w1), (2, &mut w2)];
    let distances = vec![(1, 10.0), (2, 20.0)];

    let result = aoe_loot(&mut windows, &distances, AOE_LOOT_RADIUS);
    assert_eq!(result.items.len(), 2);
    assert_eq!(result.gold, 80);
    assert_eq!(result.fully_looted.len(), 2);
}

#[test]
fn aoe_loot_skips_out_of_range() {
    let mut w1 = make_simple_window(1, 100, 50);
    let mut w2 = make_simple_window(2, 200, 30);
    let mut windows: Vec<(u64, &mut LootWindow)> = vec![(1, &mut w1), (2, &mut w2)];
    let distances = vec![(1, 10.0), (2, 100.0)]; // w2 out of range

    let result = aoe_loot(&mut windows, &distances, AOE_LOOT_RADIUS);
    assert_eq!(result.items.len(), 1);
    assert_eq!(result.items[0].item_id, 100);
    assert_eq!(result.gold, 50);
    assert_eq!(result.fully_looted, vec![1]);
}

#[test]
fn aoe_loot_empty_no_corpses() {
    let result = aoe_loot(&mut [], &[], AOE_LOOT_RADIUS);
    assert!(result.items.is_empty());
    assert_eq!(result.gold, 0);
}

#[test]
fn aoe_loot_already_looted_skipped() {
    let mut w1 = make_simple_window(1, 100, 50);
    w1.loot_item(0); // pre-loot the item
    w1.loot_gold();
    let mut windows: Vec<(u64, &mut LootWindow)> = vec![(1, &mut w1)];
    let distances = vec![(1, 5.0)];

    let result = aoe_loot(&mut windows, &distances, AOE_LOOT_RADIUS);
    assert!(result.items.is_empty());
    assert_eq!(result.gold, 0);
}

// --- Currency drop tests ---

#[test]
fn gold_scales_with_level() {
    let low = gold_drop(10, CreatureRank::Normal, 0.0);
    let high = gold_drop(80, CreatureRank::Normal, 0.0);
    assert!(high > low);
    assert_eq!(low, 200); // 10² × 2
    assert_eq!(high, 12800); // 80² × 2
}

#[test]
fn gold_elite_3x_normal() {
    let normal = gold_drop(50, CreatureRank::Normal, 0.0);
    let elite = gold_drop(50, CreatureRank::Elite, 0.0);
    assert_eq!(elite, normal * 3);
}

#[test]
fn gold_boss_10x_normal() {
    let normal = gold_drop(50, CreatureRank::Normal, 0.0);
    let boss = gold_drop(50, CreatureRank::Boss, 0.0);
    assert_eq!(boss, normal * 10);
}

#[test]
fn gold_rare_2x_normal() {
    let normal = gold_drop(50, CreatureRank::Normal, 0.0);
    let rare = gold_drop(50, CreatureRank::Rare, 0.0);
    assert_eq!(rare, normal * 2);
}

#[test]
fn gold_variance_with_roll() {
    let min = gold_drop(50, CreatureRank::Normal, 0.0);
    let max = gold_drop(50, CreatureRank::Normal, 1.0);
    assert!(max > min);
    // 30% variance: max ≈ min * 1.3
    let ratio = max as f32 / min as f32;
    assert!((ratio - 1.3).abs() < 0.05);
}

// --- Roll window tests ---

#[test]
fn roll_window_auto_passes_non_equippable() {
    let item = LootDrop {
        item_id: 100,
        count: 1,
    };
    let window = RollWindow::new(item, &[1, 2, 3], &[1, 3]); // 2 can't equip
    assert_eq!(window.pending_players, vec![1, 3]);
    assert_eq!(window.responses.len(), 1); // player 2 auto-passed
    assert_eq!(window.responses[0].player_id, 2);
    assert_eq!(window.responses[0].choice, LootRollChoice::Pass);
}

#[test]
fn roll_window_respond_and_complete() {
    let item = LootDrop {
        item_id: 100,
        count: 1,
    };
    let mut window = RollWindow::new(item, &[1, 2], &[1, 2]);
    assert!(!window.is_complete());

    assert!(window.respond(1, LootRollChoice::Need, 80));
    assert!(!window.is_complete());

    assert!(window.respond(2, LootRollChoice::Greed, 90));
    assert!(window.is_complete());

    assert_eq!(window.resolve(), Some(1)); // Need beats Greed
}

#[test]
fn roll_window_reject_duplicate_response() {
    let item = LootDrop {
        item_id: 100,
        count: 1,
    };
    let mut window = RollWindow::new(item, &[1], &[1]);
    assert!(window.respond(1, LootRollChoice::Need, 50));
    assert!(!window.respond(1, LootRollChoice::Greed, 99)); // already responded
}

#[test]
fn roll_window_timeout_auto_passes() {
    let item = LootDrop {
        item_id: 100,
        count: 1,
    };
    let mut window = RollWindow::new(item, &[1, 2, 3], &[1, 2, 3]);
    window.respond(1, LootRollChoice::Need, 50);

    // Tick past timeout
    let complete = window.tick(61.0);
    assert!(complete);
    assert!(window.is_complete());
    // Players 2 and 3 auto-passed
    assert_eq!(window.responses.len(), 3);
    assert_eq!(window.resolve(), Some(1)); // only Need roller
}

#[test]
fn roll_window_tick_not_expired() {
    let item = LootDrop {
        item_id: 100,
        count: 1,
    };
    let mut window = RollWindow::new(item, &[1, 2], &[1, 2]);
    let complete = window.tick(30.0);
    assert!(!complete);
    assert_eq!(window.pending_players.len(), 2);
}

#[test]
fn roll_window_all_pass_returns_none() {
    let item = LootDrop {
        item_id: 100,
        count: 1,
    };
    let mut window = RollWindow::new(item, &[1, 2], &[1, 2]);
    window.respond(1, LootRollChoice::Pass, 0);
    window.respond(2, LootRollChoice::Pass, 0);
    assert_eq!(window.resolve(), None);
}

// --- Group loot tests ---

#[test]
fn need_beats_greed() {
    let entries = vec![
        LootRollEntry {
            player_id: 1,
            choice: LootRollChoice::Greed,
            roll_value: 99,
        },
        LootRollEntry {
            player_id: 2,
            choice: LootRollChoice::Need,
            roll_value: 10,
        },
    ];
    assert_eq!(resolve_need_greed(&entries), Some(2));
}

#[test]
fn highest_need_wins() {
    let entries = vec![
        LootRollEntry {
            player_id: 1,
            choice: LootRollChoice::Need,
            roll_value: 50,
        },
        LootRollEntry {
            player_id: 2,
            choice: LootRollChoice::Need,
            roll_value: 80,
        },
    ];
    assert_eq!(resolve_need_greed(&entries), Some(2));
}

#[test]
fn greed_if_no_need() {
    let entries = vec![
        LootRollEntry {
            player_id: 1,
            choice: LootRollChoice::Greed,
            roll_value: 30,
        },
        LootRollEntry {
            player_id: 2,
            choice: LootRollChoice::Greed,
            roll_value: 70,
        },
        LootRollEntry {
            player_id: 3,
            choice: LootRollChoice::Pass,
            roll_value: 99,
        },
    ];
    assert_eq!(resolve_need_greed(&entries), Some(2));
}

#[test]
fn all_pass_returns_none() {
    let entries = vec![
        LootRollEntry {
            player_id: 1,
            choice: LootRollChoice::Pass,
            roll_value: 50,
        },
        LootRollEntry {
            player_id: 2,
            choice: LootRollChoice::Pass,
            roll_value: 80,
        },
    ];
    assert_eq!(resolve_need_greed(&entries), None);
}

#[test]
fn assign_ffa_goes_to_looter() {
    assert_eq!(assign_loot(LootMode::FreeForAll, 42, 0, &[]), Some(42));
}

#[test]
fn assign_round_robin() {
    assert_eq!(assign_loot(LootMode::RoundRobin, 42, 7, &[]), Some(7));
}

#[test]
fn assign_personal_goes_to_looter() {
    assert_eq!(assign_loot(LootMode::PersonalLoot, 42, 0, &[]), Some(42));
}

#[test]
fn assign_need_greed_uses_rolls() {
    let rolls = vec![
        LootRollEntry {
            player_id: 1,
            choice: LootRollChoice::Need,
            roll_value: 60,
        },
        LootRollEntry {
            player_id: 2,
            choice: LootRollChoice::Greed,
            roll_value: 90,
        },
    ];
    assert_eq!(
        assign_loot(LootMode::NeedBeforeGreed, 0, 0, &rolls),
        Some(1)
    );
}

// --- Quality tier / quest item tests ---

#[test]
fn quest_item_drops_for_eligible() {
    let table = LootTable {
        entries: vec![LootEntry {
            item_id: 9999,
            chance: 0.0, // chance ignored for quest items
            min_count: 1,
            max_count: 1,
            entry_type: LootEntryType::Quest,
        }],
    };
    let drops = table.roll(&[0.99], &[0.0], &[9999]); // eligible
    assert_eq!(drops.len(), 1);
    assert_eq!(drops[0].item_id, 9999);
}

#[test]
fn quest_item_hidden_for_ineligible() {
    let table = LootTable {
        entries: vec![LootEntry {
            item_id: 9999,
            chance: 0.0,
            min_count: 1,
            max_count: 1,
            entry_type: LootEntryType::Quest,
        }],
    };
    let drops = table.roll(&[0.0], &[0.0], &[]); // not eligible
    assert!(drops.is_empty());
}

#[test]
fn mixed_normal_and_quest_drops() {
    let table = LootTable {
        entries: vec![
            LootEntry {
                item_id: 100,
                chance: 1.0,
                min_count: 1,
                max_count: 1,
                entry_type: LootEntryType::Normal,
            },
            LootEntry {
                item_id: 9999,
                chance: 0.0,
                min_count: 1,
                max_count: 1,
                entry_type: LootEntryType::Quest,
            },
        ],
    };
    // Eligible for quest
    let drops = table.roll(&[0.5, 0.0], &[0.0, 0.0], &[9999]);
    assert_eq!(drops.len(), 2);

    // Not eligible — only normal item drops
    let drops = table.roll(&[0.5, 0.0], &[0.0, 0.0], &[]);
    assert_eq!(drops.len(), 1);
    assert_eq!(drops[0].item_id, 100);
}

#[test]
fn rare_entry_type_still_uses_chance() {
    let table = LootTable {
        entries: vec![LootEntry {
            item_id: 500,
            chance: 0.01,
            min_count: 1,
            max_count: 1,
            entry_type: LootEntryType::Rare,
        }],
    };
    // High roll = no drop
    let drops = table.roll(&[0.5], &[0.0], &[]);
    assert!(drops.is_empty());
    // Low roll = drops
    let drops = table.roll(&[0.005], &[0.0], &[]);
    assert_eq!(drops.len(), 1);
}
