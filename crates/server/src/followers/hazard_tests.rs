//! Actual hostile Consecration and Stockade routes, production fixed-order ticks.
use super::navigation_tests::stockade_fixture;
use super::utility_tests::prepare_enemy_spec;
use crate::maps::InMap;
use crate::spell_cast::test_harness::spawn_npc;
use bevy::prelude::*;
use shared::components::Position;

#[test]
fn follower_hazard_probe_waits_for_fixed_publication_before_starting_pursuit() {
    let start = Position { x: 103.0, ..FROM };
    let (mut h, unit, _) = stockade_fixture(start);
    hostile_field_at(
        &mut h,
        unit,
        Position {
            z: FROM.z + 25.0,
            ..FROM
        },
    );
    let target = spawn_npc(&mut h.app, 0.0);
    h.app
        .world_mut()
        .entity_mut(target)
        .insert((Position { x: 108.0, ..FROM }, InMap::continent(34)));
    h.app
        .world_mut()
        .entity_mut(unit)
        .remove::<crate::creature_ai::FollowOwner>()
        .insert(crate::creature_ai::CreatureState::Idle);
    super::ai::pursue(h.app.world_mut(), unit, target, 0, true);
    assert!(
        matches!(
            h.app.world().get::<crate::creature_ai::CreatureState>(unit),
            Some(crate::creature_ai::CreatureState::Idle)
        ),
        "an unpublished hazard probe must not become a clear approach"
    );
    crate::navigation::drain_fixture(&mut h.app);
    super::ai::pursue(h.app.world_mut(), unit, target, 0, true);
    assert!(
        matches!(
            h.app.world().get::<crate::creature_ai::CreatureState>(unit),
            Some(crate::creature_ai::CreatureState::Pursuing { .. })
        ),
        "completed clear route must retain ordinary pursuit policy"
    );
}

#[test]
fn follower_hostile_field_without_a_bake_never_becomes_direct_escape() {
    let (mut h, unit, _) = stockade_fixture(FROM);
    hostile_field(&mut h, unit);
    let missing = tempfile::tempdir().unwrap();
    h.app
        .insert_resource(crate::navigation::Navigation::new(missing.path()));
    for _ in 0..60 {
        h.advance(1);
    }
    assert_eq!(*h.app.world().get::<Position>(unit).unwrap(), FROM);
}

#[test]
fn follower_does_not_flee_friendly_consecration() {
    let (mut h, unit, _) = stockade_fixture(FROM);
    h.app.world_mut().entity_mut(h.caster).insert(FROM);
    crate::spell_cast::creature_cast(h.app.world_mut(), unit, 26573, Some(unit), false).unwrap();
    for _ in 0..60 {
        h.advance(1);
    }
    assert_eq!(*h.app.world().get::<Position>(unit).unwrap(), FROM);
}

#[test]
fn follower_does_not_follow_an_outside_owner_through_hostile_consecration() {
    let start = Position { x: 103.0, ..FROM };
    let (mut h, unit, _) = stockade_fixture(start);
    let source = hostile_field(&mut h, unit);
    h.app
        .world_mut()
        .entity_mut(source)
        .insert(crate::networking::CombatTarget(None));
    h.app.world_mut().entity_mut(h.caster).insert((
        Position { x: 125.0, ..FROM },
        crate::networking::CombatTarget(None),
    ));
    for tick in 0..100 {
        h.advance(1);
        let at = *h.app.world().get::<Position>(unit).unwrap();
        assert!(
            point(at).distance(point(FROM)) > 8.0,
            "follow tick{tick} entered the hostile field at{at:?}"
        );
    }
}

