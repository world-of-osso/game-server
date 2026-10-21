//! Concrete Retail matrix spells; all IDs come from world.db.
//! Purging Breath supplies an authentic non-matrix trigger binding: all nine
//! matrix roots lack such bindings; their lifecycle is tested without guessing one.
use bevy::prelude::*;
use shared::components::{Npc, Position, UnitSummonedBy};

use super::test_harness::*;

#[test]
fn summon_at_rune_weapon_is_owned_and_expires() {
    let mut h = harness(Vec::new(), &[49028], 6).unwrap();
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    install_summon_properties(&mut h);
    h.cast_and_deliver(49028, Some(h.npc));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let world = h.app.world_mut();
    let summons: Vec<_> = world
        .query::<(Entity, &Npc, &UnitSummonedBy)>()
        .iter(world)
        .filter(|(_, npc, _)| npc.template_id == 27893)
        .map(|(entity, _, owner)| (entity, owner.0))
        .collect();
    assert_eq!(
        summons.len(),
        1,
        "Dancing Rune Weapon must spawn its DB2 creature"
    );
    let (summon, owner) = summons[0];
    assert_eq!(owner, h.caster.to_bits());
    assert_eq!(world.get::<Position>(summon), world.get::<Position>(h.npc));
    h.advance(150);
    assert!(h.app.world().get_entity(summon).is_ok());
    h.advance(12);
    assert!(h.app.world().get_entity(summon).is_err());
}

#[test]
fn toyfx32_native_wild_count_scatters_secondary_actors_in_primary_radius() {
    let mut h = harness(Vec::new(), &[6122], 8).unwrap();
    place_toy_caster_on_source_area(&mut h, 778027, [-8949.95, -132.493], 9);
    let db = world_db().unwrap();
    let conn = rusqlite::Connection::open_with_flags(
        &db.powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::world_data::load_world_data(&db.powers_db));
    h.app
        .insert_resource(super::summon::load_summon_properties(&conn).unwrap());
    h.cast_and_deliver(6122, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == 6122));
    let world = h.app.world_mut();
    let positions: Vec<_> = world
        .query::<(&Npc, &Position, &super::summon::SummonedCreature)>()
        .iter(world)
        .filter(|(npc, _, summon)| npc.template_id == 15622 && summon.spell_id == 6122)
        .map(|(_, position, _)| *position)
        .collect();
    // Native value15 with variance.5: truncate the 11.25..18.75 roll.
    assert!(
        (11..=18).contains(&positions.len()),
        "{} actors",
        positions.len()
    );
    let first = positions[0];
    let horizontal_distance =
        |point: &Position| Vec2::new(point.x - first.x, point.z - first.z).length();
    assert!(
        positions
            .iter()
            .all(|point| horizontal_distance(point) <= 5.001)
    );
    assert!(
        positions
            .iter()
            .any(|point| horizontal_distance(point) > 0.01),
        "primary5yd radius must scatter subsequent actors"
    );
}

fn install_summon_properties(h: &mut Harness) {
    let conn = rusqlite::Connection::open_in_memory().unwrap();
    conn.execute_batch(include_str!(
        "../../tests/fixtures/summon-properties-12.1.0.69933.sql"
    ))
    .unwrap();
    h.app
        .insert_resource(super::summon::load_summon_properties(&conn).unwrap());
}

#[test]
fn summon_at_barrier_spawns_once_and_expires_without_invented_trigger() {
    let mut h = harness(Vec::new(), &[62618], 5).unwrap();
    let ally = spawn_player(&mut h.app, 5, 3.0);
    h.cast_and_deliver(62618, Some(ally));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let world = h.app.world_mut();
    let triggers: Vec<_> = world
        .query::<(Entity, &super::area_trigger::SpellAreaTrigger)>()
        .iter(world)
        .filter(|(_, trigger)| trigger.spell_id == 62618)
        .map(|(entity, trigger)| (entity, trigger.caster))
        .collect();
    assert_eq!(
        triggers.len(),
        1,
        "one area entity, not one per selected ally"
    );
    assert_eq!(triggers[0].1, h.caster);
    assert_eq!(
        world.get::<Position>(triggers[0].0),
        world.get::<Position>(ally)
    );
    h.advance(190);
    assert!(h.app.world().get_entity(triggers[0].0).is_ok());
    h.advance(12);
    assert!(h.app.world().get_entity(triggers[0].0).is_err());
}

/// Retain the original3yd/20yd geometry on actual terrain now that destination138
/// resolves map ground rather than silently retaining synthetic caster height.
fn place_purging_breath_actors_on_ground(h: &mut Harness) -> (Entity, Position, Position) {
    place_toy_caster_on_source_area(h, 778027, [-8949.95, -132.493], 9);
    let origin = *h.app.world().get::<Position>(h.caster).unwrap();
    let inside = Position {
        x: origin.x + 3.0,
        ..origin
    };
    let outside_position = Position {
        x: origin.x + 20.0,
        ..origin
    };
    h.app.world_mut().entity_mut(h.npc).insert(inside);
    let outside = spawn_npc(&mut h.app, outside_position.x);
    h.app
        .world_mut()
        .entity_mut(outside)
        .insert(outside_position);
    (outside, inside, outside_position)
}

