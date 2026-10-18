//! Source target32 summon primitive on actual Ramparts actors and geometry.
use super::herald_intro::{actors_in_copy, kill_actor, ordinary_ramparts_at};
use super::*;

const NAZAN: u32 = 17_536;
const VAZRUDEN: u32 = 17_537;
const CENTER: Position = Position {
    x: -1410.5204,
    y: 89.05998,
    z: -1740.5852,
};
const WEST: Position = Position {
    x: -1423.4910,
    y: 103.7056,
    z: -1710.4010,
};

fn prepare_intro2_copy() -> (tempfile::TempDir, PathBuf) {
    let (directory, path) = prepare_encounter_database("herald");
    run_preparation_script(
        "encounter_summons.py",
        &[
            "--encounter".as_ref(),
            "herald-nazan".as_ref(),
            "--db".as_ref(),
            path.as_os_str(),
        ],
    );
    import_intro_spells(&path);
    (directory, path)
}

fn import_intro_spells(path: &std::path::Path) {
    let dump = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/reference/tdb1210/TDB_full_world_1210.26091_2026_09_09.sql");
    run_preparation_script(
        "encounter_serverside_spells.py",
        &[
            "--encounter".as_ref(),
            "herald".as_ref(),
            "--db".as_ref(),
            path.as_os_str(),
            "--source-dump".as_ref(),
            dump.as_os_str(),
        ],
    );
}

fn load_prepared_templates(h: &mut Harness, conn: &Connection) {
    let mut data = h.app.world().resource::<WorldData>().clone();
    let selected: std::collections::HashSet<_> = data
        .templates
        .keys()
        .copied()
        .chain([NAZAN, VAZRUDEN])
        .collect();
    let mut templates = crate::world_data::load_templates(conn, &data.expected_stats);
    templates.retain(|entry, _| selected.contains(entry));
    crate::world_data::load_template_models(conn, &mut templates);
    data.difficulty_templates =
        crate::world_data::load_difficulty_templates(conn, &data.expected_stats, &templates);
    data.templates = templates;
    data.addons = crate::world_data::load_spawn_addons(conn);
    data.template_addons = crate::world_data::load_template_addons(conn);
    data.equipment = crate::test_data::world_data().equipment.clone();
    data.summon_levels = crate::world_data::summon_levels::SummonLevels::load(conn).unwrap();
    h.app.insert_resource(data);
    let mut spells = h.app.world().resource::<SpellStore>().clone();
    spells.append_serverside(conn).unwrap();
    h.app.insert_resource(spells);
}

pub(super) fn prepared_ramparts(difficulty: u32) -> (Harness, InMap, Entity, Vec<Entity>) {
    let (_directory, path) = prepare_intro2_copy();
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (mut h, map, herald, sentries) = ordinary_ramparts_at(difficulty);
    load_prepared_templates(&mut h, &conn);
    h.app.insert_resource(crate::test_data::ground_map());
    (h, map, herald, sentries)
}

pub(super) fn reach_center(h: &mut Harness, herald: Entity, sentries: &[Entity]) -> Duration {
    let listener = *h.app.world().get::<Position>(sentries[0]).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(listener);
    for sentry in sentries {
        kill_actor(h, *sentry);
    }
    h.advance(0.05);
    let position = *h.app.world().get::<Position>(herald).unwrap();
    let speed = h
        .app
        .world()
        .get::<crate::creature_ai::SpawnPoint>(herald)
        .unwrap()
        .speed_run
        * 7.0;
    let distance = Vec3::new(position.x, position.y, position.z)
        .distance(Vec3::new(CENTER.x, CENTER.y, CENTER.z));
    let ticks = (distance / (speed * 0.05)).ceil() as usize;
    for _ in 0..=ticks {
        if h.app.world().get::<Position>(herald) == Some(&CENTER) {
            return h.app.world().resource::<Time<Fixed>>().elapsed();
        }
        h.advance(0.05);
    }
    panic!("source center must be reached by real point movement");
}

pub(super) fn assert_grounded(world: &mut World, map: InMap, actor: Entity) {
    let position = *world.get::<Position>(actor).unwrap();
    let feet = Vec3::new(position.x, position.y, position.z);
    let probe = world
        .resource_mut::<crate::ground::GroundMap>()
        .creature_ground_in(map, feet);
    let crate::ground::GroundProbe::Supported(height) = probe else {
        panic!("source Ramparts floor is required for behavioral placement proof: {probe:?}");
    };
    assert_eq!(
        position.y, height,
        "30717 must create17537 on the actual source floor"
    );
}

fn elevated_nazan_for_target32() -> (Harness, InMap, Entity) {
    let (mut h, map, herald, sentries) = prepared_ramparts(1);
    reach_center(&mut h, herald, &sentries);
    // Isolate the existing entry primitive and real30717 cast before any INTRO2 code.
    let data = h.app.world().resource::<WorldData>();
    let placement = crate::world_data::CreatureSpawn {
        template_entry: NAZAN,
        position_x: WEST.x,
        position_y: -WEST.z,
        position_z: WEST.y,
        ..data
            .spawns
            .iter()
            .find(|spawn| spawn.guid == 20_202_693)
            .unwrap()
            .clone()
    };
    let projection = crate::zones::read_entry_projection(
        data,
        data.template_for(NAZAN, 1),
        &mut crate::zones::CreatureSpawnContext {
            map,
            difficulty_id: 1,
            ground: None,
            temporary: false,
        },
        &placement,
        None,
        &mut rand::thread_rng(),
    )
    .unwrap();
    assert!(projection.apply(h.app.world_mut().entity_mut(herald), false));
    h.app
        .world_mut()
        .entity_mut(herald)
        .insert((WEST, ReactState::Passive))
        .remove::<crate::scripted_ai::waypoint_path::GravityDisabled>();
    (h, map, herald)
}

