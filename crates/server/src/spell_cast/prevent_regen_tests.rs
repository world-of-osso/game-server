//! TC a352b1fa StatSystem.cpp863-869: zero regen for a matching power mask,
//! before percentage/flat terms. Continuous Mana/Energy only; no Rune/pet certificate.
use super::energizing_elixir_tests as energy;
use super::test_harness::{Harness, spawn_peer_connection, spawn_player};
use super::water_shield_mana_tests as mana;
use crate::auras::{RemoveReason, has_aura, remove_auras};
use shared::components::PowerType;

/// Native351520: zero amount, power bitmask8, permanent.
const PREVENT_ENERGY: u32 = 232_865;
/// Native623229: zero amount, bitmask1; sibling623258 is real379 -100%.
const PREVENT_MANA: u32 = 262_173;

fn energy_fixture() -> Harness {
    let mut h = energy::fixture_with_spells(&[PREVENT_ENERGY, PREVENT_MANA]);
    energy::spend_energy(&mut h);
    energy::cast_elixir(&mut h);
    h
}

fn mana_fixture() -> Harness {
    let mut h = mana::fixture_with_spells(&[PREVENT_ENERGY, PREVENT_MANA]);
    mana::spend_mana(&mut h);
    mana::cast_shield(&mut h);
    h
}

fn cast_prevention(h: &mut Harness, spell: u32) {
    h.cast_and_deliver(spell, None);
    energy::assert_cast(h, spell);
    h.advance(1);
    assert!(has_aura(h.app.world(), h.caster, spell));
}

fn remove_prevention(h: &mut Harness, spell: u32) {
    let caster = h.caster;
    let removed = remove_auras(h.app.world_mut(), caster, RemoveReason::Dispelled, |aura| {
        aura.spell_id == spell
    });
    assert_eq!(removed.len(), 1);
    h.advance(1);
}

fn cast_foreign_prevention(h: &mut Harness, class: u8, spell: u32) {
    let foreign = spawn_player(&mut h.app, class, 1.0);
    h.app
        .world_mut()
        .entity_mut(foreign)
        .insert(crate::class_progression::KnownSpells::fixture(vec![spell]));
    let connection = spawn_peer_connection(&mut h.app, foreign, 2);
    h.advance(1);
    let owner = h.caster;
    let owner_connection = h.connection;
    h.caster = foreign;
    h.connection = connection;
    cast_prevention(h, spell);
    h.caster = owner;
    h.connection = owner_connection;
    assert!(!has_aura(h.app.world(), owner, spell));
}

#[test]
fn prevent_regen_native_energy_zero_amount_mask_stops_base_and_flat_but_not_chi() {
    let mut h = energy_fixture();
    assert_eq!(h.power(PowerType::Chi), 2.0);
    cast_prevention(&mut h, PREVENT_ENERGY);
    assert_eq!(energy::energy_gain_in_one_second(&mut h), 0.0);
    assert_eq!(
        h.power(PowerType::Chi),
        2.0,
        "Elixir sibling remains an energize"
    );
    assert!(has_aura(h.app.world(), h.caster, PREVENT_ENERGY));
    remove_prevention(&mut h, PREVENT_ENERGY);
    assert_eq!(energy::energy_gain_in_one_second(&mut h), 25.0);
}

#[test]
fn prevent_regen_native_mana_mask_stops_remaining_flat_after_real_minus100_percent() {
    let mut h = mana_fixture();
    cast_prevention(&mut h, PREVENT_MANA);
    assert_eq!(
        mana::mana_rate(&h),
        0.0,
        "294 also blocks flat85 after native379 -100%"
    );
    let before = mana::mana(&h);
    h.advance(20);
    assert_eq!(mana::mana(&h), before);
    remove_prevention(&mut h, PREVENT_MANA);
    assert_eq!(
        mana::mana_rate(&h),
        127.0,
        "379 sibling and294 leave together"
    );
}

#[test]
fn prevent_regen_native_nonmatching_masks_preserve_other_power_regeneration() {
    let mut h = energy_fixture();
    cast_prevention(&mut h, PREVENT_MANA);
    assert_eq!(energy::energy_gain_in_one_second(&mut h), 25.0);
    let mut h = mana_fixture();
    cast_prevention(&mut h, PREVENT_ENERGY);
    assert_eq!(mana::mana_rate(&h), 127.0);
}

#[test]
fn prevent_regen_native_foreign_energy_selfcast_does_not_freeze_owner() {
    let mut h = energy_fixture();
    cast_foreign_prevention(&mut h, 10, PREVENT_ENERGY);
    assert_eq!(energy::energy_gain_in_one_second(&mut h), 25.0);
}

#[test]
fn prevent_regen_native_foreign_mana_selfcast_does_not_freeze_owner() {
    let mut h = mana_fixture();
    cast_foreign_prevention(&mut h, 7, PREVENT_MANA);
    assert_eq!(mana::mana_rate(&h), 127.0);
}

#[test]
fn prevent_regen_native_removing_other_power_mask_keeps_energy_frozen() {
    let mut h = energy_fixture();
    cast_prevention(&mut h, PREVENT_ENERGY);
    cast_prevention(&mut h, PREVENT_MANA);
    let before = h.power(PowerType::Energy);
    h.advance(1);
    assert_eq!(
        h.power(PowerType::Energy),
        before,
        "both masks remain active"
    );
    remove_prevention(&mut h, PREVENT_MANA);
    assert_eq!(energy::energy_gain_in_one_second(&mut h), 0.0);
    remove_prevention(&mut h, PREVENT_ENERGY);
    assert_eq!(energy::energy_gain_in_one_second(&mut h), 25.0);
}
