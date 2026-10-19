//! Native79949 definition-level Mana85; not Retail52127/legal-build/proc parity.
//! TC a352b1fa StatSystem.cpp874-908: base-mana multipliers, then flat85/5.
use super::test_harness::{Harness, harness, spawn_peer_connection, spawn_player};
use crate::auras::{RemoveReason, has_aura, remove_auras};
use shared::casting::CastState;
use shared::components::{PowerType, UnitPowers};
use shared::protocol::CombatLogKind;

const SHAMAN: u8 = 7;
const WATER_SHIELD: u32 = 79_949;
const WATER_SHIELD_PROC: u32 = 79_950;
const HEALING_SURGE: u32 = 8_004;
const RESTORATION: u32 = 137_039;
const FRAMES_PER_SECOND: usize = 20;
const NATIVE_MANA_MAX: f32 = 10_000.0;

fn fixture() -> Harness {
    let mut h = harness(
        Vec::new(),
        &[WATER_SHIELD, WATER_SHIELD_PROC, HEALING_SURGE, RESTORATION],
        SHAMAN,
    )
    .expect("native Water Shield/Mana catalog");
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
        "ordinary{spell}: {:?}",
        h.inbox.log
    );
}

fn spend_mana(h: &mut Harness) {
    // Real8004 with native137039 cost eligibility; no direct Mana writes.
    h.cast_and_deliver(HEALING_SURGE, Some(h.caster));
    if let Some(cast) = h.app.world().get::<CastState>(h.caster) {
        let remaining = (cast.duration - cast.elapsed).max(0.0);
        let frames = (remaining * FRAMES_PER_SECOND as f32).ceil() as usize;
        h.advance(frames + 1);
    }
    assert_cast(h, HEALING_SURGE);
    assert!(h.power(PowerType::Mana) < NATIVE_MANA_MAX);
}

fn cast_shield(h: &mut Harness) {
    h.cast_and_deliver(WATER_SHIELD, None);
    assert_cast(h, WATER_SHIELD);
    h.advance(1);
    assert!(has_aura(h.app.world(), h.caster, WATER_SHIELD));
}

fn mana_rate(h: &Harness) -> f32 {
    h.app
        .world()
        .get::<UnitPowers>(h.caster)
        .unwrap()
        .entries
        .iter()
        .find(|entry| entry.power == PowerType::Mana)
        .unwrap()
        .regen_per_sec
}

#[test]
fn water_shield_native_unbuffed_mana_deficit_keeps_restoration_base_multiplier() {
    let mut h = fixture();
    spend_mana(&mut h);
    h.advance(1);
    assert_eq!(mana_rate(&h), 125.0, "10000/100 * native137039 1.25");
    let before = h.power(PowerType::Mana);
    h.advance(1);
    assert_eq!(h.power(PowerType::Mana), before + 6.25);
}

#[test]
#[ignore = "missing native flat Mana85 consumer"]
fn water_shield_native_flat_mana_is_added_after_base_multipliers_and_removed() {
    let mut h = fixture();
    spend_mana(&mut h);
    cast_shield(&mut h);
    assert_eq!(mana_rate(&h), 127.0, "100*1.25 +10/5, not (100+2)*1.25");
    let before = h.power(PowerType::Mana);
    h.advance(1);
    assert_eq!(
        h.power(PowerType::Mana),
        before + 6.35,
        "actual native50ms gain"
    );
    let caster = h.caster;
    assert_eq!(
        remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
            aura.spell_id == WATER_SHIELD
        })
        .len(),
        1
    );
    h.advance(1);
    assert_eq!(mana_rate(&h), 125.0, "only the flat contribution ends");
    assert!(
        !h.inbox.log.iter().any(|event| {
            event.spell_id == Some(WATER_SHIELD_PROC) && event.kind == CombatLogKind::Energize
        }),
        "no proc is fabricated during no-hit regeneration"
    );
}

#[test]
#[ignore = "missing native flat Mana85 consumer"]
fn water_shield_native_regeneration_clamps_and_stops_replication_prediction_at_full() {
    let mut h = fixture();
    spend_mana(&mut h);
    cast_shield(&mut h);
    assert_eq!(mana_rate(&h), 127.0);
    h.advance(5 * FRAMES_PER_SECOND);
    assert_eq!(h.power(PowerType::Mana), NATIVE_MANA_MAX);
    assert_eq!(mana_rate(&h), 0.0);
    assert!(has_aura(h.app.world(), h.caster, WATER_SHIELD));
}

#[test]
#[ignore = "missing native flat Mana85 consumer"]
fn water_shield_native_six_hundred_second_expiry_restores_base_mana_rate() {
    let mut h = fixture();
    cast_shield(&mut h);
    // Authored Duration6=600000ms, through unchanged native50ms app frames.
    h.advance(600 * FRAMES_PER_SECOND);
    assert!(!has_aura(h.app.world(), h.caster, WATER_SHIELD));
    spend_mana(&mut h);
    h.advance(1);
    assert_eq!(mana_rate(&h), 125.0);
}

#[test]
#[ignore = "missing native flat Mana85 consumer"]
fn water_shield_native_foreign_self_cast_does_not_change_owner_mana_regeneration() {
    let mut h = fixture();
    spend_mana(&mut h);
    let foreign = spawn_player(&mut h.app, SHAMAN, 1.0);
    h.app
        .world_mut()
        .entity_mut(foreign)
        .insert(crate::class_progression::KnownSpells::fixture(vec![
            WATER_SHIELD,
            WATER_SHIELD_PROC,
        ]));
    let connection = spawn_peer_connection(&mut h.app, foreign, 2);
    h.advance(1);
    let owner = h.caster;
    let owner_connection = h.connection;
    h.caster = foreign;
    h.connection = connection;
    cast_shield(&mut h);
    h.caster = owner;
    h.connection = owner_connection;
    assert!(!has_aura(h.app.world(), owner, WATER_SHIELD));
    assert_eq!(mana_rate(&h), 125.0);
}
