//! TC a352b1fa Spell.cpp4019/4303/4409; native70448 stages, no invented ranks.
use super::kit_tests::at_level;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use bevy::prelude::*;
use shared::casting::{CastState, CastType};
use shared::components::Health;
use shared::protocol::CombatLogKind;

const KYBEL: u32 = 215240;
const FIRE_BREATH: u32 = 357208;
const FIRE_DAMAGE: u32 = 357209;
const ETERNITY_SURGE: u32 = 359073;
const SURGE_DAMAGE: u32 = 359077;

fn prepare(level: u8) -> (Harness, Entity) {
    let (mut h, caster) = at_level(KYBEL, level, |_| {});
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 1_000_000_000.0,
        max: 1_000_000_000.0,
    });
    (h, caster)
}

#[test]
#[ignore = "RED: empower uses ordinary channel duration, without native max hold"]
fn native_empower_clock_holds_without_damage_and_defers_gcd() {
    let (mut h, caster) = prepare(21);
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, FIRE_BREATH, Some(caster), false)
        .unwrap();
    let cast = h.app.world().get::<CastState>(caster).unwrap();
    assert_eq!(cast.cast_type, CastType::Channel);
    // Three native stages1000/750/750ms plus TC Spell.h88's1000ms max hold.
    assert_eq!(cast.duration, 3.5);
    let info = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(FIRE_BREATH)
        .unwrap();
    assert!(
        !h.app
            .world()
            .get::<crate::spell_cast::SpellHistory>(caster)
            .unwrap()
            .on_gcd(info, crate::spell_cast::history_now(h.app.world())),
        "TC defers the root GCD until empower release"
    );
    h.advance(69);
    assert!(h.app.world().get::<CastState>(caster).is_some());
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(caster.to_bits())
                && event.spell_id == Some(FIRE_DAMAGE)
                && event.kind == CombatLogKind::Damage)
    );
    h.advance(1);
    assert!(h.app.world().get::<CastState>(caster).is_none());
}

#[test]
#[ignore = "RED: completed native Fire Breath never dispatches its release child"]
fn native_empower_fire_breath_max_release_damages_and_keeps_rank_three_dot() {
    let (mut h, caster) = prepare(21);
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, FIRE_BREATH, Some(caster), false)
        .unwrap();
    h.advance(90);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(caster.to_bits())
                && event.spell_id == Some(FIRE_DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "max release must execute actual native357209 damage"
    );
    let dot = h
        .app
        .world()
        .get::<ActiveAuras>(h.npc)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == FIRE_DAMAGE && aura.caster == caster)
        .unwrap();
    // TC10 ticks minus3 per extra rank, native2s period; no Blast Furnace at21.
    assert_eq!(dot.duration_ms, 8_000);
}

#[test]
#[ignore = "RED: completed native Eternity Surge never dispatches rank-capped damage"]
fn native_empower_eternity_surge_rank_three_hits_three_of_four_enemies() {
    let (mut h, caster) = prepare(21);
    for index in 0..3 {
        let enemy = spawn_npc(&mut h.app, 4.0 + index as f32);
        h.app.world_mut().entity_mut(enemy).insert(Health {
            current: 1_000_000_000.0,
            max: 1_000_000_000.0,
        });
    }
    let target = h.npc;
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        caster,
        ETERNITY_SURGE,
        Some(target),
        false,
    )
    .unwrap();
    h.advance(100);
    let hits: std::collections::BTreeSet<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.source == Some(caster.to_bits())
                && event.spell_id == Some(SURGE_DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .filter_map(|event| event.target)
        .collect();
    assert_eq!(
        hits.len(),
        3,
        "native rank3 limits359077 to3 targets without Eternity's Span"
    );
}

#[test]
#[ignore = "RED: ordinary creature casts do not resolve native Font of Magic overrides"]
fn native_empower_font_dispatches_both_real_four_stage_variants() {
    for (base, variant) in [(FIRE_BREATH, 382266), (ETERNITY_SURGE, 382411)] {
        let (mut h, caster) = prepare(65);
        assert!(crate::auras::has_aura(h.app.world(), caster, 411212));
        let target = if base == FIRE_BREATH { caster } else { h.npc };
        crate::spell_cast::creature_cast(h.app.world_mut(), caster, base, Some(target), false)
            .unwrap();
        h.advance(1);
        assert!(
            h.inbox
                .go
                .iter()
                .any(|event| event.caster == caster.to_bits() && event.spell_id == variant),
            "Font must cast actual native four-stage variant{variant}, not base{base}"
        );
        let stages = &h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(variant)
            .unwrap()
            .empower_stages;
        assert_eq!(stages.len(), 4);
        assert_eq!(
            stages
                .iter()
                .map(|stage| stage.duration_ms)
                .collect::<Vec<_>>(),
            vec![1000, 750, 750, 750]
        );
    }
}