#[test]
fn encounter_coverage_herald_target32_30717_uses_source_reach_and_ground_before_ai_moves() {
    let (mut h, map, herald) = elevated_nazan_for_target32();
    let spell = h.app.world().resource::<SpellStore>().get(30717).unwrap();
    assert_eq!(spell.effects[0].implicit_targets, [32, 0]);
    let destination = crate::spell_cast::targets::effect_dest(
        h.app.world(),
        herald,
        Some(herald),
        &spell.effects[0],
        None,
    )
    .unwrap();
    let ground = h
        .app
        .world_mut()
        .resource_mut::<crate::ground::GroundMap>()
        .creature_ground_in(map, destination.position);
    assert!(
        matches!(ground, crate::ground::GroundProbe::Supported(_)),
        "actual Ramparts ground data must resolve before feature proof: {ground:?}"
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), herald, 30717, Some(herald), false)
        .expect("ordinary source30717 self-cast");
    let summoned = actors_in_copy(&mut h, VAZRUDEN, map);
    assert_eq!(summoned.len(), 1);
    // Check at spawn, before a later chase/grounding tick can mask bad placement.
    let position = h.app.world().get::<Position>(summoned[0]).unwrap();
    let horizontal = Vec2::new(position.x - WEST.x, position.z - WEST.z).length();
    let reach = crate::scripted_ai::model_reach::combat_reach(h.app.world(), herald)
        .expect("source Nazan model combat reach");
    assert_eq!(reach, 10.0, "actual18812 source model metadata");
    assert!(
        (horizontal - reach.max(1.0)).abs() < 0.001,
        "target32 ignores3yd spell radius: distance=max(PET_FOLLOW_DIST1,reach), got{horizontal}"
    );
    assert_grounded(h.app.world_mut(), map, summoned[0]);
}

#[test]
fn encounter_coverage_herald_target32_gravity_disabled_preserves_above_and_raises_below() {
    for below_floor in [false, true] {
        //30717 has a real cooldown: independent casters isolate each geometry case.
        let (mut h, map, herald) = elevated_nazan_for_target32();
        h.app
            .world_mut()
            .entity_mut(herald)
            .insert(crate::scripted_ai::waypoint_path::GravityDisabled);
        let spell = h.app.world().resource::<SpellStore>().get(30717).unwrap();
        let destination = crate::spell_cast::targets::effect_dest(
            h.app.world(),
            herald,
            Some(herald),
            &spell.effects[0],
            None,
        )
        .unwrap();
        let crate::ground::GroundProbe::Supported(floor) = h
            .app
            .world_mut()
            .resource_mut::<crate::ground::GroundMap>()
            .creature_ground_in(map, destination.position)
        else {
            panic!("real source destination floor");
        };
        if below_floor {
            h.app.world_mut().entity_mut(herald).insert(Position {
                y: floor - 0.5,
                ..WEST
            });
        }
        crate::spell_cast::creature_cast(h.app.world_mut(), herald, 30717, Some(herald), false)
            .expect("normal30717 with source gravity disabled");
        let summoned = actors_in_copy(&mut h, VAZRUDEN, map);
        assert_eq!(summoned.len(), 1);
        if below_floor {
            assert_grounded(h.app.world_mut(), map, summoned[0]);
        } else {
            assert_eq!(
                h.app.world().get::<Position>(summoned[0]).unwrap().y,
                WEST.y,
                "CanFly must not be forced down to ground"
            );
        }
    }
}

#[test]
fn encounter_coverage_herald_target32_force_uses_height_backoff_and_ground_position() {
    let (mut h, map, herald) = elevated_nazan_for_target32();
    h.app
        .world_mut()
        .entity_mut(herald)
        .insert(crate::scripted_ai::waypoint_path::GravityDisabled);
    let mut spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(30717)
        .unwrap()
        .clone();
    let wanted = crate::spell_cast::targets::effect_dest(
        h.app.world(),
        herald,
        Some(herald),
        &spell.effects[0],
        None,
    )
    .unwrap()
    .position;
    let origin = Vec3::new(WEST.x, WEST.y, WEST.z);
    for step in 0..10 {
        let probe = origin.lerp(wanted, 1.0 - step as f32 / 10.0) + Vec3::Y * 6.0;
        let crate::ground::GroundProbe::Supported(floor) = h
            .app
            .world_mut()
            .resource_mut::<crate::ground::GroundMap>()
            .creature_ground_in(map, probe)
        else {
            panic!("real source floor throughout the forced-offset control");
        };
        assert!(
            (WEST.y - floor).abs() > 6.0,
            "source control has no acceptable6yd height change along the offset"
        );
    }
    // Counterfactual source-defined flag control; no database/authored row is changed.
    assert_eq!(spell.misc.as_ref().unwrap().attributes[9] & 0x1, 0);
    spell.misc.as_mut().unwrap().attributes[9] |= 0x1;
    h.app.insert_resource(SpellStore::fixture(vec![spell]));
    crate::spell_cast::creature_cast(h.app.world_mut(), herald, 30717, Some(herald), false)
        .expect("ordinary source32 cast with the explicit FORCE flag control");
    let child = actors_in_copy(&mut h, VAZRUDEN, map)[0];
    assert_grounded(h.app.world_mut(), map, child);
    let position = h.app.world().get::<Position>(child).unwrap();
    assert_eq!(
        (position.x, position.z),
        (WEST.x, WEST.z),
        "all ten offsets exceed6yd: TC retains XY then updates ground Z"
    );
}
