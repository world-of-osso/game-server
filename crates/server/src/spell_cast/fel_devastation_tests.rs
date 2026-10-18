//! Native70448 Fel Devastation selectors and scheduled healing.
//! FEL_DEVASTATION_UPFRONT_HEAL_SOURCE_CONFLICT remains deferred, not certified absent.
use super::test_harness::*;
use crate::auras;
use crate::class_progression::ActiveSpec;
use crate::stats::CharacterStats;
use bevy::prelude::*;
use shared::components::{Level, Position, Rotation};
use shared::protocol::{CombatLogEvent, CombatLogKind};

const ROOT: u32 = 212084;
const DAMAGE: u32 = 212105;
const HEAL: u32 = 212106;
const RANK: u32 = 320639;
const SHEAR: u32 = 203782;
const INSTANT_KILL: u32 = 5;
const IDS: &[u32] = &[
    ROOT,
    DAMAGE,
    HEAL,
    RANK,
    SHEAR,
    INSTANT_KILL,
    SUNWELL_RADIANCE,
];

fn fixture(rank: bool) -> Harness {
    let known: Vec<_> = IDS
        .iter()
        .copied()
        .filter(|id| rank || *id != RANK)
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
