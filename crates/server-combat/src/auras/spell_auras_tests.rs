//! Real catalog applications followed by production CC, damage and speed paths.
use bevy::prelude::*;
use shared::casting::CastState;

use super::{ActiveAuras, AuraApplication, AuraSource, AuraStatMods, RemoveReason};
use crate::spell_cast::effects::{EffectCtx, deal_damage};
use crate::spell_cast::test_harness::{Harness, harness};
use crate::spell_info::SpellStore;

const FIREBALL: u32 = 133;
const HAMMER_OF_JUSTICE: u32 = 853;
const HAMSTRING: u32 = 1715;
const ICEBOUND_FORTITUDE: u32 = 48792;
const BARKSKIN: u32 = 22812;
const PILLAR_OF_FROST: u32 = 51271;

fn apply(h: &mut Harness, spell_id: u32, caster: Entity, target: Entity) {
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(spell_id)
        .unwrap();
    let effect_mask = super::aura_effect_mask(spell);
    super::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask,
            source: AuraSource::Cast,
        },
    );
}

fn has(h: &Harness, spell_id: u32) -> bool {
    h.app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .any(|aura| aura.spell_id == spell_id)
}

#[test]
fn spell_auras_77_removes_stun_and_rejects_reapplication_but_not_snare() {
    let mut h = harness(
        Vec::new(),
        &[ICEBOUND_FORTITUDE, HAMMER_OF_JUSTICE, HAMSTRING],
        6,
    )
    .unwrap();
    let (caster, npc) = (h.caster, h.npc);
    apply(&mut h, HAMMER_OF_JUSTICE, npc, caster);
    apply(&mut h, HAMSTRING, npc, caster);
    assert!(has(&h, HAMMER_OF_JUSTICE));
    assert_eq!(
        h.app
            .world()
            .get::<AuraStatMods>(caster)
            .unwrap()
            .movement_speed,
        0.0
    );
    apply(&mut h, ICEBOUND_FORTITUDE, caster, caster);
    assert!(!has(&h, HAMMER_OF_JUSTICE));
    assert!(has(&h, HAMSTRING));
    assert_eq!(
        h.app
            .world()
            .get::<AuraStatMods>(caster)
            .unwrap()
            .movement_speed,
        0.5
    );
    apply(&mut h, HAMMER_OF_JUSTICE, npc, caster);
    assert!(!has(&h, HAMMER_OF_JUSTICE));
    super::remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == ICEBOUND_FORTITUDE
    });
    apply(&mut h, HAMMER_OF_JUSTICE, npc, caster);
    assert!(has(&h, HAMMER_OF_JUSTICE));
}

#[test]
fn spell_auras_77_unending_resolve_rejects_interrupt_effect() {
    let mut h = harness(Vec::new(), &[104773, FIREBALL, 6552], 9).unwrap();
    let caster = h.caster;
    apply(&mut h, 104773, caster, caster);
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    assert!(h.app.world().get::<CastState>(caster).is_some());
    crate::spell_cast::creature_cast(h.app.world_mut(), h.npc, 6552, Some(caster), true).unwrap();
    assert!(h.app.world().get::<CastState>(caster).is_some());
    super::remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == 104773
    });
    crate::spell_cast::creature_cast(h.app.world_mut(), h.npc, 6552, Some(caster), true).unwrap();
    assert!(h.app.world().get::<CastState>(caster).is_none());
}

