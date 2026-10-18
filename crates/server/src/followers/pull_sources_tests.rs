//! Authored Stockade/Azure source identity, not proximity clusters or visible targets.
use super::pull_forecast_tests::source_fixture;
use super::{pull_history, pull_sources};
use crate::maps::InMap;
use crate::world_data::WorldData;
use bevy::prelude::*;
use std::collections::BTreeSet;

fn load_map_sources(world: &mut World, map: u16) {
    let native = crate::test_data::world_data();
    let mut data = world.resource_mut::<WorldData>();
    data.spawns = native
        .spawns
        .iter()
        .filter(|spawn| spawn.map == map)
        .cloned()
        .collect();
    data.spawns_by_map = crate::world_data::build_spawns_by_map(&data.spawns);
    data.formations = native.formations.clone();
}

#[test]
fn tank_pull_sources_disappearance_retains_authored_spawn_without_cross_copy_guessing() {
    let (mut h, tank, enemy) = source_fixture(46249);
    load_map_sources(h.app.world_mut(), 34);
    let map = *h.app.world().get::<InMap>(tank).unwrap();
    let origin = *h
        .app
        .world()
        .get::<crate::zones::CreatureOrigin>(enemy)
        .unwrap();
    let sources =
        pull_sources::read_present_sources(h.app.world_mut(), map, 1, &BTreeSet::new()).unwrap();
    let source = &sources[&origin.guid];
    assert_eq!(source.actor, Some(enemy));
    assert_eq!(source.entry, origin.original_entry);
    assert_eq!(source.guid, origin.guid);
    assert_eq!(
        source.pack_guid, origin.guid,
        "ungrouped Stockade spawn is a singleton"
    );
    let authored = crate::test_data::world_data()
        .spawns
        .iter()
        .find(|spawn| spawn.guid == origin.guid)
        .unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(
        authored.position_x,
        authored.position_y,
        authored.position_z,
    );
    assert_eq!(source.position, Vec3::new(x, y, z));
    // The same authored GUID in a different actual copy is never this copy's actor.
    h.app.world_mut().entity_mut(h.npc).insert((
        origin,
        InMap {
            instance_id: map.instance_id + 1,
            ..map
        },
    ));
    h.app.world_mut().despawn(enemy);
    let sources =
        pull_sources::read_present_sources(h.app.world_mut(), map, 1, &BTreeSet::new()).unwrap();
    let source = &sources[&origin.guid];
    assert_eq!(source.actor, None);
    assert_eq!(source.position, Vec3::new(x, y, z));
    assert_eq!(source.pack_guid, origin.guid);
}

#[test]
fn tank_pull_sources_uses_actual_azure_formation_and_population_difficulty() {
    let (mut h, _, _) = source_fixture(46249);
    load_map_sources(h.app.world_mut(), 2515);
    let map = InMap {
        map_id: 2515,
        instance_id: 93,
    };
    let sources =
        pull_sources::read_present_sources(h.app.world_mut(), map, 205, &BTreeSet::new()).unwrap();
    let members: Vec<_> = sources
        .values()
        .filter(|source| source.pack_guid == 29001835)
        .map(|source| source.guid)
        .collect();
    assert_eq!(members, vec![29001817, 29001831, 29001835]);
    assert_eq!(sources[&29001835].entry, 187155);
    let ordinary =
        pull_sources::read_present_sources(h.app.world_mut(), map, 1, &BTreeSet::new()).unwrap();
    assert_eq!(
        sources.keys().collect::<Vec<_>>(),
        ordinary.keys().collect::<Vec<_>>()
    );
    let absent =
        pull_sources::read_present_sources(h.app.world_mut(), map, u32::MAX, &BTreeSet::new())
            .unwrap();
    assert!(
        absent.is_empty(),
        "no source rows at an unsupported population difficulty"
    );
}

#[test]
fn tank_pull_sources_native_observed_death_clears_only_that_source_guid() {
    let (mut h, tank, enemy) = source_fixture(46249);
    load_map_sources(h.app.world_mut(), 34);
    let owner = h.caster;
    let map = *h.app.world().get::<InMap>(tank).unwrap();
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(super::stockade_diagnostic::DiagnosticParty {
            copy: map,
            members: vec![tank],
            player_role: shared::group::GroupRole::Dps,
        });
    pull_history::sample_all(h.app.world_mut());
    let origin = *h
        .app
        .world()
        .get::<crate::zones::CreatureOrigin>(enemy)
        .unwrap();
    let other = crate::test_data::world_data()
        .spawns
        .iter()
        .find(|spawn| {
            spawn.map == 34
                && spawn.template_entry == origin.original_entry
                && spawn.guid != origin.guid
                && spawn.spawns_at(1)
        })
        .unwrap()
        .guid;
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(133)
        .unwrap()
        .clone();
    let ctx = crate::spell_cast::effects::EffectCtx {
        caster: owner,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let lethal = h.health(enemy) * 100.0;
    crate::spell_cast::effects::deal_damage(h.app.world_mut(), &ctx, enemy, lethal, false, false);
    assert_eq!(h.health(enemy), 0.0, "ordinary source actor actually died");
    let cleared = pull_history::snapshot(h.app.world(), owner)
        .unwrap()
        .cleared_spawns;
    assert!(cleared.contains(&origin.guid));
    let sources = pull_sources::read_present_sources(h.app.world_mut(), map, 1, &cleared).unwrap();
    assert!(!sources.contains_key(&origin.guid));
    assert_eq!(sources[&other].entry, origin.original_entry);
}
