//! Real-DB tests against data/world.db: characters created with the Retail kit.

use shared::components::CharacterAppearance;
use shared::protocol::EquipmentSlot as Slot;

use super::*;
use crate::persistence::inventory_store::load_inventory_record;
use crate::persistence::{create_character_with_appearance, temp_database};
use crate::persistence_progression::load_progression;

/// What a created character owns: equipped `(slot, item)`, equipped bags
/// `(container, item)`, backpack `(slot, item, count)` and the stored action bar.
#[derive(Debug, PartialEq)]
struct Created {
    level: u16,
    equipped: Vec<(Slot, u32)>,
    bags: Vec<(u8, u32)>,
    backpack: Vec<(u8, u32, u32)>,
    bar: Vec<(u8, ActionRefData)>,
}

fn create(kit: &OwnedKit, name: &str, race: u8, class: u8) -> Created {
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
        .map(|entry| (Slot::from_visual_slot(entry.slot), entry.item.item_id))
        .collect();
    equipped.sort_by_key(|&(slot, _)| slot);
    let mut appearance: Vec<_> = data
        .equipment_appearance
        .entries
        .iter()
        .map(|entry| (Slot::from_visual_slot(entry.slot), entry.item_id.unwrap()))
        .collect();
    appearance.sort_by_key(|&(slot, _)| slot);
    assert_eq!(
        appearance, equipped,
        "character list shows the equipped items"
    );
    for (id, item) in &record.nonvisual_equipped {
        let slot = Slot::ALL
            .into_iter()
            .find(|slot| slot.inv_slot_id() == *id && slot.visual_slot().is_none())
            .expect("stored nonvisual equipment slot");
        equipped.push((slot, item.item_id));
    }
    equipped.sort_by_key(|&(slot, _)| slot);
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
            assert_eq!(placement.bag, 0, "the kit fits the backpack");
            (placement.slot, item.item_id, item.stack_count)
        })
        .collect();
    backpack.sort();
    assert_eq!(
        backpack.len(),
        record.inventory.items.len(),
        "every item placed"
    );
    let mut bags: Vec<_> = record
        .bag_slots
        .iter()
        .map(|entry| (entry.bag, entry.item.item_id))
        .collect();
    bags.sort();
    Created {
        level: data.level,
        equipped,
        bags,
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
            bags: vec![],
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
            bags: vec![],
            backpack: vec![(0, HEARTHSTONE, 1)],
            // Slam, Attack, Blood Fury, Recuperate.
            bar: spells(&[(0, 1464), (1, 88163), (9, 20572), (10, RECUPERATE)]),
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

/// Loadout 1288 (Human Death Knight, level 8): four Deathweave Bags equip into bag
/// slots 1-4 (`StoreNewItemInBestSlots` -> `FindEquipSlot` INVTYPE_BAG); the choker and
/// two rings equip, leaving ten Black Mushrooms (food: 10 for a Death Knight);
/// playercreateinfo_item (0, 6, 40582, -1) removes the Scourgestone.
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
                (Slot::Neck, 34657),
                (Slot::Shoulder, 34655),
                (Slot::Chest, 34650),
                (Slot::Waist, 34651),
                (Slot::Legs, 34656),
                (Slot::Feet, 34648),
                (Slot::Wrist, 34653),
                (Slot::Hands, 34649),
                (Slot::Finger1, 34658),
                (Slot::Finger2, 38147),
                (Slot::Back, 34659),
            ],
            bags: vec![(1, 38145), (2, 38145), (3, 38145), (4, 38145)],
            backpack: vec![(0, 41751, 10)],
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
    // Large Red Sack equipped; Hearthstone, Haranir Footlocker.
    assert_eq!(created.bags, vec![(1, 157787)]);
    assert_eq!(created.backpack, vec![(0, HEARTHSTONE, 1), (1, 270431, 1)]);
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
            bags: vec![],
            backpack: vec![(0, 77272, 4), (1, HEARTHSTONE, 1)],
            // Tiger Palm, Quaking Palm.
            bar: spells(&[(0, 100780), (9, 107079), (10, RECUPERATE)]),
        }
    );
}

