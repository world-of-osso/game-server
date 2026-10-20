//! Passive armor specializations need fitting armor in every armor slot: TC a352b1fa
//! Player::HandlePassiveSpellLearn (Player.cpp3104–3117), ApplyItemDependentAuras /
//! RemoveItemDependentAurasAndCasts (8297–8317, 26258–26281) and
//! HasItemFitToSpellRequirements ATTR8_REQUIRES_EQUIPPED_INV_TYPES (26217–26228).
use crate::auras::has_aura;
use crate::spell_cast::test_harness::{Harness, harness_with};
use crate::stats::{CharacterStats, EquippedItems};
use shared::item_data::ItemDefinitionSource;
use shared::protocol::EquipmentSlot;

const PALADIN: u8 = 2;
/// Retribution spec70 Plate Specialization: aura137 +5% Strength.
const PLATE_SPECIALIZATION: u32 = 86_539;
const PLATE_FACTOR: f32 = 1.05;
/// CharacterLoadout164 (Paladin, Purpose0): plate subclass4 in all eight armor slots.
const LOADOUT_164_PLATE: [(EquipmentSlot, u32); 8] = [
    (EquipmentSlot::Head, 72_974),
    (EquipmentSlot::Shoulder, 72_976),
    (EquipmentSlot::Chest, 72_971),
    (EquipmentSlot::Waist, 72_977),
    (EquipmentSlot::Legs, 72_975),
    (EquipmentSlot::Feet, 72_972),
    (EquipmentSlot::Wrist, 72_978),
    (EquipmentSlot::Hands, 72_973),
];
/// CharacterLoadout109 (Druid) leather chest, subclass2.
const LEATHER_CHEST: u32 = 72_689;

fn equipped(items: &[(EquipmentSlot, u32)]) -> EquippedItems {
    EquippedItems {
        items: items
            .iter()
            .map(|&(slot, id)| (slot, ItemDefinitionSource::Retail, id))
            .collect(),
    }
}

fn plate_without_chest() -> Vec<(EquipmentSlot, u32)> {
    LOADOUT_164_PLATE
        .into_iter()
        .filter(|&(slot, _)| slot != EquipmentSlot::Chest)
        .collect()
}

fn paladin(known_passive: bool, items: &[(EquipmentSlot, u32)]) -> Harness {
    let spells: &[u32] = if known_passive {
        &[PLATE_SPECIALIZATION]
    } else {
        &[]
    };
    let mut h = harness_with(Vec::new(), spells, PALADIN, true).unwrap();
    let path = crate::test_data::world_db_path();
    h.app
        .world_mut()
        .insert_resource(crate::item_templates::load_item_templates(&path).unwrap());
    let caster = h.caster;
    h.app.world_mut().entity_mut(caster).insert(equipped(items));
    h.advance(3);
    h
}

fn strength(h: &Harness) -> f32 {
    h.app
        .world()
        .get::<CharacterStats>(h.caster)
        .unwrap()
        .primary
        .strength
}

fn set_items(h: &mut Harness, items: &[(EquipmentSlot, u32)]) {
    let caster = h.caster;
    h.app.world_mut().entity_mut(caster).insert(equipped(items));
    h.advance(3);
}

fn assert_specialized(h: &Harness, unspecialized_strength: f32) {
    assert!(has_aura(h.app.world(), h.caster, PLATE_SPECIALIZATION));
    let expected = unspecialized_strength * PLATE_FACTOR;
    assert!(
        (strength(h) - expected).abs() < 0.01,
        "{} vs {expected}",
        strength(h)
    );
}

fn assert_unspecialized(h: &Harness, unspecialized_strength: f32) {
    assert!(
        !has_aura(h.app.world(), h.caster, PLATE_SPECIALIZATION),
        "Plate Specialization without full plate"
    );
    assert_eq!(strength(h), unspecialized_strength);
}

#[test]
#[ignore = "RED: armor specialization ignores its equipped-armor requirement"]
fn armor_specialization_native_plate_absent_without_armor() {
    let base = strength(&paladin(false, &[]));
    assert_unspecialized(&paladin(true, &[]), base);
}

#[test]
fn armor_specialization_native_full_plate_applies_five_percent_strength() {
    let base = strength(&paladin(false, &LOADOUT_164_PLATE));
    assert_specialized(&paladin(true, &LOADOUT_164_PLATE), base);
}

#[test]
#[ignore = "RED: armor specialization ignores its equipped-armor requirement"]
fn armor_specialization_native_unequipping_chest_removes_and_reequip_restores() {
    let partial = strength(&paladin(false, &plate_without_chest()));
    let full = strength(&paladin(false, &LOADOUT_164_PLATE));
    let mut h = paladin(true, &LOADOUT_164_PLATE);
    assert_specialized(&h, full);
    set_items(&mut h, &plate_without_chest());
    assert_unspecialized(&h, partial);
    set_items(&mut h, &LOADOUT_164_PLATE);
    assert_specialized(&h, full);
}

#[test]
#[ignore = "RED: armor specialization ignores its equipped-armor requirement"]
fn armor_specialization_native_leather_chest_does_not_fit_plate() {
    let mut items = plate_without_chest();
    items.push((EquipmentSlot::Chest, LEATHER_CHEST));
    let base = strength(&paladin(false, &items));
    assert_unspecialized(&paladin(true, &items), base);
}
