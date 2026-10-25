//! Active70448 Agony980/SuddenOnset1260209; SimC76f8118 actions1405–1491.
//! Cap8, start1/3, damage before increment; no shard RNG/legacy Writhe policy.
use shared::components::{CombatRatings, DerivedStats, Level, UnitStats};
use shared::protocol::{CombatLogEvent, CombatLogKind, MissKind};

use super::*;
use crate::class_progression::{ActiveSpec, KnownSpells};
use crate::spell_cast::test_harness::{
    Harness, drain, harness_with, spawn_connection, spawn_player, varied, world_db,
};
use crate::spell_info::SpellStore;
use crate::stats::CharacterStats;

const AGONY: u32 = 980;
const SUDDEN_ONSET: u32 = 1_260_209;
const CORRUPTION: u32 = 172;
const CORRUPTION_DOT: u32 = 146_739;
const SHADOW_IMMUNITY: u32 = 7743;
const SHIELD: u32 = 17;
const BASE_IDS: &[u32] = &[AGONY, CORRUPTION, CORRUPTION_DOT];
const STORE_IDS: &[u32] = &[
    AGONY,
    CORRUPTION,
    CORRUPTION_DOT,
    SUDDEN_ONSET,
    SHADOW_IMMUNITY,
    SHIELD,
];
const PERIOD_FRAMES: usize = 2000 / 50;
const AGONY_COEFFICIENT: f32 = 0.093_129_701_9;

fn known_ids(talent: bool) -> Vec<u32> {
    let mut ids = BASE_IDS.to_vec();
    if talent {
        ids.push(SUDDEN_ONSET);
    }
    ids
}

fn fixture(talent: bool) -> Harness {
    let mut h = harness_with(Vec::new(), &known_ids(talent), 9, true).unwrap();
    let data = world_db().unwrap();
    h.app.insert_resource(SpellStore::fixture(
        STORE_IDS
            .iter()
            .map(|id| data.store.get(*id).unwrap().clone())
            .collect(),
    ));
    let conn = rusqlite::Connection::open_with_flags(
        &data.powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_class_spells(&conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), ActiveSpec(265)));
    h.advance(4);
    assert_eq!(has_aura(h.app.world(), h.caster, SUDDEN_ONSET), talent);
    h
}

fn owned(h: &Harness, target: Entity, caster: Entity, spell: u32) -> AuraInstance {
    h.app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == caster)
        .expect("owned native aura")
        .clone()
}