#[test]
fn summon_at_data_bound_purging_breath_hits_inside_not_outside() {
    // Real E179 MiscValue 42145 -> EffectTriggerSpell 1287711, radius 2,
    // duration 6000 ms. This is engine proof, NOT a missing matrix binding.
    let mut h = harness(Vec::new(), &[1287715, 1287711], 8).unwrap();
    let (outside, inside_position, outside_position) =
        place_purging_breath_actors_on_ground(&mut h);
    h.cast_and_deliver(1287715, None);
    h.advance(31); // Its real SpellCastTimes record is 1500 ms.
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app
            .world()
            .get::<shared::components::Health>(h.npc)
            .unwrap()
            .current
            < 5000.0
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Health>(outside)
            .unwrap()
            .current,
        5000.0
    );
    let world = h.app.world_mut();
    let triggers: Vec<_> = world
        .query_filtered::<Entity, With<super::area_trigger::SpellAreaTrigger>>()
        .iter(world)
        .collect();
    assert_eq!(triggers.len(), 1);
    let health_after_enter = h
        .app
        .world()
        .get::<shared::components::Health>(h.npc)
        .unwrap()
        .current;
    h.advance(4);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Health>(h.npc)
            .unwrap()
            .current,
        health_after_enter,
        "on-enter trigger must not reapply every fixed tick"
    );
    h.app.world_mut().entity_mut(h.npc).insert(outside_position);
    h.advance(2);
    h.app.world_mut().entity_mut(h.npc).insert(inside_position);
    h.advance(2);
    assert!(
        h.app
            .world()
            .get::<shared::components::Health>(h.npc)
            .unwrap()
            .current
            < health_after_enter,
        "returning inside re-enters the trigger"
    );
    h.advance(122);
    assert!(h.app.world().get_entity(triggers[0]).is_err());
}

#[test]
fn summon_at_periodic_configuration_ticks_only_inside() {
    // Timing fixture only: IDs, child, radius and duration remain real DB2 data.
    // No matrix spell is assigned a fictional trigger binding.
    let mut spell = world_db().unwrap().store.get(1287715).unwrap().clone();
    spell.effects[0].aura_period_ms = 200;
    let mut h = harness(vec![spell], &[1287711], 8).unwrap();
    let (outside, _, _) = place_purging_breath_actors_on_ground(&mut h);
    h.cast_and_deliver(1287715, None);
    h.advance(36);
    let first = h
        .app
        .world()
        .get::<shared::components::Health>(h.npc)
        .unwrap()
        .current;
    assert!(first < 5000.0);
    h.advance(8);
    assert!(
        h.app
            .world()
            .get::<shared::components::Health>(h.npc)
            .unwrap()
            .current
            < first
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Health>(outside)
            .unwrap()
            .current,
        5000.0
    );
}

#[test]
fn summon_at_available_matrix_area_entities_do_not_invent_actions() {
    // Emerald Blossom355913 now has a named delayed heal, exercised by dummy9.
    // Only unbound generic actors belong here: Wildfire's named primary hit
    // is separate from its still-unbound265163 cone actor.
    for (root, area_spell, class, count) in [
        (84714, 84714, 8, 2),
        (153626, 153626, 8, 2),
        (259495, 265163, 3, 1),
    ] {
        let ids = if root == 259495 {
            vec![root, area_spell, 265157, 269747]
        } else {
            vec![root]
        };
        let mut h = harness(Vec::new(), &ids, class).unwrap();
        let target = h.npc;
        h.cast_and_deliver(root, Some(target));
        h.advance(5);
        assert!(
            h.inbox.failed.is_empty(),
            "spell {root}: {:?}",
            h.inbox.failed
        );
        let world = h.app.world_mut();
        let triggers: Vec<_> = world
            .query::<&super::area_trigger::SpellAreaTrigger>()
            .iter(world)
            .filter(|trigger| trigger.spell_id == area_spell)
            .collect();
        assert_eq!(triggers.len(), count, "spell {root}");
        assert!(triggers.iter().all(|trigger| trigger.trigger_spell == 0));
    }
}

