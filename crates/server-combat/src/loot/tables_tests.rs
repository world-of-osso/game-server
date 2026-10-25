//! Real-DB tests against data/world.db.

use std::collections::HashSet;

use rand::SeedableRng;
use rand::rngs::StdRng;
use rusqlite::{Connection, OpenFlags};

use super::*;
use crate::item_templates::load_item_templates;

const KOBOLD_VERMIN: u32 = 6;
const DISEASED_YOUNG_WOLF: u32 = 299;
/// Reference 20002 "Beast 1-5": Ruined Pelt, Broken Fang, Chipped Claw (group 1, 100%).
const BEAST_PARTS: [u32; 3] = [4865, 7073, 7074];
/// Reference 20000 "Grey 1-5": 23 equal-chance items in group 1.
const GREY_1_5: u32 = 20000;
/// QuestRequired drop of the Diseased Young Wolf.
const DISEASED_WOLF_PELT: u32 = 50432;

fn world_db() -> Option<(LootTables, ItemTemplates)> {
    let path = crate::test_data::world_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    Some((
        load_loot_tables(&conn).unwrap(),
        load_item_templates(&path).unwrap(),
    ))
}

fn ids(drops: &[DroppedItem]) -> Vec<u32> {
    drops.iter().map(|drop| drop.item_id).collect()
}

#[test]
fn kobold_vermin_drops_its_1_to_5_copper_every_kill() {
    let Some((tables, items)) = world_db() else {
        return;
    };
    let mut rng = StdRng::seed_from_u64(1);
    let coins: HashSet<u64> = (0..200)
        .map(|_| {
            tables
                .roll_creature(KOBOLD_VERMIN, 0, &items, |_| false, &mut rng)
                .copper
        })
        .collect();
    assert_eq!(coins, (1..=5).collect());
}

#[test]
fn diseased_wolf_always_drops_one_beast_part_and_no_coins() {
    let Some((tables, items)) = world_db() else {
        return;
    };
    let mut rng = StdRng::seed_from_u64(2);
    for _ in 0..200 {
        let loot = tables.roll_creature(DISEASED_YOUNG_WOLF, 0, &items, |_| false, &mut rng);
        assert_eq!(loot.copper, 0, "mingold = maxgold = 0");
        let parts = ids(&loot.items)
            .into_iter()
            .filter(|id| BEAST_PARTS.contains(id))
            .count();
        // The 100% reference always yields one; the grouped 30% reference may add one.
        assert!((1..=2).contains(&parts), "{:?}", ids(&loot.items));
    }
}

#[test]
fn quest_pelt_drops_only_for_a_player_on_the_quest() {
    let Some((tables, items)) = world_db() else {
        return;
    };
    let mut rng = StdRng::seed_from_u64(3);
    let without: usize = (0..200)
        .map(|_| {
            let loot = tables.roll_creature(DISEASED_YOUNG_WOLF, 0, &items, |_| false, &mut rng);
            ids(&loot.items).contains(&DISEASED_WOLF_PELT) as usize
        })
        .sum();
    assert_eq!(without, 0);
    let with: usize = (0..200)
        .map(|_| {
            let loot = tables.roll_creature(
                DISEASED_YOUNG_WOLF,
                0,
                &items,
                |item| item == DISEASED_WOLF_PELT,
                &mut rng,
            );
            ids(&loot.items).contains(&DISEASED_WOLF_PELT) as usize
        })
        .sum();
    // 90% chance.
    assert!((160..=199).contains(&with), "{with} of 200");
}

#[test]
fn equal_chance_group_yields_exactly_one_item() {
    let Some((tables, items)) = world_db() else {
        return;
    };
    let grey = &tables.reference[&GREY_1_5];
    let members: HashSet<u32> = grey.groups[0].iter().map(|row| row.item).collect();
    assert_eq!(members.len(), 23);
    let mut rng = StdRng::seed_from_u64(4);
    let mut seen = HashSet::new();
    for _ in 0..500 {
        let mut drops = Vec::new();
        tables.process(grey, &items, &|_| false, &mut rng, &mut drops);
        assert_eq!(drops.len(), 1);
        assert!(members.contains(&drops[0].item_id));
        seen.insert(drops[0].item_id);
    }
    assert!(
        seen.len() > 15,
        "picks spread over the group: {}",
        seen.len()
    );
}

