//! Native429433 cast counters and their authored next-spell modifiers.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::{Health, Mana, Position};
use shared::protocol::CombatLogKind;

const BLOOMING_INFUSION: u32 = 429_433;
const OFFENSIVE_INFUSION: u32 = 429_474;
const HEALING_INFUSION: u32 = 429_438;
const REGROWTH: u32 = 8_936;
const WRATH: u32 = 5_176;
const STARSURGE: u32 = 78_674;
const STARFALL: u32 = 191_034;
const BALANCE_PASSIVE: u32 = 137_013;

const FIXTURE_IDS: &[u32] = &[
    BLOOMING_INFUSION,
    OFFENSIVE_INFUSION,
    HEALING_INFUSION,
    REGROWTH,
    WRATH,
    STARSURGE,
    STARFALL,
];

fn infusion_harness() -> Harness {
    let mut h = harness(Vec::new(), FIXTURE_IDS, 11).unwrap();
    h.app.world_mut().entity_mut(h.npc).insert((
        Health {
            current: 1_000_000.0,
            max: 1_000_000.0,
        },
        Position {
            x: 20.0,
            y: 0.0,
            z: 0.0,
        },
    ));
    h.advance(3);
    h
}

fn wrath_base(h: &Harness) -> f32 {
    let spell = world_db().unwrap().store.get(WRATH).unwrap();
    super::effects::scaled_amount(h.app.world(), h.caster, spell, &spell.effects[0])
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

fn fill_pools(h: &mut Harness) {
    // Native full pools isolate cast counting; resource policy is separately tested.
    let mut mana = h.app.world_mut().get_mut::<Mana>(h.caster).unwrap();
    mana.current = mana.max;
    if let Some(mut pools) = h
        .app
        .world_mut()
        .get_mut::<super::powers::PowerPools>(h.caster)
    {
        for pool in &mut pools.pools {
            pool.current = pool.max;
        }
    }
}

fn cast_regrowth(h: &mut Harness) {
    fill_pools(h);
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
        !h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage),
        "native missile has not landed at consumption"
    );
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
    let base = wrath_base(&h);
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
    h.inbox.log.clear();
    h.cast_and_deliver(WRATH, Some(h.npc));
    h.advance(100);
    let ordinary = h
        .inbox
        .log
        .iter()
        .find(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(WRATH))
        .unwrap();
    assert!(
        varied(base).contains(&(ordinary.amount as f32)),
        "following Wrath has no consumed bonus: {}",
        ordinary.amount
    );
}

#[test]
fn class_completeness_native_blooming_new_buff_does_not_change_already_launched_wrath() {
    let mut h = infusion_harness();
    h.inbox.go.clear();
    h.inbox.log.clear();
    h.cast(WRATH, Some(h.npc));
    for _ in 0..100 {
        h.advance(1);
        if h.inbox.go.iter().any(|go| go.spell_id == WRATH) {
            break;
        }
    }
    assert!(
        h.inbox.go.iter().any(|go| go.spell_id == WRATH),
        "real successful launch"
    );
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage),
        "native missile remains in flight"
    );
    let caster = h.caster;
    super::cast_triggered(
        h.app.world_mut(),
        caster,
        OFFENSIVE_INFUSION,
        Some(caster),
        1,
    );
    h.advance(60);
    let hit = h
        .inbox
        .log
        .iter()
        .find(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(WRATH))
        .unwrap();
    assert!(
        varied(wrath_base(&h)).contains(&(hit.amount as f32)),
        "late buff must not change old damage: {}",
        hit.amount
    );
    assert!(
        crate::auras::has_aura(h.app.world(), caster, OFFENSIVE_INFUSION),
        "in-flight cast cannot consume the new use"
    );
}

#[test]
fn class_completeness_native_blooming_triggered_regrowth_and_cancelled_cast_do_not_count() {
    let mut h = infusion_harness();
    let caster = h.caster;
    for _ in 0..native_cast_count() {
        super::cast_triggered(h.app.world_mut(), caster, REGROWTH, Some(caster), 1);
        h.advance(45);
    }
    assert!(!crate::auras::has_aura(
        h.app.world(),
        caster,
        OFFENSIVE_INFUSION
    ));
    for _ in 1..native_cast_count() {
        cast_regrowth(&mut h);
    }
    fill_pools(&mut h);
    h.inbox.go.clear();
    h.cast_and_deliver(REGROWTH, Some(caster));
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_some()
    );
    super::cancel_cast(h.app.world_mut(), caster);
    h.advance(45);
    assert!(
        !h.inbox.go.iter().any(|go| go.spell_id == REGROWTH),
        "cancelled cast never completes"
    );
    assert!(!crate::auras::has_aura(
        h.app.world(),
        caster,
        OFFENSIVE_INFUSION
    ));
    cast_regrowth(&mut h);
    assert!(crate::auras::has_aura(
        h.app.world(),
        caster,
        OFFENSIVE_INFUSION
    ));
}

