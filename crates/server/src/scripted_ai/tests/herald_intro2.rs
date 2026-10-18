//! Natural INTRO2 boundary plus existing target-32 grounding proof; no phase stubs.
use super::herald_intro::{actors_in_copy, kill_actor, ordinary_ramparts_at, spawn_second_copy};
use super::*;
use shared::components::UnitPose;

const HERALD: u32 = 17_307;
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

fn prepared_ramparts(difficulty: u32) -> (Harness, InMap, Entity, Vec<Entity>) {
    let (_directory, path) = prepare_intro2_copy();
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (mut h, map, herald, sentries) = ordinary_ramparts_at(difficulty);
    load_prepared_templates(&mut h, &conn);
    h.app.insert_resource(crate::test_data::ground_map());
    for (_, connection, _) in &h.players {
        h.app.world_mut().entity_mut(*connection).insert((
            MessageSender::<shared::protocol::EmoteEvent>::default(),
            MessageReceiver::<shared::protocol::EmoteEvent>::default(),
        ));
    }
    (h, map, herald, sentries)
}

fn reach_center(h: &mut Harness, herald: Entity, sentries: &[Entity]) -> Duration {
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

fn ai_identity(h: &Harness, actor: Entity) -> *const () {
    let state = h
        .app
        .world()
        .get::<CreatureAiState>(actor)
        .expect("live source Herald AI");
    std::ptr::from_ref(state.ai.as_deref().unwrap()) as *const ()
}

fn assert_native_nazan(h: &Harness, actor: Entity, difficulty: u32) {
    use crate::level_scaling::{CreatureArmor, LevelStats, TuningInputs};
    let (tuning, health_modifier, damage_modifier) = match difficulty {
        1 => (220, 9.0, 1.0),
        2 => (1065, 12.0, 13.0),
        _ => panic!("source Normal/Heroic only"),
    };
    let stats = LevelStats::build(
        &h.app.world().resource::<WorldData>().expected_stats,
        &TuningInputs {
            content_tuning_id: tuning,
            health_scaling_expansion: 1,
            unit_class: 1,
            health_modifier,
            damage_modifier,
            armor_modifier: 1.0,
        },
    );
    assert_eq!(h.app.world().get::<UnitLevel>(actor), Some(&UnitLevel(30)));
    assert_eq!(h.health(actor).max, stats.max_health(30));
    assert_eq!(h.health(actor).current, stats.max_health(30));
    assert_eq!(
        h.app.world().get::<CreatureArmor>(actor).unwrap().0,
        stats.armor(30)
    );
    assert_eq!(
        crate::level_scaling::UnitScaling::of(h.app.world(), actor).swing_damage_range(2.0),
        Some(stats.swing_damage_range(30, 2.0))
    );
}

fn assert_no_land_substitute(h: &mut Harness, herald: Entity, pose: UnitPose) {
    assert_eq!(
        h.app.world().get::<UnitPose>(herald),
        Some(&pose),
        "NPC_ONESHOT_EMOTE_UNIMPLEMENTED must not become a persistent pose"
    );
    let connections: Vec<_> = h
        .players
        .iter()
        .map(|(_, connection, _)| *connection)
        .collect();
    for connection in connections {
        assert!(
            drain::<shared::protocol::EmoteEvent>(h.app.world_mut(), connection).is_empty(),
            "no substitute social emote for the missing LAND one-shot"
        );
    }
}

fn assert_grounded(world: &mut World, map: InMap, actor: Entity) {
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

#[test]
#[ignore = "RED: natural center arrival has no one-second INTRO2 transition"]
fn encounter_coverage_herald_intro2_arrival_deadline_normal_and_heroic_keep_actor_ai_origin() {
    for difficulty in [1, 2] {
        let (mut h, map, herald, sentries) = prepared_ramparts(difficulty);
        let origin = *h
            .app
            .world()
            .get::<crate::zones::CreatureOrigin>(herald)
            .unwrap();
        let ai = ai_identity(&h, herald);
        let pose = *h.app.world().get::<UnitPose>(herald).unwrap();
        let arrived_at = reach_center(&mut h, herald, &sentries);
        h.advance(0.95);
        assert_eq!(
            h.app.world().resource::<Time<Fixed>>().elapsed(),
            arrived_at + Duration::from_millis(950)
        );
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::Npc>(herald)
                .unwrap()
                .template_id,
            HERALD
        );
        assert!(actors_in_copy(&mut h, VAZRUDEN, map).is_empty());
        assert_no_land_substitute(&mut h, herald, pose);
        h.advance(0.05);
        assert_eq!(
            h.app.world().resource::<Time<Fixed>>().elapsed(),
            arrived_at + Duration::from_secs(1)
        );
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::Npc>(herald)
                .unwrap()
                .template_id,
            NAZAN,
            "actual arrival+1s must update the existing Herald to Nazan"
        );
        assert_eq!(ai_identity(&h, herald), ai);
        assert_eq!(
            h.app.world().get::<crate::zones::CreatureOrigin>(herald),
            Some(&origin)
        );
        assert_eq!(h.app.world().get::<InMap>(herald), Some(&map));
        assert_native_nazan(&h, herald, difficulty);
        assert_no_land_substitute(&mut h, herald, pose);
        let summoned = actors_in_copy(&mut h, VAZRUDEN, map);
        assert_eq!(summoned.len(), 1, "one actual17537, no synthetic actor");
        let child = summoned[0];
        assert_eq!(
            h.app
                .world()
                .get::<crate::zones::TempSummon>(child)
                .unwrap()
                .summoner,
            herald
        );
        assert_eq!(
            h.app
                .world()
                .get::<crate::spell_cast::summon::SummonedCreature>(child)
                .unwrap()
                .properties_id,
            64
        );
        assert_grounded(h.app.world_mut(), map, child);
        h.advance(0.05);
        assert!(
            h.has_aura(child, 5301),
            "real Vazruden appearance defensive state"
        );
        assert_eq!(
            get_boss_state(h.app.world_mut(), map, 1),
            Some(EncounterState::InProgress)
        );
        let west_distance =
            Vec3::new(CENTER.x, CENTER.y, CENTER.z).distance(Vec3::new(WEST.x, WEST.y, WEST.z));
        let speed = h
            .app
            .world()
            .get::<crate::creature_ai::SpawnPoint>(herald)
            .unwrap()
            .speed_run
            * 7.0;
        let ticks = (west_distance / (speed * 0.05)).ceil() as usize;
        for _ in 0..=ticks {
            if h.app.world().get::<Position>(herald) == Some(&WEST) {
                break;
            }
            h.advance(0.05);
        }
        assert_eq!(
            h.app.world().get::<Position>(herald),
            Some(&WEST),
            "real first West arrival"
        );
        assert!(
            h.has_aura(herald, 18430),
            "source hover on real flight arrival"
        );
        assert_eq!(
            h.app.world().get::<ReactState>(herald),
            Some(&ReactState::Aggressive)
        );
        assert_eq!(
            ai_identity(&h, herald),
            ai,
            "the same live AI handles post-morph flight arrival"
        );
    }
}