#[test]
fn counts_above_the_stack_size_split_into_stacks() {
    let row = LootRow {
        item: 159,
        reference: 0,
        chance: 100.0,
        needs_quest: false,
        group: 0,
        min_count: 45,
        max_count: 45,
    };
    let mut items = ItemTemplates::default();
    items.retail.insert(
        159,
        ItemTemplate {
            max_stack: 20,
            ..Default::default()
        },
    );
    let tables = LootTables {
        creature: [(
            1,
            LootTemplate {
                entries: vec![row],
                groups: Vec::new(),
            },
        )]
        .into(),
        money: [(
            1,
            CreatureLoot {
                loot_id: 1,
                min_gold: 0,
                max_gold: 0,
            },
        )]
        .into(),
        ..Default::default()
    };
    let loot = tables.roll_creature(1, 0, &items, |_| false, &mut StdRng::seed_from_u64(5));
    let counts: Vec<u32> = loot.items.iter().map(|drop| drop.count).collect();
    assert_eq!(counts, [20, 20, 5]);
}

#[test]
fn money_follows_generate_money_loot() {
    let mut rng = StdRng::seed_from_u64(6);
    assert_eq!(money_loot(0, 0, &mut rng), 0);
    assert_eq!(money_loot(9, 7, &mut rng), 7, "max <= min pays max");
    for _ in 0..50 {
        assert!((1..=5).contains(&money_loot(1, 5, &mut rng)));
    }
}

const DEFIAS_THUG: u32 = 38;

fn chance_of(chances: &[(u32, f32)], item: u32) -> f32 {
    chances
        .iter()
        .find(|(id, _)| *id == item)
        .map(|(_, chance)| *chance)
        .unwrap_or_else(|| panic!("item {item} missing from {chances:?}"))
}

#[test]
fn defias_thug_drop_chances_resolve_references_and_groups() {
    let Some((tables, items)) = world_db() else {
        return;
    };
    let chances = tables.drop_chances(DEFIAS_THUG, 0, &items);

    let close = |item: u32, expected: f32| {
        let chance = chance_of(&chances, item);
        assert!(
            (chance - expected).abs() < 1e-3,
            "{item}: {chance} vs {expected}"
        );
    };
    // Own rows: Red Burlap Bandana (quest) 60 %, Pitted Defias Shortsword 2 %.
    close(752, 60.0);
    close(2057, 2.0);
    // Reference 20017 at 30 %: its group rolls Darnassian Bleu 67 %, Refreshing
    // Spring Water 33 %.
    close(2070, 30.0 * 0.67);
    close(159, 30.0 * 0.33);
    // Reference 11111 at 0.2 %: five equal-chance pouches.
    for pouch in [805, 828, 4496, 5571, 5572] {
        close(pouch, 0.2 / 5.0);
    }
    // Group 1 reference 20000 at 30 %: 23 equal-chance grey items.
    close(1364, 30.0 / 23.0);
    assert_eq!(chances.len(), 2 + 2 + 5 + 23);
    assert!(
        chances.windows(2).all(|pair| pair[0].1 >= pair[1].1),
        "highest chance first: {chances:?}"
    );
}

/// Lumberjack (842): `mingold` 40, `maxgold` 57, `lootid` 0.
const LUMBERJACK: u32 = 842;

#[test]
fn a_creature_without_a_loot_table_drops_nothing_not_even_coins() {
    let Some((tables, items)) = world_db() else {
        return;
    };
    let mut rng = StdRng::seed_from_u64(3);

    let rolled = tables.roll_creature(LUMBERJACK, 0, &items, |_| true, &mut rng);

    assert_eq!(rolled, RolledLoot::default());
    assert!(tables.drop_chances(LUMBERJACK, 0, &items).is_empty());
}

/// Hogger (46254) has no TrinityCore loot (Normal LootID 0): he drops Wowhead's
/// stockade-20260926 Normal-mode rates, e.g. Corpse Rompers 41,982 / 135,040.
#[test]
fn stockade_hogger_drops_his_wowhead_items_at_their_rates() {
    let Some((tables, items)) = world_db() else {
        return;
    };
    let chances = tables.drop_chances(46254, 0, &items);
    let chance = |item| chances.iter().find(|(id, _)| *id == item).map(|c| c.1);
    let rompers = chance(2168).expect("Corpse Rompers");
    assert!((rompers - 31.089).abs() < 0.01, "{rompers}");
    assert_eq!(chances.len(), 5, "{chances:?}");
    let mut rng = StdRng::seed_from_u64(7);
    let dropped: HashSet<u32> = (0..200)
        .flat_map(|_| {
            ids(&tables
                .roll_creature(46254, 0, &items, |_| false, &mut rng)
                .items)
        })
        .collect();
    assert_eq!(dropped, HashSet::from([2168, 1934, 1959, 132569, 151074]));
}