#[test]
fn class_completeness_native_blooming_starsurges_grant_one_instant_regrowth_use() {
    let mut h = infusion_harness();
    for i in 1..=native_cast_count() {
        fill_pools(&mut h);
        h.cast_and_deliver(STARSURGE, Some(h.npc));
        h.advance(45);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert_eq!(
            crate::auras::has_aura(h.app.world(), h.caster, HEALING_INFUSION),
            i == native_cast_count()
        );
    }
    h.inbox.go.clear();
    fill_pools(&mut h);
    h.cast_and_deliver(REGROWTH, Some(h.caster));
    assert!(
        h.inbox.go.iter().any(|go| go.spell_id == REGROWTH),
        "next Regrowth completes instantly"
    );
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        HEALING_INFUSION
    ));
    h.advance(45);
    h.inbox.go.clear();
    fill_pools(&mut h);
    h.cast_and_deliver(REGROWTH, Some(h.caster));
    assert!(
        !h.inbox.go.iter().any(|go| go.spell_id == REGROWTH),
        "following Regrowth retains its authored cast time"
    );
}

#[test]
fn class_completeness_native_blooming_unlearn_and_foreign_aura_reset_cast_counts() {
    let mut h = infusion_harness();
    for _ in 1..native_cast_count() {
        cast_regrowth(&mut h);
    }
    let ids: Vec<_> = FIXTURE_IDS
        .iter()
        .copied()
        .filter(|&id| id != BLOOMING_INFUSION)
        .collect();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(ids));
    h.advance(3);
    let foreign = spawn_player(&mut h.app, 11, 0.0);
    let owner = h.caster;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: foreign,
            target: owner,
            spell_id: BLOOMING_INFUSION,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    for _ in 0..native_cast_count() {
        cast_regrowth(&mut h);
    }
    assert!(!crate::auras::has_aura(
        h.app.world(),
        owner,
        OFFENSIVE_INFUSION
    ));
    crate::auras::remove_auras(
        h.app.world_mut(),
        owner,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == BLOOMING_INFUSION,
    );
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(crate::class_progression::KnownSpells::fixture(
            FIXTURE_IDS.to_vec(),
        ));
    h.advance(3);
    for _ in 1..native_cast_count() {
        cast_regrowth(&mut h);
    }
    assert!(!crate::auras::has_aura(
        h.app.world(),
        owner,
        OFFENSIVE_INFUSION
    ));
    cast_regrowth(&mut h);
    assert!(crate::auras::has_aura(
        h.app.world(),
        owner,
        OFFENSIVE_INFUSION
    ));
}

#[test]
fn class_completeness_native_blooming_starfall_counts_only_with_balance_passive() {
    let mut h = infusion_harness();
    for _ in 0..native_cast_count() {
        fill_pools(&mut h);
        h.cast_and_deliver(STARFALL, Some(h.caster));
        h.advance(45);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    }
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        HEALING_INFUSION
    ));
    let ids: Vec<_> = FIXTURE_IDS
        .iter()
        .copied()
        .chain([BALANCE_PASSIVE])
        .collect();
    h.app
        .insert_resource(crate::spell_info::SpellStore::fixture(
            ids.iter()
                .map(|&id| world_db().unwrap().store.get(id).unwrap().clone())
                .collect(),
        ));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(ids));
    h.advance(3);
    for _ in 1..native_cast_count() {
        fill_pools(&mut h);
        h.cast_and_deliver(STARSURGE, Some(h.npc));
        h.advance(45);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    }
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        HEALING_INFUSION
    ));
    fill_pools(&mut h);
    h.cast_and_deliver(STARFALL, Some(h.caster));
    h.advance(45);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        HEALING_INFUSION
    ));
}

#[test]
fn class_completeness_native_blooming_missing_buff_rejects_before_payment_or_counter_change() {
    let mut h = infusion_harness();
    for _ in 1..native_cast_count() {
        cast_regrowth(&mut h);
    }
    let store = crate::spell_info::SpellStore::fixture(
        FIXTURE_IDS
            .iter()
            .filter(|&&id| id != OFFENSIVE_INFUSION)
            .map(|&id| world_db().unwrap().store.get(id).unwrap().clone())
            .collect(),
    );
    h.app.insert_resource(store);
    fill_pools(&mut h);
    let before = h.app.world().get::<Mana>(h.caster).unwrap().current;
    h.cast_and_deliver(REGROWTH, Some(h.caster));
    h.advance(45);
    assert_eq!(h.app.world().get::<Mana>(h.caster).unwrap().current, before);
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(
        h.inbox.failed[0].detail.as_deref(),
        Some("Blooming Infusion missing native child 429474")
    );
    h.inbox.failed.clear();
    let store = crate::spell_info::SpellStore::fixture(
        FIXTURE_IDS
            .iter()
            .map(|&id| world_db().unwrap().store.get(id).unwrap().clone())
            .collect(),
    );
    h.app.insert_resource(store);
    cast_regrowth(&mut h);
    assert!(
        crate::auras::has_aura(h.app.world(), h.caster, OFFENSIVE_INFUSION),
        "rejected attempt did not reset or advance the threshold"
    );
}
