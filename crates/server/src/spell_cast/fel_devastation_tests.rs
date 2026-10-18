//! Native70448 Fel Devastation selectors and scheduled healing.
//! FEL_DEVASTATION_UPFRONT_HEAL_SOURCE_CONFLICT remains deferred, not certified absent.
use super::test_harness::*;
use crate::auras;
use crate::class_progression::ActiveSpec;
use crate::stats::CharacterStats;
use bevy::prelude::*;
use shared::components::{Level, Position, PowerType, Rotation};
use shared::protocol::{CombatLogEvent, CombatLogKind, MissKind};
use shared::spell_data::CastFailReason;

const ROOT: u32 = 212084;
const DAMAGE: u32 = 212105;
const HEAL: u32 = 212106;
const RANK: u32 = 320639;
const SHEAR: u32 = 203782;
const INSTANT_KILL: u32 = 5;
const FIREBALL: u32 = 133;
const FIRE_IMMUNITY: u32 = 7942;
const IDS: &[u32] = &[
    ROOT,
    DAMAGE,
    HEAL,
    RANK,
    SHEAR,
    INSTANT_KILL,
    FIREBALL,
    FIRE_IMMUNITY,
    SUNWELL_RADIANCE,
];

fn fixture(rank: bool) -> Harness {
    // Store-only environmental immunity must not be learned as the caster's passive.
    let known: Vec<_> = IDS
        .iter()
        .copied()
        .filter(|id| *id != FIRE_IMMUNITY && (rank || *id != RANK))
        .collect();
    let mut h = harness_with(Vec::new(), &known, 12, true).unwrap();
    let data = world_db().unwrap();
    let source = rusqlite::Connection::open_with_flags(
        &data.powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_info::load_class_spells(&source).unwrap());
    h.app
        .insert_resource(crate::spell_info::SpellStore::fixture(
            IDS.iter()
                .map(|id| data.store.get(*id).unwrap().clone())
                .collect(),
        ));
    h.app.insert_resource(load_native_creature_world(&[6]));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((Level(80), ActiveSpec(581)));
    h.advance(4);
    assert_eq!(auras::has_aura(h.app.world(), h.caster, RANK), rank);
    assert!(
        h.app
            .world()
            .get::<CharacterStats>(h.caster)
            .unwrap()
            .attack_power
            > 0.0
    );
    h
}

fn at(h: &mut Harness, x: f32, z: f32) -> Entity {
    let unit = spawn_npc(&mut h.app, x);
    h.app
        .world_mut()
        .entity_mut(unit)
        .insert(Position { x, y: 0.0, z });
    unit
}

fn move_unit(h: &mut Harness, unit: Entity, x: f32, z: f32) {
    h.app
        .world_mut()
        .entity_mut(unit)
        .insert(Position { x, y: 0.0, z });
}

fn events<'a>(h: &'a Harness, spell: u32, kind: CombatLogKind) -> Vec<&'a CombatLogEvent> {
    h.inbox
        .log
        .iter()
        .filter(|e| {
            e.source == Some(h.caster.to_bits()) && e.spell_id == Some(spell) && e.kind == kind
        })
        .collect()
}

fn trigger_damage(h: &mut Harness) {
    let caster = h.caster;
    // Same existing aura23 dispatch boundary: explicit aura unit is self.
    super::cast_triggered(h.app.world_mut(), caster, DAMAGE, Some(caster), 1);
    h.advance(3);
}