fn cast_agony(h: &mut Harness) {
    h.cast_and_deliver(AGONY, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(has_aura(h.app.world(), h.npc, AGONY));
}

fn spell_power(h: &Harness, caster: Entity) -> f32 {
    h.app
        .world()
        .get::<CharacterStats>(caster)
        .unwrap()
        .spell_power
}

fn ticks(events: &[CombatLogEvent], caster: Entity, spell: u32) -> Vec<(i32, bool)> {
    events
        .iter()
        .filter(|event| {
            event.spell_id == Some(spell)
                && event.source == Some(caster.to_bits())
                && event.periodic
                && event.kind == CombatLogKind::Damage
        })
        .map(|event| (event.amount, event.crit))
        .collect()
}

fn assert_damage(actual: (i32, bool), base: f32, stacks: u8, modifier: f32) {
    let crit = if actual.1 { 2.0 } else { 1.0 };
    assert_eq!(
        actual.0 as f32,
        (base * f32::from(stacks) * modifier * crit).round()
    );
}

fn apply_control(h: &mut Harness, spell: u32) {
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.npc,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

#[test]
fn affliction_agony_native_baseline_ticks_ramp_one_to_eight_and_expire() {
    let mut h = fixture(false);
    let caster = h.caster;
    let base = spell_power(&h, caster) * AGONY_COEFFICIENT;
    let health = h.health(h.npc);
    cast_agony(&mut h);
    let aura = owned(&h, h.npc, caster, AGONY);
    assert_eq!(aura.stacks, 1);
    assert_eq!(aura.max_stacks, 8);
    assert!(
        ticks(&h.inbox.log, caster, AGONY).is_empty(),
        "no initial tick"
    );
    h.advance(aura.duration_ms as usize / 50 + 6);
    let damage = ticks(&h.inbox.log, caster, AGONY);
    assert_eq!(damage.len(), 9);
    for (actual, stack) in damage.iter().copied().zip([1, 2, 3, 4, 5, 6, 7, 8, 8]) {
        assert_damage(actual, base, stack, 1.0);
    }
    assert_eq!(
        health - h.health(h.npc),
        damage.iter().map(|event| event.0 as f32).sum::<f32>()
    );
    assert!(!has_aura(h.app.world(), h.npc, AGONY));
    h.advance(PERIOD_FRAMES + 4);
    assert_eq!(ticks(&h.inbox.log, caster, AGONY).len(), 9);
}

#[test]
fn affliction_agony_native_sudden_onset_starts_three_and_uses_native_ten_percent_once() {
    let mut h = fixture(true);
    let caster = h.caster;
    let base = spell_power(&h, caster) * AGONY_COEFFICIENT;
    cast_agony(&mut h);
    let aura = owned(&h, h.npc, caster, AGONY);
    assert_eq!(aura.stacks, 3);
    assert_eq!(aura.max_stacks, 8, "no legacy Writhe18 cap");
    h.advance(aura.duration_ms as usize / 50 + 6);
    let damage = ticks(&h.inbox.log, caster, AGONY);
    assert_eq!(damage.len(), 9);
    for (actual, stack) in damage.iter().copied().zip([3, 4, 5, 6, 7, 8, 8, 8, 8]) {
        assert_damage(actual, base, stack, 1.10);
    }
}

#[test]
fn affliction_agony_native_refresh_preserves_low_high_and_capped_stacks_and_tick_phase() {
    let mut h = fixture(false);
    let caster = h.caster;
    cast_agony(&mut h);
    // Native1500ms GCD is ready while the first2000ms tick is still pending.
    h.advance(30);
    let before = owned(&h, h.npc, caster, AGONY);
    assert_eq!(before.stacks, 1);
    cast_agony(&mut h);
    let after = owned(&h, h.npc, caster, AGONY);
    assert_eq!(after.stacks, 1, "recast is not a damage-stack tick");
    assert_eq!(
        after.effects[0].next_tick_at(),
        before.effects[0].next_tick_at()
    );
    h.advance(140);
    let high = owned(&h, h.npc, caster, AGONY);
    assert!(high.stacks > 1 && high.stacks < 8);
    cast_agony(&mut h);
    let after = owned(&h, h.npc, caster, AGONY);
    assert_eq!(after.stacks, high.stacks);
    assert_eq!(
        after.effects[0].next_tick_at(),
        high.effects[0].next_tick_at()
    );
    h.advance(240);
    assert_eq!(owned(&h, h.npc, caster, AGONY).stacks, 8);
    cast_agony(&mut h);
    assert_eq!(owned(&h, h.npc, caster, AGONY).stacks, 8);
}

#[test]
fn affliction_agony_native_learning_sudden_onset_applies_only_a_minimum_refresh_floor() {
    let mut h = fixture(false);
    let caster = h.caster;
    cast_agony(&mut h);
    h.advance(30);
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(KnownSpells::fixture(known_ids(true)));
    h.advance(2);
    assert!(has_aura(h.app.world(), caster, SUDDEN_ONSET));
    let before = owned(&h, h.npc, caster, AGONY);
    assert_eq!(before.stacks, 1, "learning alone is not a hit/refresh");
    cast_agony(&mut h);
    let after = owned(&h, h.npc, caster, AGONY);
    assert_eq!(after.stacks, 3);
    assert_eq!(
        after.effects[0].next_tick_at(),
        before.effects[0].next_tick_at()
    );
    h.advance(160);
    let high = owned(&h, h.npc, caster, AGONY);
    assert!(high.stacks > 3);
    cast_agony(&mut h);
    assert_eq!(owned(&h, h.npc, caster, AGONY).stacks, high.stacks);
}

#[test]
fn affliction_agony_native_casters_keep_independent_stacks_talent_and_damage() {
    let mut h = fixture(false);
    let first = h.caster;
    let second = spawn_player(&mut h.app, 9, 0.0);
    h.app
        .world_mut()
        .entity_mut(second)
        .remove::<CharacterStats>()
        .insert((
            Level(80),
            ActiveSpec(265),
            UnitStats::default(),
            CombatRatings::default(),
            DerivedStats::default(),
            KnownSpells::fixture(known_ids(true)),
        ));
    let peer = spawn_connection(&mut h.app, second);
    h.advance(4);
    cast_agony(&mut h);
    let connection = std::mem::replace(&mut h.connection, peer);
    h.cast(AGONY, Some(h.npc));
    h.connection = connection;
    let mut foreign_events = Vec::new();
    for _ in 0..(PERIOD_FRAMES * 2 + 8) {
        h.advance(1);
        foreign_events.extend(drain::<CombatLogEvent>(&mut h.app, peer));
    }
    assert_eq!(owned(&h, h.npc, first, AGONY).stacks, 3);
    assert_eq!(owned(&h, h.npc, second, AGONY).stacks, 5);
    let own = ticks(&h.inbox.log, first, AGONY);
    let foreign = ticks(&foreign_events, second, AGONY);
    assert_eq!(own.len(), 2);
    assert_eq!(foreign.len(), 2);
    for (actual, stack) in own.into_iter().zip([1, 2]) {
        assert_damage(
            actual,
            spell_power(&h, first) * AGONY_COEFFICIENT,
            stack,
            1.0,
        );
    }
    for (actual, stack) in foreign.into_iter().zip([3, 4]) {
        assert_damage(
            actual,
            spell_power(&h, second) * AGONY_COEFFICIENT,
            stack,
            1.10,
        );
    }
}

#[test]
fn affliction_agony_native_due_flush_reads_each_live_stack_without_replaying_ticks() {
    let mut h = fixture(false);
    let caster = h.caster;
    let base = spell_power(&h, caster) * AGONY_COEFFICIENT;
    cast_agony(&mut h);
    let aura = owned(&h, h.npc, caster, AGONY);
    // Existing native deadline-flush API, not a live-server lag certificate.
    // No metadata clock, fixed timestep or scheduler budget is changed.
    let deadline = aura.expires_at.unwrap();
    super::periodic::run_ticks_until(h.app.world_mut(), h.npc, aura.instance_id, deadline);
    h.advance(6);
    let damage = ticks(&h.inbox.log, caster, AGONY);
    assert_eq!(damage.len(), 9);
    for (actual, stack) in damage.into_iter().zip([1, 2, 3, 4, 5, 6, 7, 8, 8]) {
        assert_damage(actual, base, stack, 1.0);
    }
    assert_eq!(owned(&h, h.npc, caster, AGONY).stacks, 8);
    super::periodic::run_ticks_until(h.app.world_mut(), h.npc, aura.instance_id, deadline);
    h.advance(6);
    assert_eq!(ticks(&h.inbox.log, caster, AGONY).len(), 9);
}

#[test]
fn affliction_agony_native_school_immunity_stops_ramp_but_full_absorb_is_still_a_hit() {
    let mut h = fixture(false);
    let caster = h.caster;
    cast_agony(&mut h);
    let health = h.health(h.npc);
    apply_control(&mut h, SHADOW_IMMUNITY);
    assert!(
        has_aura(h.app.world(), h.npc, AGONY),
        "non-purging native7743"
    );
    h.advance(PERIOD_FRAMES + 4);
    assert_eq!(h.health(h.npc), health);
    assert_eq!(owned(&h, h.npc, caster, AGONY).stacks, 1);
    assert!(h.inbox.log.iter().any(|event| event.spell_id == Some(AGONY)
        && event.kind == CombatLogKind::Miss(MissKind::Immune)
        && event.periodic));
    remove_auras(h.app.world_mut(), h.npc, RemoveReason::Cancelled, |aura| {
        aura.spell_id == SHADOW_IMMUNITY
    });
    h.advance(PERIOD_FRAMES);
    assert_eq!(owned(&h, h.npc, caster, AGONY).stacks, 2);
    assert!(h.health(h.npc) < health);

    let mut shielded = fixture(false);
    let caster = shielded.caster;
    cast_agony(&mut shielded);
    let health = shielded.health(shielded.npc);
    apply_control(&mut shielded, SHIELD);
    shielded.advance(PERIOD_FRAMES * 2 + 4);
    assert_eq!(shielded.health(shielded.npc), health);
    assert_eq!(owned(&shielded, shielded.npc, caster, AGONY).stacks, 3);
    let events: Vec<_> = shielded
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(AGONY) && event.kind == CombatLogKind::Damage && event.periodic
        })
        .collect();
    assert_eq!(events.len(), 2);
    assert!(
        events
            .iter()
            .all(|event| event.amount == 0 && event.absorbed > 0)
    );
}