#[test]
fn toyfx_companion_fixture_follows_is_immune_and_replaces_slot() {
    // The published Toy dependency closure now supplies actual Green Balloon53453.
    // Exercise native99104's unmodified effect28 and canonical3141, without substitutions.
    // This mechanism proof does not claim unimplemented root target43/aura4 or native pixels.
    let mut h = harness(Vec::new(), &[99104], 8).unwrap();
    let path = &world_db().unwrap().powers_db;
    h.app
        .insert_resource(crate::world_data::load_world_data(path));
    let conn =
        rusqlite::Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    h.app
        .insert_resource(super::summon::load_summon_properties(&conn).unwrap());
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::CombatStatus(false));
    let spell = world_db().unwrap().store.get(99104).unwrap();
    assert_eq!(spell.effects[0].misc_values, [53453, 3141]);
    assert!(super::summon::unsupported_reason(h.app.world(), &spell.effects[0]).is_none());
    // TC Creature::Create initializes terrain area before SelectLevel. Normal53453
    // has tuning0/delta0; real area7558 supplies tuning332 with levels10–45.
    super::test_harness::place_toy_caster_on_source_map_area(
        &mut h,
        1220,
        955950,
        [4584.0, 6450.666],
        7558,
    );
    let position = *h.app.world().get::<Position>(h.caster).unwrap();
    let position = Vec3::new(position.x, position.y, position.z);
    let (area_tuning, maximum): (u32, u8) = conn
        .query_row(
            "SELECT a.ContentTuningID, c.maxlevel FROM area_table a
         JOIN content_tuning_level c ON c.ID = a.ContentTuningID WHERE a.ID = 7558",
            [],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .unwrap();
    assert_eq!((area_tuning, maximum), (332, 45));
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Level>(h.caster)
            .unwrap()
            .0,
        10
    );
    let ctx = super::effects::EffectCtx {
        caster: h.caster,
        spell,
        depth: 0,
        // Target43 selection is outside this companion lifecycle boundary.
        dest: Some(super::targets::Dest {
            position,
            orientation: 0.0,
        }),
        chain_multiplier: 1.0,
    };
    super::summon::summon_creature(h.app.world_mut(), &ctx, &spell.effects[0]);
    let first = {
        let world = h.app.world_mut();
        world
            .query_filtered::<Entity, With<super::summon::SummonedCreature>>()
            .single(world)
            .unwrap()
    };
    h.advance(40); // The authored companion has no finite lifetime.
    assert!(h.app.world().get_entity(first).is_ok());
    super::summon::summon_creature(h.app.world_mut(), &ctx, &spell.effects[0]);
    assert!(
        h.app.world().get_entity(first).is_err(),
        "second native summon effect replaces the old slot occupant"
    );
    let world = h.app.world_mut();
    let summons: Vec<_> = world
        .query_filtered::<Entity, With<super::summon::SummonedCreature>>()
        .iter(world)
        .collect();
    assert_eq!(
        summons.len(),
        1,
        "companion slot 6 replaces, rather than duplicates"
    );
    let summon = summons[0];
    assert_eq!(
        world
            .get::<shared::components::Npc>(summon)
            .unwrap()
            .template_id,
        53453
    );
    assert_eq!(
        world
            .get::<shared::components::ModelDisplay>(summon)
            .unwrap()
            .display_id,
        38340
    );
    assert_eq!(
        world.get::<shared::components::UnitLevel>(summon),
        Some(&shared::components::UnitLevel(maximum)),
        "UseCreatureLevel must preserve native regional birth instead of caster level"
    );
    assert!(
        world
            .get::<super::summon::SummonedCreature>(summon)
            .unwrap()
            .expires_at
            .is_none()
    );
    assert!(
        world
            .get::<crate::creature_ai::FollowOwner>(summon)
            .is_some()
    );
    assert_eq!(
        world.get::<UnitSummonedBy>(summon).unwrap().0,
        h.caster.to_bits()
    );
    let flags = world
        .get::<shared::components::UnitFlags>(summon)
        .unwrap()
        .0;
    assert_eq!(flags & 0x300, 0x300, "immune to player and NPC attacks");
    world
        .entity_mut(h.caster)
        .insert(crate::maps::InMap::continent(1));
    h.advance(2);
    assert!(h.app.world().get_entity(summon).is_err());
}

#[test]
fn summon_at_summon_and_area_are_removed_when_owner_leaves_map() {
    let mut h = harness(Vec::new(), &[49028, 62618], 6).unwrap();
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    install_summon_properties(&mut h);
    h.cast_and_deliver(49028, Some(h.npc));
    // The area is a granted triggered cast: no DK spellbook/GCD restriction.
    super::cast_triggered(h.app.world_mut(), h.caster, 62618, Some(h.caster), 0);
    let world = h.app.world_mut();
    let summons: Vec<_> = world
        .query_filtered::<Entity, With<super::summon::SummonedCreature>>()
        .iter(world)
        .collect();
    let areas: Vec<_> = world
        .query_filtered::<Entity, With<super::area_trigger::SpellAreaTrigger>>()
        .iter(world)
        .collect();
    assert_eq!(summons.len(), 1);
    assert_eq!(areas.len(), 1);
    world
        .entity_mut(h.caster)
        .insert(crate::maps::InMap::continent(1));
    h.advance(2);
    for entity in summons.into_iter().chain(areas) {
        assert!(h.app.world().get_entity(entity).is_err());
    }
}

/// 12.1.5 authors SummonProperties 3001 with Title -1 (TC `int32 Title`); startup must load it
/// and report it unsupported instead of aborting the whole table.
#[test]
fn installed_summon_properties_load_signed_title_as_unsupported() {
    let conn = rusqlite::Connection::open_with_flags(
        concat!(env!("CARGO_MANIFEST_DIR"), "/../../data/world.db"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let store = super::summon::load_summon_properties(&conn).unwrap();
    let properties = store.0[&3001];
    assert_eq!((properties.control, properties.title), (1, -1));
    assert!(!properties.is_supported());
}