#[test]
#[ignore = "RED: missing copy-local and duplicate INTRO2 continuation"]
fn encounter_coverage_herald_intro2_copy_and_duplicate_notifications_do_not_replay() {
    let (mut h, map, herald, sentries) = prepared_ramparts(1);
    let other_map = spawn_second_copy(&mut h);
    let other_herald = actors_in_copy(&mut h, HERALD, other_map)[0];
    let arrival = reach_center(&mut h, herald, &sentries);
    dispatch(
        h.app.world_mut(),
        herald,
        Hook::MovementInform(MotionType::Point, 0),
    );
    dispatch(
        h.app.world_mut(),
        herald,
        Hook::MovementInform(MotionType::Effect, 0),
    );
    h.advance(0.95);
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Npc>(herald)
            .unwrap()
            .template_id,
        HERALD
    );
    h.advance(0.05);
    assert_eq!(
        h.app.world().resource::<Time<Fixed>>().elapsed(),
        arrival + Duration::from_secs(1)
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Npc>(herald)
            .unwrap()
            .template_id,
        NAZAN
    );
    assert_eq!(
        h.app
            .world()
            .get::<shared::components::Npc>(other_herald)
            .unwrap()
            .template_id,
        HERALD
    );
    assert!(actors_in_copy(&mut h, VAZRUDEN, other_map).is_empty());
    assert_eq!(actors_in_copy(&mut h, VAZRUDEN, map).len(), 1);
    dispatch(
        h.app.world_mut(),
        herald,
        Hook::MovementInform(MotionType::Point, 0),
    );
    dispatch(h.app.world_mut(), herald, Hook::DoAction(1_730_700));
    h.advance(1.1);
    assert_eq!(actors_in_copy(&mut h, VAZRUDEN, map).len(), 1);
    assert_eq!(
        h.casts(0, herald, 30717),
        1,
        "no replayed summon on duplicate arrival/action"
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
#[ignore = "RED: target32 must ignore radius, clamp to reach and ground before AI movement"]
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
#[ignore = "RED: flying target32 must preserve altitude above floor but raise it from below"]
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