#[test]
fn vengeance_fel_existing_geometry_native_cone_75deg_13yd_is_uncapped_with_native_exclusions() {
    let mut h = fixture(true);
    let front = h.npc;
    move_unit(&mut h, front, 8.0, 0.0);
    let inside = at(
        &mut h,
        8.0 * 30.0_f32.to_radians().cos(),
        -8.0 * 30.0_f32.to_radians().sin(),
    );
    let outside_angle = at(
        &mut h,
        8.0 * 50.0_f32.to_radians().cos(),
        -8.0 * 50.0_f32.to_radians().sin(),
    );
    let behind = at(&mut h, -8.0, 0.0);
    let far = at(&mut h, 14.0, 0.0);
    let friend = spawn_player(&mut h.app, 12, 8.0);
    let other_copy = at(&mut h, 8.0, 0.0);
    h.app
        .world_mut()
        .entity_mut(other_copy)
        .insert(crate::maps::InMap::continent(1));
    let dead = at(&mut h, 8.0, 0.0);
    let caster = h.caster;
    super::cast_triggered(h.app.world_mut(), caster, INSTANT_KILL, Some(dead), 1);
    h.advance(3);
    assert_eq!(h.health(dead), 0.0);
    let excluded = [outside_angle, behind, far, friend, other_copy];
    let before: Vec<_> = excluded.iter().map(|&unit| h.health(unit)).collect();
    let front_health = h.health(front);
    let inside_health = h.health(inside);
    trigger_damage(&mut h);
    let hits = events(&h, DAMAGE, CombatLogKind::Damage);
    let targets: std::collections::HashSet<_> = hits.iter().map(|e| e.target.unwrap()).collect();
    assert_eq!(
        targets,
        [front.to_bits(), inside.to_bits()].into_iter().collect()
    );
    assert_eq!(hits.len(), 2, "native MaxTargets0 is not max-one");
    let ap = h
        .app
        .world()
        .get::<CharacterStats>(caster)
        .unwrap()
        .attack_power;
    for hit in &hits {
        let critical = if hit.crit { 2.0 } else { 1.0 };
        assert!(varied(ap * 0.411_000_013 * critical).contains(&(hit.amount as f32)));
        assert!(!hit.periodic);
    }
    assert_eq!(
        front_health - h.health(front),
        hits.iter()
            .find(|e| e.target == Some(front.to_bits()))
            .unwrap()
            .amount as f32
    );
    assert_eq!(
        inside_health - h.health(inside),
        hits.iter()
            .find(|e| e.target == Some(inside.to_bits()))
            .unwrap()
            .amount as f32
    );
    for (&unit, health) in excluded.iter().zip(before) {
        assert_eq!(h.health(unit), health);
    }
}

#[test]
fn vengeance_fel_existing_geometry_coincident_self_destination_uses_current_facing() {
    let mut h = fixture(true);
    let old_front = h.npc;
    move_unit(&mut h, old_front, 8.0, 0.0);
    let new_front = at(&mut h, -8.0, 0.0);
    h.app.world_mut().entity_mut(h.caster).insert(Rotation {
        x: 0.0,
        y: std::f32::consts::PI + std::f32::consts::FRAC_PI_2,
        z: 0.0,
    });
    let before = h.health(old_front);
    trigger_damage(&mut h);
    let hits = events(&h, DAMAGE, CombatLogKind::Damage);
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].target, Some(new_front.to_bits()));
    assert_eq!(h.health(old_front), before);
}

#[test]
fn vengeance_fel_existing_geometry_native_heal_is_self_only_not_width_six_area() {
    let mut h = fixture(true);
    let friend = spawn_player(&mut h.app, 12, 2.0);
    let (caster, enemy) = (h.caster, h.npc);
    let before = (h.health(friend), h.health(enemy));
    super::cast_triggered(h.app.world_mut(), caster, HEAL, Some(enemy), 1);
    h.advance(3);
    let heals = events(&h, HEAL, CombatLogKind::Heal);
    assert_eq!(heals.len(), 1);
    assert_eq!(heals[0].target, Some(caster.to_bits()));
    let ap = h
        .app
        .world()
        .get::<CharacterStats>(caster)
        .unwrap()
        .attack_power;
    let critical = if heals[0].crit { 2.0 } else { 1.0 };
    assert!(varied(ap * 5.625 * critical).contains(&(heals[0].amount as f32)));
    assert_eq!(heals[0].overflow, heals[0].amount);
    assert_eq!((h.health(friend), h.health(enemy)), before);
}

fn gain_native_fury(h: &mut Harness) {
    apply_native_enemy_dodge_reduction(h);
    let cooldown = world_db()
        .unwrap()
        .store
        .get(SHEAR)
        .unwrap()
        .cooldowns
        .as_ref()
        .unwrap();
    let wait = cooldown.start_recovery_ms as usize / 50 + 4;
    for _ in 0..5 {
        if h.health(h.npc) <= 0.0 {
            h.npc = spawn_npc(&mut h.app, 3.0);
        }
        let target = h.npc;
        move_unit(h, target, 3.0, 0.0);
        h.cast_and_deliver(SHEAR, Some(target));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        h.advance(wait);
    }
    assert_eq!(h.power(PowerType::Fury), 50.0, "five native10-Fury Shears");
}