fn incoming_fire(h: &mut Harness, periodic: bool) {
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(FIREBALL)
        .unwrap()
        .clone();
    let ctx = EffectCtx {
        caster: h.npc,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    deal_damage(h.app.world_mut(), &ctx, h.caster, 100.0, false, periodic);
    h.advance(1);
}

fn observe_cast_deadline(h: &Harness) -> f64 {
    let cast = h.app.world().get::<CastState>(h.caster).unwrap();
    h.app.world().resource::<Time<Fixed>>().elapsed_secs_f64()
        + f64::from(cast.duration - cast.elapsed)
}

fn pushback_with(spell_id: Option<u32>, expected: f32) {
    let mut ids = vec![FIREBALL];
    ids.extend(spell_id);
    let mut h = harness(Vec::new(), &ids, 8).unwrap();
    let caster = h.caster;
    if let Some(id) = spell_id {
        apply(&mut h, id, caster, caster);
    }
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    assert!(h.inbox.failed.is_empty());
    h.advance(20); // Real progress beyond500ms allows the full reduced delay.
    let before = observe_cast_deadline(&h);
    incoming_fire(&mut h, false);
    let delay = observe_cast_deadline(&h) - before;
    assert!(
        (delay - f64::from(expected)).abs() < 0.0001,
        "actual pushback delay={delay}"
    );
}

#[test]
fn spell_auras_149_damage_delays_unprotected_cast() {
    pushback_with(None, 0.5);
}

#[test]
fn spell_auras_149_barkskin_prevents_damage_pushback() {
    pushback_with(Some(BARKSKIN), 0.0);
}

#[test]
fn spell_auras_149_catalog_70_percent_reduces_damage_pushback() {
    // Spell 1258016 carries REDUCE_PUSHBACK 70 in the local catalog.
    pushback_with(Some(1258016), 0.15);
}

#[test]
fn spell_auras_149_channel_reduction_and_two_hit_limit() {
    let mut h = harness(Vec::new(), &[FIREBALL, 1258016, 5143], 8).unwrap();
    let caster = h.caster;
    apply(&mut h, 1258016, caster, caster);
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(CastState::channel(5143, h.npc.to_bits(), 4.0, 1.0, true));
    incoming_fire(&mut h, false);
    assert!((h.app.world().get::<CastState>(caster).unwrap().duration - 3.7).abs() < 0.0001);
    incoming_fire(&mut h, false);
    let duration = h.app.world().get::<CastState>(caster).unwrap().duration;
    assert!((duration - 3.4225).abs() < 0.0001);
    incoming_fire(&mut h, false);
    assert_eq!(
        h.app.world().get::<CastState>(caster).unwrap().duration,
        duration
    );
}

#[test]
fn spell_auras_149_periodic_damage_does_not_delay_cast() {
    let mut h = harness(Vec::new(), &[FIREBALL], 8).unwrap();
    h.cast_and_deliver(FIREBALL, Some(h.npc));
    let before = h.app.world().get::<CastState>(h.caster).unwrap().duration;
    incoming_fire(&mut h, true);
    assert_eq!(
        h.app.world().get::<CastState>(h.caster).unwrap().duration,
        before
    );
}

#[test]
fn spell_auras_191_pillar_caps_absolute_speed_and_removal_restores_it() {
    let mut h = harness(Vec::new(), &[PILLAR_OF_FROST, HAMSTRING], 6).unwrap();
    let (caster, npc) = (h.caster, h.npc);
    apply(&mut h, PILLAR_OF_FROST, caster, caster);
    let speed = h
        .app
        .world()
        .get::<AuraStatMods>(caster)
        .unwrap()
        .movement_speed;
    assert!((speed * 7.0 - 2.0).abs() < 0.0001);
    let mods = h.app.world().get::<AuraStatMods>(caster).unwrap();
    assert!((mods.flight_speed * 7.0 - 2.0).abs() < 0.0001);
    assert!((mods.swim_speed * 4.722_222 - 2.0).abs() < 0.0001);
    // TrinityCore applies the strongest slow after the absolute speed cap.
    apply(&mut h, HAMSTRING, npc, caster);
    let speed = h
        .app
        .world()
        .get::<AuraStatMods>(caster)
        .unwrap()
        .movement_speed;
    assert!((speed * 7.0 - 1.0).abs() < 0.0001);
    super::remove_auras(h.app.world_mut(), caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == PILLAR_OF_FROST
    });
    assert_eq!(
        h.app
            .world()
            .get::<AuraStatMods>(caster)
            .unwrap()
            .movement_speed,
        0.5
    );
}
