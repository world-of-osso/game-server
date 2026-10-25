//! Pinned SimC76f8118 unique_gear_midnight.cpp3750–3803,6175–6202.
use super::{devastate, item_fixture};
use crate::spell_cast::{ItemCooldown, cast_item_spell};
use shared::protocol::{CombatLogKind, EquipmentSlot};

fn equip(h: &mut super::Harness, slot: EquipmentSlot, item: u32, bonuses: Vec<u32>) {
    h.app
        .world_mut()
        .get_mut::<crate::stats::EquippedItems>(h.caster)
        .unwrap()
        .items
        .push((slot, shared::item_data::ItemDefinitionSource::Retail, item));
    h.app
        .world_mut()
        .get_mut::<crate::stats::EquippedItemModifiers>(h.caster)
        .unwrap()
        .items
        .push((
            slot,
            server::item_instance::ItemInstanceModifiers {
                bonus_list: bonuses,
                ..Default::default()
            },
        ));
    h.advance(3);
}

fn install_native_spec_and_neutral_dummy(h: &mut super::Harness) {
    h.app.insert_resource(
        crate::spell_cast::test_harness::world_db()
            .unwrap()
            .classes
            .clone(),
    );
    let conn = super::native_catalog_connection();
    let faction: u32 = conn
        .query_row(
            "SELECT faction FROM creature_template WHERE entry=31146",
            [],
            |row| row.get(0),
        )
        .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(shared::components::UnitFactionTemplate(faction));
}

#[test]
fn midnight_items_heart_use_enables_morsels_and_native_stacking_stats() {
    let mut h = item_fixture(false);
    install_native_spec_and_neutral_dummy(&mut h);
    equip(&mut h, EquipmentSlot::Trinket2, 270175, vec![]);
    cast_item_spell(
        h.app.world_mut(),
        h.caster,
        1297761,
        Some(h.caster),
        ItemCooldown {
            category: 0,
            category_ms: 0,
        },
        219,
        Some(270175),
    )
    .unwrap();
    h.advance(3);
    let aura = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|a| a.spell_id == 1297761)
        .unwrap();
    assert!(
        aura.effects.iter().any(|e| e.index == 0 && e.amount > 0.0),
        "native on-use strength amount"
    );
    for _ in 0..8 {
        devastate(&mut h);
        if h.inbox
            .log
            .iter()
            .any(|e| e.spell_id == Some(1305374) && e.kind == CombatLogKind::Damage && e.amount > 0)
        {
            break;
        }
    }
    assert!(
        h.inbox.log.iter().any(|e| e.spell_id == Some(1305374)
            && e.kind == CombatLogKind::Damage
            && e.amount > 0),
        "active Heart must produce Morsels"
    );
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 1305376 && a.effects.iter().any(|e| e.amount > 0.0)),
        "Morsels stack native primary stats"
    );
    h.advance(450);
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 1297761));
    assert!(!crate::auras::has_aura(h.app.world(), h.caster, 1305376));
}

#[test]
fn midnight_items_equipped_omnium_fire_emits_native_direct_and_lingering_damage() {
    let mut h = item_fixture(false);
    install_native_spec_and_neutral_dummy(&mut h);
    // Native expansion TraitNodeEntry136822/136817/136814, not waist item spells.
    let caster = h.caster;
    for id in [1279599, 1287555, 1279614] {
        super::apply_passive(&mut h, caster, id);
    }
    for _ in 0..100 {
        devastate(&mut h);
    }
    for id in [1286970, 1287663] {
        assert!(
            h.inbox
                .log
                .iter()
                .any(|e| e.spell_id == Some(id) && e.kind == CombatLogKind::Damage && e.amount > 0),
            "native item damage {id}"
        );
    }
}