/// Loadout 1647 (Dracthyr Evoker, level 10): Evokers get no weapon; the four Large
/// Green Bags equip into bag slots 1-4; rings, amulet and trinkets equip, leaving
/// only the Hearthstone in the backpack.
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
            (Slot::Neck, 188814),
            (Slot::Chest, 188822),
            (Slot::Legs, 188819),
            (Slot::Feet, 188824),
            (Slot::Wrist, 188823),
            (Slot::Hands, 188821),
            (Slot::Finger1, 188812),
            (Slot::Finger2, 188813),
            (Slot::Trinket1, 188816),
            (Slot::Trinket2, 188817),
            (Slot::Back, 188815),
        ]
    );
    assert_eq!(
        created.bags,
        vec![(1, 188213), (2, 188213), (3, 188213), (4, 188213)]
    );
    assert_eq!(created.backpack, vec![(0, HEARTHSTONE, 1)]);
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
/// waits in the backpack (no dual wield); the four Demon Hide Satchels equip into bag
/// slots 1-4. InventoryType 2 equips the Illidari Chain in Neck, 11 equips the
/// Illidari Ring/Band in Finger1/2, and 12 equips Demon Trophy/Charm of Demonic Fire
/// in Trinket1/2 (TrinityCore a352b1fa Player::StoreNewItemInBestSlots/FindEquipSlot).
/// The backpack holds the second warglaive, four Illidari Rations and Hearthstone.
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
            (Slot::Neck, 112461),
            (Slot::Shoulder, 133318),
            (Slot::Chest, 133322),
            (Slot::Waist, 133325),
            (Slot::Legs, 133319),
            (Slot::Feet, 133324),
            (Slot::Wrist, 133323),
            (Slot::Hands, 133321),
            (Slot::Finger1, 112459),
            (Slot::Finger2, 112460),
            (Slot::Trinket1, 123959),
            (Slot::Trinket2, 123960),
            (Slot::Back, 112462),
            (Slot::MainHand, 112458),
        ]
    );
    assert_eq!(
        created.bags,
        vec![(1, 123958), (2, 123958), (3, 123958), (4, 123958)]
    );
    assert_eq!(
        created.backpack,
        vec![(0, 129181, 1), (1, 132752, 4), (2, HEARTHSTONE, 1)]
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

/// Every playable race/class gets its authored loadout, and every kit item has a
/// template, so creation stores all of them.
#[test]
fn every_playable_pair_has_an_authored_start_loadout_and_creates() {
    // External creation-catalog contract captured by noinstall-2026-10-10.
    #[derive(serde::Deserialize)]
    struct CatalogRace {
        race: u8,
        classes: Vec<u8>,
    }
    let catalog: Vec<CatalogRace> = serde_json::from_str(include_str!(
        "../tests/fixtures/noinstall-creation-catalog.json"
    ))
    .unwrap();
    let catalog_pairs: std::collections::HashSet<_> = catalog
        .into_iter()
        .flat_map(|race| {
            race.classes
                .into_iter()
                .map(move |class| (race.race, class))
        })
        .collect();
    assert_eq!(catalog_pairs.len(), 246);
    let playable_pairs: std::collections::HashSet<_> =
        crate::race_class::playable_pairs().collect();
    assert!(
        catalog_pairs.is_subset(&playable_pairs),
        "every offered catalog pair must be creatable"
    );
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
                kit.kit().new_item(race, item_id, 1, 0).is_ok(),
                "race {race} item {item_id} source-local template"
            );
        }
        kit.kit()
            .starting_location(race, class)
            .unwrap_or_else(|error| panic!("offered pair has no start: {error}"));
        let created = create(&kit, "Probe", race, class);
        let owned = created.equipped.len() + created.bags.len() + created.backpack.len();
        let expected: usize = info
            .items
            .iter()
            .map(|&(item_id, count)| {
                let (_, template, _) = kit.kit().new_item(race, item_id, count, 0).unwrap();
                if template.max_stack > 1 {
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
