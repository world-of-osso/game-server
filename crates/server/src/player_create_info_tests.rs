//! Real-DB tests against data/world.db: characters created with the Retail kit.
//! Each test skips when the DB is absent.

use shared::components::{CharacterAppearance, EquipmentVisualSlot as Slot};
use shared::protocol::EquipmentSlot;

use super::*;
use crate::persistence::inventory_store::load_inventory_record;
use crate::persistence::{create_character_with_appearance, temp_database};
use crate::persistence_progression::load_progression;

/// What a created character owns: equipped `(slot, item)`, backpack `(slot, item, count)`
/// and the stored action bar.
#[derive(Debug, PartialEq)]
struct Created {
    level: u16,
    equipped: Vec<(Slot, u32)>,
    backpack: Vec<(u8, u32, u32)>,
    bar: Vec<(u8, ActionRefData)>,
}

pub(crate) fn create(kit: &OwnedKit, name: &str, race: u8, class: u8) -> Created {
    let db = temp_database();
    let (id, data) = create_character_with_appearance(
        &db,
        1,
        name,
        race,
        class,
        CharacterAppearance::default(),
        &kit.kit(),
    )
    .unwrap();
    let record = load_inventory_record(&db, id).unwrap();
    assert_eq!(
        record.character, data,
        "returned character is the stored one"
    );
    let mut equipped: Vec<_> = record
        .equipped
        .iter()
        .map(|entry| (entry.slot, entry.item.item_id))
        .collect();
    equipped.sort_by_key(|&(slot, _)| EquipmentSlot::from_visual_slot(slot));
    let mut appearance: Vec<_> = data
        .equipment_appearance
        .entries
        .iter()
        .map(|entry| (entry.slot, entry.item_id.unwrap()))
        .collect();
    appearance.sort_by_key(|&(slot, _)| EquipmentSlot::from_visual_slot(slot));
    assert_eq!(
        appearance, equipped,
        "character list shows the equipped items"
    );
    let mut backpack: Vec<_> = record
        .placements
        .iter()
        .map(|placement| {
            let item = record
                .inventory
                .items
                .iter()
                .find(|item| item.item_guid == placement.item_guid)
                .unwrap();
            assert_eq!(placement.bag, 0, "bags 1-4 hold nothing");
            (placement.slot, item.item_id, item.stack_count)
        })
        .collect();
    backpack.sort();
    assert_eq!(
        backpack.len(),
        record.inventory.items.len(),
        "every item placed"
    );
    Created {
        level: data.level,
        equipped,
        backpack,
        bar: load_progression(&db, id).unwrap().action_bar,
    }
}

fn spells(buttons: &[(u8, u32)]) -> Vec<(u8, ActionRefData)> {
    buttons
        .iter()
        .map(|&(slot, spell)| (slot, ActionRefData::Spell(spell)))
        .collect()
}

const HEARTHSTONE: u32 = 6948;
const RECUPERATE: u32 = 1231411;

/// CharacterLoadout 1277 (Human Mage, Purpose 9) and playercreateinfo_action (1, 8).
#[test]
fn human_mage_equips_the_mage_loadout_and_keeps_the_hearthstone_in_the_backpack() {
    let Some(kit) = OwnedKit::from_world_db() else {
        return;
    };
    assert_eq!(
        create(&kit, "Elara", 1, 8),
        Created {
            level: 1,
            equipped: vec![
                (Slot::Chest, 187761), // Mage's Robes
                (Slot::Waist, 187760), // Mage's Sash
                (Slot::Legs, 187758),  // Mage's Trousers
                (Slot::Feet, 187759),  // Mage's Slippers
                (Slot::Wrist, 187757), // Mage's Wristwraps
                (Slot::Hands, 187762), // Mage's Mitts
                (Slot::MainHand, 35),  // Bent Staff
            ],
            backpack: vec![(0, HEARTHSTONE, 1)],
            // Frostbolt, Will to Survive.
            bar: spells(&[(0, 116), (9, 59752), (10, RECUPERATE)]),
        }
    );
}