#[test]
fn affliction_agony_native_cancel_then_fresh_cast_does_not_keep_old_ramp() {
    let mut h = fixture(true);
    let caster = h.caster;
    cast_agony(&mut h);
    h.advance(PERIOD_FRAMES * 2 + 4);
    assert_eq!(owned(&h, h.npc, caster, AGONY).stacks, 5);
    remove_auras(h.app.world_mut(), h.npc, RemoveReason::Cancelled, |aura| {
        aura.spell_id == AGONY
    });
    h.advance(PERIOD_FRAMES * 2 + 4);
    assert_eq!(ticks(&h.inbox.log, caster, AGONY).len(), 2);
    cast_agony(&mut h);
    assert_eq!(owned(&h, h.npc, caster, AGONY).stacks, 3);
    h.advance(PERIOD_FRAMES + 4);
    let damage = ticks(&h.inbox.log, caster, AGONY);
    assert_eq!(damage.len(), 3);
    assert_damage(
        damage[2],
        spell_power(&h, caster) * AGONY_COEFFICIENT,
        3,
        1.10,
    );
}

#[test]
fn affliction_agony_native_sudden_onset_does_not_ramp_or_amplify_corruption() {
    let mut h = fixture(true);
    let caster = h.caster;
    let power = spell_power(&h, caster);
    h.cast_and_deliver(CORRUPTION, Some(h.npc));
    h.advance(PERIOD_FRAMES * 2 + 8);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let aura = owned(&h, h.npc, caster, CORRUPTION_DOT);
    assert_eq!(aura.stacks, 1);
    let damage = ticks(&h.inbox.log, caster, CORRUPTION_DOT);
    assert_eq!(damage.len(), 2);
    for (amount, critical) in damage {
        let crit = if critical { 2.0 } else { 1.0 };
        assert!(varied(power * 0.380_104_989 * crit).contains(&(amount as f32)));
    }
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(CORRUPTION)
                && event.kind == CombatLogKind::Damage
                && !event.periodic)
    );
}