fn start_channel(h: &mut Harness) -> usize {
    h.cast_and_deliver(ROOT, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(auras::has_aura(h.app.world(), h.caster, ROOT));
    assert_eq!(
        h.app
            .world()
            .get::<shared::casting::CastState>(h.caster)
            .unwrap()
            .spell_id,
        ROOT
    );
    assert_eq!(h.power(PowerType::Fury), 0.0, "native50-Fury payment");
    // Start the observation after application. Any separately implemented
    // up-front heal is outside these tests, never certified absent.
    h.inbox.log.len()
}

fn finish_channel(h: &mut Harness) {
    let duration = world_db()
        .unwrap()
        .store
        .get(ROOT)
        .unwrap()
        .misc
        .as_ref()
        .unwrap()
        .duration
        .as_ref()
        .unwrap()
        .duration_ms;
    h.advance(duration as usize / 50 + 6);
    assert!(!auras::has_aura(h.app.world(), h.caster, ROOT));
}

fn periodic_heals<'a>(h: &'a Harness, start: usize) -> Vec<&'a CombatLogEvent> {
    h.inbox.log[start..]
        .iter()
        .filter(|e| {
            e.spell_id == Some(HEAL)
                && e.source == Some(h.caster.to_bits())
                && e.kind == CombatLogKind::Heal
        })
        .collect()
}

fn assert_native_heals(h: &Harness, start: usize, count: usize) {
    let heals = periodic_heals(h, start);
    assert_eq!(heals.len(), count, "native scheduled212106 heals");
    let ap = h
        .app
        .world()
        .get::<CharacterStats>(h.caster)
        .unwrap()
        .attack_power;
    for heal in heals {
        let critical = if heal.crit { 2.0 } else { 1.0 };
        assert_eq!(heal.target, Some(h.caster.to_bits()));
        assert!(varied(ap * 5.625 * critical).contains(&(heal.amount as f32)));
        assert!(
            !heal.periodic,
            "real E10 child, not a synthetic periodic heal"
        );
    }
}

fn injure_with_native_fireballs(h: &mut Harness) {
    // Existing100SP Mage fixture, not a legal PvP claim or direct stat/HP edit.
    let mage = spawn_player(&mut h.app, 8, -3.0);
    let caster = h.caster;
    let before = h.health(caster);
    for _ in 0..4 {
        super::cast_triggered(h.app.world_mut(), mage, FIREBALL, Some(caster), 1);
        h.advance(10);
    }
    assert!(h.health(caster) > 0.0 && h.health(caster) < before);
}

#[test]
fn vengeance_fel_native_periodic_heals_precede_damage_and_restore_actual_injury() {
    let mut h = fixture(true);
    gain_native_fury(&mut h);
    let producer = h.npc;
    move_unit(&mut h, producer, 20.0, 0.0);
    let target = at(&mut h, 8.0, 0.0);
    injure_with_native_fireballs(&mut h);
    let before = h.health(h.caster);
    let start = start_channel(&mut h);
    finish_channel(&mut h);
    assert_native_heals(&h, start, 10);
    let restored: i32 = periodic_heals(&h, start)
        .iter()
        .map(|e| e.amount - e.overflow)
        .sum();
    assert!(restored > 0);
    assert_eq!(h.health(h.caster) - before, restored as f32);
    let mut damage_since_heal = 0;
    let mut heals = 0;
    let mut hits = 0;
    for event in &h.inbox.log[start..] {
        if event.source != Some(h.caster.to_bits()) {
            continue;
        }
        if event.spell_id == Some(HEAL) && event.kind == CombatLogKind::Heal {
            heals += 1;
            damage_since_heal = 0;
        }
        if event.spell_id == Some(DAMAGE) && event.kind == CombatLogKind::Damage {
            assert!(heals > 0, "heal before the first damage");
            assert_eq!(
                damage_since_heal, 0,
                "one-target pulse preceded by its own heal"
            );
            assert_eq!(event.target, Some(target.to_bits()));
            damage_since_heal += 1;
            hits += 1;
        }
    }
    assert!(hits > 0, "actual native cone damage retained");
}

#[test]
fn vengeance_fel_native_periodic_heal_count_is_independent_of_zero_one_or_three_enemies() {
    for enemies in [0, 1, 3] {
        let mut h = fixture(true);
        gain_native_fury(&mut h);
        let producer = h.npc;
        move_unit(&mut h, producer, 20.0, 0.0);
        for i in 0..enemies {
            at(&mut h, 8.0, i as f32 - 1.0);
        }
        let start = start_channel(&mut h);
        finish_channel(&mut h);
        assert_native_heals(&h, start, 10);
        let targets: std::collections::HashSet<_> = h.inbox.log[start..]
            .iter()
            .filter(|e| e.spell_id == Some(DAMAGE) && e.kind == CombatLogKind::Damage)
            .map(|e| e.target.unwrap())
            .collect();
        assert_eq!(targets.len(), enemies as usize);
    }
}

