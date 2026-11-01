//! Real Ara-Kara ground, source placements and a class party at the sessile Ixin.
use crate::maps::InMap;
use crate::networking::CombatTarget;
use bevy::prelude::*;
use shared::components::{Health, Level, Position};
use std::collections::HashMap;
use std::path::PathBuf;

const IXIN: u32 = 217531;
const TDB_GUID_OFFSET: u32 = 20_000_000;
/// Creatures the party fought before Ixin, from the live combat log: entity ids were
/// assigned in TDB GUID order at copy creation (entity 4294738999 is GUID 10003734).
const KILLED_BEFORE_IXIN: &[u32] = &[
    10003750, 10003781, 10003788, 10003789, 10003790, 10003792, 10003795, 10003797, 10003798,
    10003799, 10003800, 10003804, 10003805, 10003809, 10003810, 10003811, 10003815, 10003816,
    10003820, 10003847, 10003849, 10003851, 10003852, 10003855, 10003857, 10003858, 10003859,
    10003863, 10003864, 10003867, 10003874, 10003875, 10003879, 10003881, 10003886,
];
const LIVE_PARTY_REACH: f32 = 100.0;

fn at(point: Vec3) -> Position {
    Position {
        x: point.x,
        y: point.y,
        z: point.z,
    }
}

