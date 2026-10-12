//! Native429433 cast counters and their authored next-spell modifiers.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Health, Mana};
use shared::protocol::CombatLogKind;

const BLOOMING_INFUSION: u32 = 429_433;
const OFFENSIVE_INFUSION: u32 = 429_474;
const HEALING_INFUSION: u32 = 429_438;
const REGROWTH: u32 = 8_936;
const WRATH: u32 = 5_176;

fn infusion_harness() -> Harness {
    let mut h = harness(
        Vec::new(),
        &[
            BLOOMING_INFUSION,
            OFFENSIVE_INFUSION,
            HEALING_INFUSION,
            REGROWTH,
            WRATH,
        ],
        11,
    )
    .unwrap();
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 1_000_000.0,
        max: 1_000_000.0,
    });
    h.advance(3);
    h
}

fn native_cast_count() -> usize {
    world_db()
        .unwrap()
        .store
        .get(BLOOMING_INFUSION)
        .unwrap()
        .effects[0]
        .base_points as usize
}

fn cast_regrowth(h: &mut Harness) {
    // Native full pools isolate cast counting; resource policy is separately tested.
    let mut mana = h.app.world_mut().get_mut::<Mana>(h.caster).unwrap();
    mana.current = mana.max;
    h.cast_and_deliver(REGROWTH, Some(h.caster));
    h.advance(45);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
}

#[test]
fn class_completeness_native_blooming_infusion_every_regrowth_threshold_grants_one_buff() {
    let mut h = infusion_harness();
    for _ in 1..native_cast_count() {
        cast_regrowth(&mut h);
        assert!(!crate::auras::has_aura(
            h.app.world(),
            h.caster,
            OFFENSIVE_INFUSION
        ));
    }
    cast_regrowth(&mut h);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        OFFENSIVE_INFUSION
    ));
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        HEALING_INFUSION
    ));
}

#[test]
fn class_completeness_native_blooming_infusion_delayed_wrath_keeps_consumed_damage_bonus() {
    let mut h = infusion_harness();
    for _ in 0..native_cast_count() {
        cast_regrowth(&mut h);
    }
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        OFFENSIVE_INFUSION
    ));
    h.inbox.log.clear();
    h.cast_and_deliver(WRATH, Some(h.npc));
    assert!(
        !crate::auras::has_aura(h.app.world(), h.caster, OFFENSIVE_INFUSION),
        "next-spell buff must be consumed at successful launch"
    );
    h.advance(60);
    let hit = h
        .inbox
        .log
        .iter()
        .find(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(WRATH))
        .unwrap();
    let native = world_db().unwrap().store.get(WRATH).unwrap();
    let effect = &native.effects[0];
    let base = super::effects::scaled_amount(h.app.world(), h.caster, native, effect);
    let percent = world_db()
        .unwrap()
        .store
        .get(OFFENSIVE_INFUSION)
        .unwrap()
        .effects[1]
        .base_points;
    let expected = base * (1.0 + percent / 100.0);
    assert!(
        varied(expected).contains(&(hit.amount as f32)),
        "Wrath amount{} vs native expected{expected}",
        hit.amount
    );
}