/// The Hearthstone comes first in loadout 1274 (Orc Warrior) and the shield equips
/// beside the one-hander.
#[test]
fn orc_warrior_equips_sword_and_shield() {
    let Some(kit) = OwnedKit::from_world_db() else {
        return;
    };
    assert_eq!(
        create(&kit, "Grukk", 2, 1),
        Created {
            level: 1,
            equipped: vec![
                (Slot::Chest, 187748),
                (Slot::Waist, 187745),
                (Slot::Legs, 187744),
                (Slot::Feet, 187743),
                (Slot::Wrist, 187747),
                (Slot::Hands, 187746),
                (Slot::MainHand, 25),  // Worn Shortsword
                (Slot::OffHand, 2362), // Worn Wooden Shield
            ],
            backpack: vec![(0, HEARTHSTONE, 1)],
            // Slam, Blood Fury, Recuperate; Attack and Blood Fury on the stance bar.
            bar: spells(&[
                (0, 1464),
                (9, 20572),
                (10, RECUPERATE),
                (72, 88163),
                (81, 20572)
            ]),
        }
    );
}

/// Loadout 1304: the second dagger has no off-hand slot because the server does not
/// track dual wield (TrinityCore equips it for a Rogue).
#[test]
fn human_rogue_keeps_the_second_dagger_in_the_backpack() {
    let Some(kit) = OwnedKit::from_world_db() else {
        return;
    };
    let created = create(&kit, "Vanna", 1, 4);
    assert!(
        created.equipped.contains(&(Slot::MainHand, 2092)),
        "Worn Dagger"
    );
    assert_eq!(created.backpack, vec![(0, 50055, 1), (1, HEARTHSTONE, 1)]);
    assert_eq!(
        created.bar,
        spells(&[(0, 1752), (9, 59752), (10, RECUPERATE)])
    );
}

/// Loadout 1288 (Human Death Knight, level 8): four Deathweave Bags (bags 1-4 have no
/// container support), jewelry (no neck/finger storage), ten Black Mushrooms (food:
/// 10 for a Death Knight); playercreateinfo_item (0, 6, 40582, -1) removes the
/// Scourgestone.
#[test]
fn human_death_knight_gets_plate_bags_jewelry_and_ten_mushrooms() {
    let Some(kit) = OwnedKit::from_world_db() else {
        return;
    };
    assert_eq!(
        create(&kit, "Mordrek", 1, 6),
        Created {
            level: 8,
            equipped: vec![
                (Slot::Head, 34652),
                (Slot::Shoulder, 34655),
                (Slot::Chest, 34650),
                (Slot::Waist, 34651),
                (Slot::Legs, 34656),
                (Slot::Feet, 34648),
                (Slot::Wrist, 34653),
                (Slot::Hands, 34649),
                (Slot::Back, 34659),
            ],
            backpack: vec![
                (0, 38145, 1),
                (1, 38145, 1),
                (2, 38145, 1),
                (3, 38145, 1),
                (4, 34657, 1), // Choker of Damnation
                (5, 34658, 1), // Plague Band
                (6, 38147, 1), // Corrupted Band
                (7, 41751, 10),
            ],
            // Rune Strike, Death Coil, Death and Decay, Blood Boil, Death Grip; button 2's
            // spell 45477 is not in Retail SpellName, so IsActionButtonDataValid drops it.
            bar: spells(&[
                (0, 316239),
                (1, 47541),
                (3, 43265),
                (4, 50842),
                (5, 49576),
                (10, RECUPERATE),
                (11, 59752),
            ]),
        }
    );
}

/// Loadout 2074: the Haranir gear's AllowableRaces use PlayableRaceBit 19 for race 91.
#[test]
fn haranir_hunter_equips_race_locked_gear_and_bow() {
    let Some(kit) = OwnedKit::from_world_db() else {
        return;
    };
    let created = create(&kit, "Thornwood", 91, 3);
    assert_eq!(created.level, 10);
    assert_eq!(
        created.equipped,
        vec![
            (Slot::Chest, 257525),
            (Slot::Waist, 257590),
            (Slot::Legs, 257555),
            (Slot::Feet, 257551),
            (Slot::Wrist, 257594),
            (Slot::Hands, 257533),
            (Slot::MainHand, 266263), // Wildgrove Haranir Longbow
        ]
    );
    // Large Red Sack, Hearthstone, Haranir Footlocker.
    assert_eq!(
        created.backpack,
        vec![(0, 157787, 1), (1, HEARTHSTONE, 1), (2, 270431, 1)]
    );
    assert_eq!(
        created.bar,
        spells(&[
            (0, 56641),
            (1, 185358),
            (2, 257284),
            (3, 195645),
            (4, 781),
            (5, 5384),
            (6, 187650),
            (7, 109304),
            (8, 186265),
            (9, 186257),
        ])
    );
}