#[test]
fn vengeance_fel_native_missing_owner_rank_is_not_supplied_by_another_vengeance_actor() {
    let mut h = fixture(false);
    let foreign = spawn_player(&mut h.app, 12, 1.0);
    h.app.world_mut().entity_mut(foreign).insert((
        ActiveSpec(581),
        crate::class_progression::KnownSpells::fixture(vec![RANK]),
    ));
    h.advance(4);
    assert!(auras::has_aura(h.app.world(), foreign, RANK));
    assert!(!auras::has_aura(h.app.world(), h.caster, RANK));
    gain_native_fury(&mut h);
    let start = start_channel(&mut h);
    finish_channel(&mut h);
    assert_native_heals(&h, start, 0);
    assert!(
        h.inbox.log[start..]
            .iter()
            .any(|e| e.spell_id == Some(DAMAGE) && e.kind == CombatLogKind::Damage)
    );
}

#[test]
fn vengeance_fel_native_cancellation_stops_future_owned_periodic_heals() {
    let mut h = fixture(true);
    gain_native_fury(&mut h);
    let start = start_channel(&mut h);
    h.advance(9);
    super::cancel_cast(h.app.world_mut(), h.caster);
    h.advance(6);
    assert!(!auras::has_aura(h.app.world(), h.caster, ROOT));
    let count = periodic_heals(&h, start).len();
    assert!((1..10).contains(&count));
    h.advance(50);
    assert_eq!(periodic_heals(&h, start).len(), count);
}

#[test]
fn vengeance_fel_native_actual_caster_death_cannot_continue_or_resurrect_with_periodic_heals() {
    let mut h = fixture(true);
    gain_native_fury(&mut h);
    let start = start_channel(&mut h);
    h.advance(9);
    assert!(!periodic_heals(&h, start).is_empty());
    let (killer, caster) = (h.npc, h.caster);
    super::cast_triggered(h.app.world_mut(), killer, INSTANT_KILL, Some(caster), 1);
    h.advance(6);
    assert_eq!(h.health(caster), 0.0);
    let count = periodic_heals(&h, start).len();
    h.advance(50);
    assert_eq!(h.health(caster), 0.0);
    assert_eq!(periodic_heals(&h, start).len(), count);
}

#[test]
fn vengeance_fel_native_fire_immune_enemy_does_not_gate_or_multiply_periodic_self_heals() {
    let mut h = fixture(true);
    gain_native_fury(&mut h);
    let producer = h.npc;
    move_unit(&mut h, producer, 20.0, 0.0);
    let target = at(&mut h, 8.0, 0.0);
    auras::apply_aura(
        h.app.world_mut(),
        &auras::AuraApplication {
            caster: h.caster,
            target,
            spell_id: FIRE_IMMUNITY,
            effect_mask: u32::MAX,
            source: auras::AuraSource::Cast,
        },
    );
    let before = h.health(target);
    let start = start_channel(&mut h);
    finish_channel(&mut h);
    assert_native_heals(&h, start, 10);
    assert_eq!(h.health(target), before);
    let immune = h.inbox.log[start..]
        .iter()
        .filter(|e| {
            e.spell_id == Some(DAMAGE)
                && e.target == Some(target.to_bits())
                && e.kind == CombatLogKind::Miss(MissKind::Immune)
        })
        .count();
    assert_eq!(immune, 10);
}

#[test]
fn vengeance_fel_native_refused_fury_cost_does_not_create_periodic_healing() {
    let mut h = fixture(true);
    assert_eq!(h.power(PowerType::Fury), 0.0);
    h.cast_and_deliver(ROOT, None);
    assert_eq!(h.inbox.failed.len(), 1);
    assert_eq!(h.inbox.failed[0].reason, CastFailReason::NotEnoughResource);
    assert!(!auras::has_aura(h.app.world(), h.caster, ROOT));
    h.advance(50);
    assert!(events(&h, HEAL, CombatLogKind::Heal).is_empty());
}

#[test]
fn vengeance_fel_native_recast_after_actual_category_recovery_has_fresh_periodic_heals_only() {
    let mut h = fixture(true);
    gain_native_fury(&mut h);
    let start = start_channel(&mut h);
    finish_channel(&mut h);
    assert_native_heals(&h, start, 10);
    let cooldown = world_db()
        .unwrap()
        .store
        .get(ROOT)
        .unwrap()
        .cooldowns
        .as_ref()
        .unwrap();
    h.advance(cooldown.recovery_ms.max(cooldown.category_recovery_ms) as usize / 50 + 4);
    assert_native_heals(&h, start, 10);
    gain_native_fury(&mut h);
    let fresh = start_channel(&mut h);
    finish_channel(&mut h);
    assert_native_heals(&h, fresh, 10);
}
