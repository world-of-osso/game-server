//! Using real food and water from the bags on real 50 ms ticks against data/world.db
//! (skipped when absent or without its `item_effect` tables).

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{CombatStatus, Health, PlayerStandState, PowerType, StandState};
use shared::protocol::{
    CastFailed, InventoryChannel, InventoryError, InventoryErrorReason, ItemLocation, UseItem,
};
use shared::spell_data::CastFailReason;

use super::*;
use crate::auras::ActiveAuras;
use crate::persistence::{self, InventoryData, InventoryItemData};
use crate::spell_cast::test_harness::*;

const MAGE: u8 = 8;
/// Tough Hunk of Bread: on use Food 1291822 (category 11 "Item - Food", 1 s category
/// cooldown, expendable).
const BREAD: u32 = 4540;
const BREAD_FOOD: u32 = 1_291_822;
/// Refreshing Spring Water: on use Drink 1291950 (category 59 "Item - Drink").
const WATER: u32 = 159;
const WATER_DRINK: u32 = 1_291_950;

struct Eater {
    h: Harness,
    character_id: u64,
}

fn world_db() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/world.db")
}

/// A level 10 mage out of combat at 100/1000 health with `count` of each of `items` in
/// its backpack, in order from slot 0.
fn eater(items: &[u32], count: u32) -> Option<Eater> {
    let mut h = harness(Vec::new(), &[BREAD_FOOD, WATER_DRINK], MAGE)?;
    let on_use = match load_item_on_use_spells(&world_db()) {
        Ok(spells) => spells,
        Err(error) => {
            eprintln!("skipping: world.db has no item_effect data: {error}");
            return None;
        }
    };
    let db = persistence::temp_database();
    let (character_id, _) = persistence::create_character(&db, 1, "Eater", 1, MAGE).unwrap();
    let items = items
        .iter()
        .map(|&item_id| InventoryItemData {
            item_guid: persistence::next_id(&db, "item_instance"),
            item_id,
            name: String::new(),
            quality: 1,
            required_level: 1,
            stack_count: count,
            vendor_sell_price: 1,
            unique_auction: false,
        })
        .collect();
    persistence::save_inventory(&db, character_id, &InventoryData { gold: 0, items });
    let templates = crate::item_templates::load_item_templates(&world_db()).unwrap();
    let app = &mut h.app;
    app.insert_resource(crate::persistence::GameDatabase { db });
    app.insert_resource(templates);
    app.insert_resource(on_use);
    app.init_resource::<crate::gold::GoldUpdateQueue>();
    app.add_systems(Update, process_use_requests);
    app.world_mut().entity_mut(h.connection).insert((
        crate::auth::InWorld { character_id },
        MessageSender::<UseItem>::default(),
        MessageReceiver::<UseItem>::default(),
        MessageSender::<InventoryError>::default(),
        MessageReceiver::<InventoryError>::default(),
    ));
    h.advance(1);
    let caster = h.caster;
    app_world(&mut h).get_mut::<CombatStatus>(caster).unwrap().0 = false;
    app_world(&mut h).entity_mut(caster).insert(Health {
        current: 100.0,
        max: 1000.0,
    });
    Some(Eater { h, character_id })
}

fn app_world(h: &mut Harness) -> &mut World {
    h.app.world_mut()
}

fn slot(slot: u8) -> ItemLocation {
    ItemLocation::Bag { bag: 0, slot }
}

/// `CMSG_USE_ITEM` for the backpack slot; it arrives and resolves 3 frames later.
fn use_slot(eater: &mut Eater, index: u8) {
    let connection = eater.h.connection;
    app_world(&mut eater.h)
        .get_mut::<MessageSender<UseItem>>(connection)
        .unwrap()
        .send::<InventoryChannel>(UseItem {
            location: slot(index),
            target: None,
        });
    eater.h.advance(3);
}

fn count_of(eater: &Eater, item_id: u32) -> u32 {
    let db = &eater.h.app.world().resource::<crate::persistence::GameDatabase>().db;
    persistence::load_inventory(db, eater.character_id)
        .items
        .iter()
        .filter(|item| item.item_id == item_id)
        .map(|item| item.stack_count)
        .sum()
}