/// Loadout 1324: Small Sugarcane Stalk is food, so 4 of them.
#[test]
fn pandaren_monk_gets_four_sugarcane_stalks() {
    let Some(kit) = OwnedKit::from_world_db() else {
        return;
    };
    assert_eq!(
        create(&kit, "Mei", 24, 10),
        Created {
            level: 1,
            equipped: vec![
                (Slot::Chest, 72019),
                (Slot::Waist, 77509),
                (Slot::Legs, 72020),
                (Slot::Wrist, 77526),
            ],
            backpack: vec![(0, 77272, 4), (1, HEARTHSTONE, 1)],
            // Tiger Palm, Quaking Palm.
            bar: spells(&[(0, 100780), (9, 107079), (10, RECUPERATE)]),
        }
    );
}

/// Loadout 1647 (Dracthyr Evoker, level 10): Evokers get no weapon; the four bags,
/// rings, amulet and trinkets stay in the backpack.
#[test]
fn dracthyr_evoker_wears_mail_and_carries_bags_and_jewelry() {
    let Some(kit) = OwnedKit::from_world_db() else {
        return;
    };
    let created = create(&kit, "Azuregos", 52, 13);
    assert_eq!(created.level, 10);
    assert_eq!(
        created.equipped,
        vec![
            (Slot::Head, 188820),
            (Slot::Chest, 188822),
            (Slot::Legs, 188819),
            (Slot::Feet, 188824),
            (Slot::Wrist, 188823),
            (Slot::Hands, 188821),
            (Slot::Back, 188815),
        ]
    );
    assert_eq!(
        created.backpack,
        vec![
            (0, 188213, 1),
            (1, 188812, 1),
            (2, 188813, 1),
            (3, 188814, 1),
            (4, 188816, 1),
            (5, 188817, 1),
            (6, 188213, 1),
            (7, 188213, 1),
            (8, 188213, 1),
            (9, HEARTHSTONE, 1),
        ]
    );
    assert_eq!(
        created.bar,
        spells(&[
            (0, 361469),
            (1, 362969),
            (2, 357214),
            (3, 368970),
            (10, 358733)
        ])
    );
}

/// Loadout 1346 (Night Elf Demon Hunter, level 8): one Warglaive equips, the second
/// waits in the backpack (no dual wield); Illidari Rations are food, 4.
#[test]
fn night_elf_demon_hunter_equips_one_warglaive() {
    let Some(kit) = OwnedKit::from_world_db() else {
        return;
    };
    let created = create(&kit, "Varedis", 4, 12);
    assert_eq!(created.level, 8);
    assert_eq!(
        created.equipped,
        vec![
            (Slot::Head, 133320),
            (Slot::Shoulder, 133318),
            (Slot::Chest, 133322),
            (Slot::Waist, 133325),
            (Slot::Legs, 133319),
            (Slot::Feet, 133324),
            (Slot::Wrist, 133323),
            (Slot::Hands, 133321),
            (Slot::Back, 112462),
            (Slot::MainHand, 112458),
        ]
    );
    assert_eq!(
        created.backpack,
        vec![
            (0, 123958, 1),
            (1, 123958, 1),
            (2, 123958, 1),
            (3, 123958, 1),
            (4, 112459, 1),
            (5, 112460, 1),
            (6, 112461, 1),
            (7, 129181, 1),
            (8, 123959, 1),
            (9, 123960, 1),
            (10, 132752, 4),
            (11, HEARTHSTONE, 1),
        ]
    );
    assert_eq!(
        created.bar,
        spells(&[
            (0, 344865),
            (1, 344859),
            (2, 344862),
            (9, 58984),
            (10, 188501),
            (11, 200175)
        ])
    );
}

/// Every playable race/class gets a Retail loadout, and every kit item has a
/// template, so creation stores all of them.
#[test]
fn every_playable_pair_has_a_loadout_and_creates() {
    let Some(kit) = OwnedKit::from_world_db() else {
        return;
    };
    let mut empty = Vec::new();
    for (race, class) in crate::race_class::playable_pairs() {
        let info = kit.create_info.get(race, class).unwrap();
        if info.items.is_empty() {
            empty.push((race, class));
        }
        for &(item_id, _) in &info.items {
            assert!(
                kit.templates.get(item_id).is_some(),
                "item {item_id} template"
            );
        }
        let created = create(&kit, "Probe", race, class);
        let owned = created.equipped.len() + created.backpack.len();
        let expected: usize = info
            .items
            .iter()
            .map(|&(item_id, count)| {
                if kit.templates.get(item_id).unwrap().max_stack > 1 {
                    1
                } else {
                    count as usize
                }
            })
            .sum();
        assert_eq!(
            owned, expected,
            "race {race} class {class} owns every kit item"
        );
    }
    assert_eq!(empty, Vec::<(u8, u8)>::new());
}