#[test]
#[ignore = "requires actual Ara-Kara navigation, ground and line-of-sight assets"]
fn ara_kara_party_fights_sessile_ixin_inside_idle_pack_reach() {
    let (mut h, placeholder, _) =
        super::profile_behavior_tests::spawn_profile_fixture_with_plugins(209057, |app| {
            app.insert_resource(crate::test_data::world_data().clone());
            app.add_plugins((
                crate::stats::StatsPlugin,
                crate::creature_ai::CreatureAiPlugin,
                crate::creature_aggro::CreatureAggroPlugin,
            ));
        });
    h.app.world_mut().despawn(placeholder);
    h.app.world_mut().despawn(h.npc);
    let owner = h.caster;
    h.app
        .world_mut()
        .get_mut::<crate::networking::CombatTarget>(owner)
        .unwrap()
        .0 = None;
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::scripted_ai::model_reach::load_model_reaches(&db).unwrap());
    let maps = crate::maps::store::load_map_store(&db).unwrap();
    let mut instances = crate::maps::InstanceManager::default();
    let map = instances
        .create_map(
            &maps,
            2660,
            crate::maps::instances::InstanceEntrant {
                character_id: 1,
                leader: 1,
                difficulty_id: 205,
                ..default()
            },
            crate::maps::instances::Now {
                elapsed: 0.0,
                unix: 1_793_540_000,
            },
        )
        .unwrap()
        .map;
    h.app.insert_resource(instances);
    // Live 2026-11-01 14:11-14:35 UTC: Ixin (TDB GUID 10003734) was the party's only
    // engaged enemy. The map copy at that moment: every source spawn within 125 yd of the
    // tank except KILLED_BEFORE_IXIN.
    let ixin_at = Vec3::new(-235.422, 148.564, -514.12);
    let tank_at = Vec3::new(-241.75, 140.37, -492.37);
    let source = crate::test_data::world_data();
    let mut data = source.clone();
    let spawns: Vec<_> = source
        .spawns
        .iter()
        .filter(|spawn| {
            let point = Vec3::new(spawn.position_x, spawn.position_z, -spawn.position_y);
            spawn.map == 2660
                && point.distance(tank_at) <= 125.0
                && !KILLED_BEFORE_IXIN.contains(&(spawn.guid - TDB_GUID_OFFSET))
        })
        .cloned()
        .collect();
    let entries: Vec<_> = spawns.iter().map(|spawn| spawn.template_entry).collect();
    data.spawns = spawns.into();
    data.spawns_by_map = crate::world_data::build_spawns_by_map(&data.spawns);
    let native = crate::spell_cast::test_harness::load_native_creature_world(&entries);
    data.templates.extend(native.templates);
    data.difficulty_templates
        .extend(native.difficulty_templates);
    h.app.insert_resource(data.clone());
    let ground_dir = crate::ground::ground_data_dir();
    let mut ground = crate::ground::GroundMap::new(
        &ground_dir,
        HashMap::from([(2660, maps.map(2660).unwrap().wdt_file_data_id)]),
    );
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let mut commands = Commands::new(&mut queue, h.app.world());
    crate::zones::spawn_map_creatures(
        &mut commands,
        &data,
        crate::zones::MapContent {
            map,
            difficulty_id: 205,
            dead_bosses: vec![],
        },
        false,
        Some(&mut ground),
    );
    queue.apply(h.app.world_mut());
    h.app.insert_resource(ground);
    h.app
        .insert_resource(crate::line_of_sight::LineOfSight::load(&ground_dir.join("los")).unwrap());
    let nav = std::env::var_os("GAME_SERVER_NAV_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../data/ground/navmesh")
        });
    crate::navigation::install_fixture(&mut h.app, crate::navigation::Navigation::new(&nav));
    let spawned: Vec<_> = h
        .app
        .world_mut()
        .query::<(Entity, &shared::components::Npc, &InMap)>()
        .iter(h.app.world())
        .filter(|(_, _, copy)| **copy == map)
        .map(|(unit, npc, _)| (unit, npc.template_id))
        .collect();
    let ixin = spawned
        .iter()
        .find(|(_, entry)| *entry == IXIN)
        .map(|(unit, _)| *unit)
        .expect("source Ixin");
    let neighbours: Vec<_> = spawned
        .iter()
        .filter(|(unit, _)| *unit != ixin)
        .map(|(unit, _)| *unit)
        .collect();

    // Live roster samples at 08:13 UTC: Arcane Mage owner and the drawn LFG party.
    h.app.world_mut().entity_mut(owner).insert((
        at(Vec3::new(-238.0, 141.2, -498.2)),
        map,
        Level(80),
        // Arcane Blast, Arcane Barrage and Arcane Explosion: the live owner's spread reach.
        crate::class_progression::KnownSpells::fixture(vec![30451, 44425, 1449]),
    ));
    // Class rotations live in the APL crate; this fixture proves the positioning and
    // hold state that gates them (`holding_pull_damage`).
    h.app.insert_resource(super::FollowerActionProvider {
        source_actions: |_, _, _| Ok(Vec::new()),
        equipped_actions: |_, _, _| Ok(Vec::new()),
    });
    let mut members = Vec::new();
    for (spec, point) in [
        (104, tank_at),
        (105, Vec3::new(-245.5, 140.4, -491.0)),
        (72, Vec3::new(-244.06, 140.19, -489.33)),
        (577, Vec3::new(-241.9, 140.04, -488.79)),
    ] {
        let unit = super::native_fixture::spawn_class(h.app.world_mut(), owner, spec).unwrap();
        h.app.world_mut().entity_mut(unit).insert((at(point), map));
        members.push(unit);
    }
    let (tank, dps) = (members[0], [members[2], members[3]]);
    h.app
        .world_mut()
        .entity_mut(owner)
        .insert(super::stockade_diagnostic::DiagnosticParty {
            copy: map,
            members: members.clone(),
            player_role: shared::group::GroupRole::Dps,
        });
    h.advance(1);

    // One owner-pace pull; the party fights from there.
    h.cast(30451, Some(ixin));
    // Pull-back at the live party's reach: Havoc The Hunt declares 100 yd
    // (party_unintended_reach with the APL action provider, 2026-11-01), so idle hostiles
    // such as the Engorged Crawlers 73 yd away cover the party. Live, the wait held the tank 22.6 yd
    // from Ixin and kept Fury and Havoc out of the fight until Ixin died.
    let follower = h.app.world().get::<super::Follower>(tank).unwrap().clone();
    let party: Vec<_> = members.iter().copied().chain([owner]).collect();
    let mut waited = false;
    for _ in 0..200 {
        super::owner_trail::record(h.app.world_mut(), owner);
        super::pull_history::sample_all(h.app.world_mut());
        super::tank_pull_back::update(h.app.world_mut(), tank, &follower, &party, LIVE_PARTY_REACH);
        waited |= super::tank_pull_back::party_should_wait(h.app.world(), owner);
        h.advance(1);
    }
    assert!(
        super::ai::engaged_enemies(h.app.world_mut(), &follower, tank, &party)
            .iter()
            .any(|(enemy, _)| *enemy == ixin),
        "the owner pull must engage Ixin: {:?}",
        h.inbox.failed
    );
    assert!(
        !waited,
        "pull-back must not hold the party for sessile Ixin"
    );
    let in_melee = |world: &World, unit: Entity| {
        crate::scripted_ai::model_reach::melee_range(world, unit, ixin)
            .zip(crate::spell_cast::targets::position(world, unit))
            .is_some_and(|(range, point)| point.distance(ixin_at) <= range)
    };
    let mut tank_in_melee = false;
    let mut dps_engaged = [false; 2];
    for _ in 0..3600 {
        super::update_ai(h.app.world_mut());
        h.advance(1);
        let world = h.app.world();
        tank_in_melee |= in_melee(world, tank);
        for (index, unit) in dps.iter().enumerate() {
            dps_engaged[index] |= world
                .get::<CombatTarget>(*unit)
                .is_some_and(|target| target.0 == Some(ixin))
                && in_melee(world, *unit)
                && !super::holding_pull_damage(world, *unit);
        }
        if tank_in_melee && dps_engaged.iter().all(|engaged| *engaged) {
            break;
        }
    }
    let world = h.app.world();
    assert!(
        tank_in_melee,
        "tank must reach melee with sessile Ixin: {:?}",
        crate::spell_cast::targets::position(world, tank)
    );
    assert!(
        dps_engaged.iter().all(|engaged| *engaged),
        "Fury and Havoc must engage Ixin in melee: {dps_engaged:?} at {:?}",
        dps.map(|unit| crate::spell_cast::targets::position(world, unit))
    );
    for unit in neighbours {
        let health = world.get::<Health>(unit).unwrap();
        assert!(
            health.current == health.max
                && world
                    .get::<CombatTarget>(unit)
                    .is_none_or(|victim| victim.0.is_none()),
            "idle neighbour {unit:?} must stay unpulled"
        );
    }
}
