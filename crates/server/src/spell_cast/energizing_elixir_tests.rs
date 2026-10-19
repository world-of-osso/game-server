//! Definition-level native115288; not a legal talent/loadout certificate.
//! TC a352b1fa StatSystem.cpp901-908 adds aura85/5 after percentage modifiers.
use super::test_harness::{Harness, harness, spawn_player};
use crate::auras::{RemoveReason, has_aura, remove_auras};
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

const MONK: u8 = 10;
const TIGER_PALM: u32 = 100_780;
const ENERGIZING_ELIXIR: u32 = 115_288;
const ADRENALINE_RUSH: u32 = 13_750;
/// Native harness frames are 50ms; no clock overrides in these fixtures.
const FRAMES_PER_SECOND: usize = 20;

fn fixture() -> Harness {
    let mut h = harness(
        Vec::new(),
        &[TIGER_PALM, ENERGIZING_ELIXIR, ADRENALINE_RUSH],
        MONK,
    )
    .expect("native Monk spell/power catalog");
    h.advance(1);
    h
}

fn assert_cast(h: &Harness, spell: u32) {
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.inbox.log.iter().any(|event| {
            event.spell_id == Some(spell)
                && event.source == Some(h.caster.to_bits())
                && event.kind == CombatLogKind::CastSuccess
        }),
        "ordinary spell{spell} succeeds: {:?}",
        h.inbox.log
    );
}

fn spend_energy(h: &mut Harness) {
    h.cast_and_deliver(TIGER_PALM, Some(h.npc));
    assert_cast(h, TIGER_PALM);
    let energy = h.power(PowerType::Energy);
    assert!(
        energy < 100.0,
        "native Tiger Palm spends50 Energy: {energy}"
    );
}

fn cast_elixir(h: &mut Harness) {
    h.cast_and_deliver(ENERGIZING_ELIXIR, None);
    assert_cast(h, ENERGIZING_ELIXIR);
    h.advance(1);
}

fn energy_gain_in_one_second(h: &mut Harness) -> f32 {
    let before = h.power(PowerType::Energy);
    h.advance(FRAMES_PER_SECOND);
    h.power(PowerType::Energy) - before
}

#[test]
fn energizing_elixir_native_unbuffed_tiger_palm_deficit_regenerates_ten_energy_per_second() {
    let mut h = fixture();
    spend_energy(&mut h);
    assert_eq!(energy_gain_in_one_second(&mut h), 10.0);
}

#[test]
#[ignore = "missing native flat power regeneration aura85 consumer"]
fn energizing_elixir_native_cast_regenerates_energy_preserves_chi_and_stops_on_removal() {
    let mut h = fixture();
    spend_energy(&mut h);
    let chi = h.power(PowerType::Chi);
    cast_elixir(&mut h);
    // Native effect1 remains an ordinary +2 Chi energize, not a regen modifier.
    assert_eq!(h.power(PowerType::Chi) - chi, 2.0);
    assert_eq!(energy_gain_in_one_second(&mut h), 25.0, "base10 +75/5");
    assert_eq!(h.power(PowerType::Chi) - chi, 2.0, "Energy selector only");
    let caster = h.caster;
    let removed = remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == ENERGIZING_ELIXIR
    });
    assert_eq!(removed.len(), 1);
    h.advance(1);
    assert_eq!(energy_gain_in_one_second(&mut h), 10.0);
}

#[test]
#[ignore = "missing native flat power regeneration aura85 consumer"]
fn energizing_elixir_native_flat_energy_is_added_after_percentage_regeneration() {
    let mut h = fixture();
    spend_energy(&mut h);
    // Definition-level composition: real13750 has +75% Energy regen and +50
    // max Energy. No Rogue/Monk legal-loadout assertion is made by this fixture.
    h.cast_and_deliver(ADRENALINE_RUSH, None);
    assert_cast(&h, ADRENALINE_RUSH);
    cast_elixir(&mut h);
    assert_eq!(
        energy_gain_in_one_second(&mut h),
        32.5,
        "10*1.75 +75/5, not (10+15)*1.75"
    );
}

#[test]
#[ignore = "missing native flat power regeneration aura85 consumer"]
fn energizing_elixir_native_five_second_expiry_restores_unbuffed_energy() {
    let mut h = fixture();
    cast_elixir(&mut h);
    assert!(has_aura(h.app.world(), h.caster, ENERGIZING_ELIXIR));
    h.advance(5 * FRAMES_PER_SECOND);
    assert!(!has_aura(h.app.world(), h.caster, ENERGIZING_ELIXIR));
    spend_energy(&mut h);
    assert_eq!(energy_gain_in_one_second(&mut h), 10.0);
}

#[test]
#[ignore = "missing native flat power regeneration aura85 consumer"]
fn energizing_elixir_native_foreign_recipient_does_not_regenerate_caster_energy() {
    let mut h = fixture();
    let foreign = spawn_player(&mut h.app, MONK, 1.0);
    h.advance(1);
    // Authenticated native effect on another recipient, without an invented aura.
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: foreign,
            spell_id: ENERGIZING_ELIXIR,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(has_aura(h.app.world(), foreign, ENERGIZING_ELIXIR));
    assert!(!has_aura(h.app.world(), h.caster, ENERGIZING_ELIXIR));
    spend_energy(&mut h);
    assert_eq!(energy_gain_in_one_second(&mut h), 10.0);
}