fn auras(eater: &Eater) -> Vec<u32> {
    eater
        .h
        .app
        .world()
        .get::<ActiveAuras>(eater.h.caster)
        .map(|auras| auras.iter().map(|aura| aura.spell_id).collect())
        .unwrap_or_default()
}

fn errors(eater: &mut Eater) -> Vec<InventoryError> {
    drain::<InventoryError>(&mut eater.h.app, eater.h.connection)
}

/// Eating the bread casts its food on the player, who sits down and heals 3% of max
/// health every second while seated; one bread is used up. Its 1 s food category
/// cooldown refuses a second bread at once (`SPELL_FAILED_NOT_READY`), not a second
/// later.
#[test]
fn eating_bread_sits_the_player_heals_it_and_uses_one_bread() {
    let Some(mut eater) = eater(&[BREAD], 5) else {
        return;
    };
    use_slot(&mut eater, 0);
    assert!(errors(&mut eater).is_empty());
    assert_eq!(auras(&eater), vec![BREAD_FOOD]);
    assert_eq!(
        eater.h.app.world().get::<PlayerStandState>(eater.h.caster),
        Some(&PlayerStandState(StandState::Sit))
    );
    assert_eq!(count_of(&eater, BREAD), 4);
    // In combat nothing else heals: two 1 s food ticks of 3% of 1000.
    let caster = eater.h.caster;
    app_world(&mut eater.h)
        .get_mut::<CombatStatus>(caster)
        .unwrap()
        .0 = true;
    let before = eater.h.health(caster);
    eater.h.advance(41);
    assert_eq!(eater.h.health(caster) - before, 60.0);
    app_world(&mut eater.h)
        .get_mut::<CombatStatus>(caster)
        .unwrap()
        .0 = false;

    use_slot(&mut eater, 0);
    let failed: Vec<CastFailed> = std::mem::take(&mut eater.h.inbox.failed);
    assert!(failed.is_empty(), "the 1 s cooldown ran out: {failed:?}");
    assert_eq!(count_of(&eater, BREAD), 3);
    use_slot(&mut eater, 0);
    let failed = std::mem::take(&mut eater.h.inbox.failed);
    assert_eq!(failed.len(), 1, "{failed:?}");
    assert_eq!(failed[0].reason, CastFailReason::OnCooldown);
    assert_eq!(count_of(&eater, BREAD), 3);
}

/// `Player::ProcessItemCast`: the food spell is `SPELL_ATTR0_NOT_IN_COMBAT_ONLY_PEACEFUL`,
/// so eating in combat is refused with `EQUIP_ERR_NOT_IN_COMBAT` and keeps the bread.
#[test]
fn bread_cannot_be_eaten_in_combat() {
    let Some(mut eater) = eater(&[BREAD], 5) else {
        return;
    };
    let caster = eater.h.caster;
    app_world(&mut eater.h)
        .get_mut::<CombatStatus>(caster)
        .unwrap()
        .0 = true;
    use_slot(&mut eater, 0);
    let refused = errors(&mut eater);
    assert_eq!(refused.len(), 1, "{refused:?}");
    assert_eq!(refused[0].reason, InventoryErrorReason::NotInCombat);
    assert!(auras(&eater).is_empty());
    assert_eq!(count_of(&eater, BREAD), 5);
}

/// The water's drink restores 1% of max mana on apply (`SPELL_ATTR5_EXTRA_INITIAL_PERIOD`)
/// and every second after, beside the bread's food.
#[test]
fn water_restores_a_percent_of_mana_each_second_beside_food() {
    let Some(mut eater) = eater(&[BREAD, WATER], 5) else {
        return;
    };
    eater.h.set_power(PowerType::Mana, 0.0);
    let max = {
        let pools = eater
            .h
            .app
            .world()
            .get::<crate::spell_cast::powers::PowerPools>(eater.h.caster)
            .unwrap();
        pools.get(PowerType::Mana).unwrap().max
    };
    use_slot(&mut eater, 1);
    assert_eq!(auras(&eater), vec![WATER_DRINK]);
    assert_eq!(count_of(&eater, WATER), 4);
    let first = eater.h.power(PowerType::Mana);
    assert!(first >= (max / 100.0).trunc(), "{first} of {max}");
    use_slot(&mut eater, 0);
    let mut active = auras(&eater);
    active.sort_unstable();
    assert_eq!(active, vec![BREAD_FOOD, WATER_DRINK]);
}