#[test]
fn follower_does_not_chase_an_outside_enemy_through_hostile_consecration() {
    let start = Position { x: 103.0, ..FROM };
    let (mut h, unit, _) = stockade_fixture(start);
    let source = hostile_field(&mut h, unit);
    h.app
        .world_mut()
        .entity_mut(source)
        .insert(crate::networking::CombatTarget(None));
    let target = spawn_npc(&mut h.app, 0.0);
    h.app
        .world_mut()
        .entity_mut(target)
        .insert((Position { x: 125.0, ..FROM }, InMap::continent(34)));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::networking::CombatTarget(Some(target)));
    for tick in 0..100 {
        h.advance(1);
        let at = *h.app.world().get::<Position>(unit).unwrap();
        assert!(
            point(at).distance(point(FROM)) > 8.0,
            "chase tick{tick} entered the hostile field at{at:?}"
        );
    }
}

fn point(position: Position) -> Vec3 {
    Vec3::new(position.x, position.y, position.z)
}

pub(super) const FROM: Position = Position {
    x: 116.0,
    y: -34.85623,
    z: -76.0,
};

fn hostile_field(h: &mut crate::spell_cast::test_harness::Harness, unit: Entity) -> Entity {
    hostile_field_at(h, unit, FROM)
}

pub(super) fn hostile_field_at(
    h: &mut crate::spell_cast::test_harness::Harness,
    unit: Entity,
    from: Position,
) -> Entity {
    let enemy = spawn_npc(&mut h.app, 0.0);
    h.app
        .world_mut()
        .entity_mut(enemy)
        .insert((from, InMap::continent(34)));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert((from, crate::networking::CombatTarget(Some(enemy))));
    h.app
        .world_mut()
        .entity_mut(enemy)
        .insert(crate::networking::CombatTarget(Some(unit)));
    prepare_enemy_spec(h, enemy, 66);
    crate::spell_cast::creature_cast(h.app.world_mut(), enemy, 26573, Some(enemy), false).unwrap();
    enemy
}

#[test]
fn follower_leaves_hostile_consecration_on_actual_stockade_navigation() {
    let from = FROM;
    let (mut h, unit, engine) = stockade_fixture(from);
    hostile_field(&mut h, unit);
    let mut los = los::LosMap::new(&engine.join("los/34"));
    let mut previous = from;
    let mut escaped = false;
    for tick in 0..200 {
        h.advance(1);
        let at = *h.app.world().get::<Position>(unit).unwrap();
        assert!(
            los.clear(point(previous) + Vec3::Y, point(at) + Vec3::Y)
                .unwrap(),
            "flee tick{tick} crossed real collision"
        );
        if point(at).distance(point(from)) > 8.0 {
            escaped = true;
            break;
        }
        previous = at;
    }
    assert!(
        escaped,
        "companion must escape actual hostile Consecration before expiry"
    );
    for _ in 0..40 {
        h.advance(1);
        let at = *h.app.world().get::<Position>(unit).unwrap();
        assert!(
            point(at).distance(point(from)) > 8.0,
            "combat must not pull the companion back into the active field"
        );
    }
}

const NATIVE_SPROUT_AURA: u32 = 374161;
const NATIVE_SPROUT_DAMAGE: u32 = 374523;

fn load_native_field_fixture(h: &mut crate::spell_cast::test_harness::Harness, unit: Entity) {
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::spell_cast::area_trigger::load_aura_field_catalog(&db).unwrap());
    h.app
        .insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&db).unwrap());
    let entry = h
        .app
        .world()
        .get::<shared::components::Npc>(unit)
        .unwrap()
        .template_id;
    let (display_id, scale) = load_single_follower_model(&db, entry);
    // The common class-profile fixture does not render models. Supply the actual
    // source body fields used by zones::choose_display, never guessed geometry.
    h.app.world_mut().entity_mut(unit).insert((
        shared::components::ModelDisplay { display_id },
        shared::components::UnitScale(scale),
        crate::zones::NativeObjectScale(scale),
    ));
}

