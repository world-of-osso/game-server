//! Authentic Grimoire Imp / archive Firebolt binding, not a Wild Imp substitution.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::Health;

fn imp(h: &mut Harness) -> Entity {
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    // TDB1210 creature_template_spell: (416,0,3110,41031),(416,2,3110,44325).
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch("CREATE TABLE guardian_spells (CreatureID INTEGER, SpellIndex INTEGER, Spell INTEGER, VerifiedBuild INTEGER); INSERT INTO guardian_spells VALUES (416,0,3110,41031),(416,2,3110,44325);").unwrap();
    h.app
        .insert_resource(super::guardian::load_guardian_spells(&conn).unwrap());
    h.select(Some(h.npc));
    h.advance(4);
    super::cast_triggered(h.app.world_mut(), h.caster, 111859, Some(h.npc), 0);
    let world = h.app.world_mut();
    world
        .query_filtered::<Entity, With<super::summon::SummonedCreature>>()
        .single(world)
        .unwrap()
}

#[test]
fn guardians_imp_casts_with_native_time_retargets_and_expires() {
    let mut h = harness(Vec::new(), &[111859, 3110], 9).unwrap();
    let guardian = imp(&mut h);
    h.advance(10);
    assert_eq!(
        h.health(h.npc),
        5000.0,
        "1750ms Firebolt cannot resolve immediately"
    );
    h.advance(35);
    assert!(
        h.health(h.npc) < 5000.0,
        "guardian must cast its archive spell"
    );
    let other = spawn_npc(&mut h.app, 3.0);
    h.select(Some(other));
    h.advance(85);
    assert!(
        h.health(other) < 5000.0,
        "next cast inherits changed selection"
    );
    h.advance(600);
    assert!(h.app.world().get_entity(guardian).is_err());
}

#[test]
fn guardians_imp_does_not_attack_with_no_target_or_after_owner_death() {
    let mut h = harness(Vec::new(), &[111859, 3110], 9).unwrap();
    let guardian = imp(&mut h);
    h.select(None);
    h.advance(60);
    let before = h.health(h.npc);
    h.advance(60);
    assert_eq!(h.health(h.npc), before);
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 0.0;
    h.advance(2);
    assert!(h.app.world().get_entity(guardian).is_err());
}

fn sacrifice() -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &[6940, 108853], 2).unwrap();
    let ally = spawn_player(&mut h.app, 2, 0.0);
    super::cast_triggered(h.app.world_mut(), h.caster, 6940, Some(ally), 0);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::stats::CharacterStats {
            spell_power: 100.0,
            ..default()
        });
    (h, ally)
}

#[test]
fn guardians_sacrifice_splits_spell_damage_and_stops_on_native_health_floor() {
    let (mut h, ally) = sacrifice();
    let before = h.health(ally);
    super::cast_triggered(h.app.world_mut(), h.npc, 108853, Some(ally), 1);
    h.advance(5);
    assert!(varied(197.25 * 0.7).contains(&(before - h.health(ally))));
    assert!(varied(197.25 * 0.3).contains(&(1000.0 - h.health(h.caster))));
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 210.0;
    super::cast_triggered(h.app.world_mut(), h.npc, 108853, Some(ally), 1);
    h.advance(5);
    assert_eq!(h.health(h.caster), 200.0);
    assert!(!crate::auras::has_aura(h.app.world(), ally, 6940));
}

#[test]
fn guardians_sacrifice_splits_direct_damage_and_excludes_foreign_map() {
    let (mut h, ally) = sacrifice();
    crate::combat::deal_direct_damage(h.app.world_mut(), h.npc, ally, 100.0);
    assert_eq!(h.health(ally), 930.0);
    assert_eq!(h.health(h.caster), 970.0);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::maps::InMap::continent(1));
    crate::combat::deal_direct_damage(h.app.world_mut(), h.npc, ally, 100.0);
    assert_eq!(h.health(ally), 830.0);
    assert_eq!(h.health(h.caster), 970.0);
}