fn load_single_follower_model(db: &rusqlite::Connection, entry: u32) -> (u32, f32) {
    let mut statement = db.prepare(
        "SELECT CreatureDisplayID,DisplayScale FROM content_creature_template_model WHERE CreatureID=?1 ORDER BY Idx",
    ).unwrap();
    let models = statement
        .query_map([entry], |row| {
            Ok((row.get::<_, u32>(0)?, row.get::<_, f32>(1)?))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap();
    assert_eq!(
        models.len(),
        1,
        "single authored follower model prerequisite"
    );
    models[0]
}

fn cast_native_hostile_field(h: &mut crate::spell_cast::test_harness::Harness) -> (Entity, Entity) {
    use crate::spell_cast::area_trigger::SpellAreaTrigger;
    let caster = spawn_npc(&mut h.app, 0.0);
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert((FROM, InMap::continent(34)));
    crate::spell_cast::cast_triggered(
        h.app.world_mut(),
        caster,
        NATIVE_SPROUT_AURA,
        Some(caster),
        1,
    );
    let fields: Vec<_> = h
        .app
        .world_mut()
        .query::<(Entity, &SpellAreaTrigger)>()
        .iter(h.app.world())
        .filter(|(_, field)| field.spell_id == NATIVE_SPROUT_AURA && field.caster == caster)
        .map(|(entity, _)| entity)
        .collect();
    assert_eq!(
        fields.len(),
        1,
        "native aura395 creates the real field; not a hand-built hazard"
    );
    (fields[0], caster)
}

#[test]
fn follower_native_aura395_escape_leaves_the_actual_source_collision_envelope() {
    use crate::spell_cast::area_trigger::SpellAreaTrigger;
    let start = Position {
        x: FROM.x + 2.0,
        ..FROM
    };
    let (mut h, unit, engine) = stockade_fixture(start);
    load_native_field_fixture(&mut h, unit);
    let (field, caster) = cast_native_hostile_field(&mut h);
    let reach = crate::scripted_ai::model_reach::combat_reach(h.app.world(), unit)
        .expect("authored follower combat reach, not a body-radius default");
    let radius = h.app.world().get::<SpellAreaTrigger>(field).unwrap().radius;
    let distance = point(start).distance(point(FROM));
    assert!(
        distance > radius && distance < radius + reach,
        "actual native fringe prerequisite"
    );
    assert!(crate::spell_cast::targets::relation_matches(
        h.app.world(),
        caster,
        unit,
        crate::spell_cast::targets::Relation::Enemy,
    ));
    let mut collision = los::LosMap::new(&engine.join("los/34"));
    let mut previous = start;
    for tick in 0..200 {
        h.advance(1);
        let at = *h.app.world().get::<Position>(unit).unwrap();
        assert!(
            collision
                .clear(point(previous) + Vec3::Y, point(at) + Vec3::Y)
                .unwrap(),
            "native escape tick{tick} crossed actual collision"
        );
        previous = at;
    }
    assert!(
        h.app
            .world()
            .get::<shared::components::Health>(unit)
            .unwrap()
            .current
            > 0.0
    );
    let radius = h.app.world().get::<SpellAreaTrigger>(field).unwrap().radius;
    let distance = point(previous).distance(point(FROM));
    assert!(
        distance > radius + reach,
        "escape stopped inside native affect range: distance{distance}, radius{radius}, source reach{reach}"
    );
    let own_damage = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(unit)
        .unwrap()
        .iter()
        .any(|aura| aura.spell_id == NATIVE_SPROUT_DAMAGE && aura.caster == caster);
    assert!(
        !own_damage,
        "escaped follower no longer carries the field caster's aura"
    );
    for _ in 0..40 {
        h.advance(1);
        let at = *h.app.world().get::<Position>(unit).unwrap();
        let radius = h.app.world().get::<SpellAreaTrigger>(field).unwrap().radius;
        assert!(
            point(at).distance(point(FROM)) > radius + reach,
            "ordinary follow/combat must not re-enter the native collision envelope"
        );
    }
}
