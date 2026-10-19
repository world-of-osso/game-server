//! Natural center-arrival INTRO2 transition through the first real West arrival.
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

fn prepared_source_ramparts(difficulty: u32) -> (Harness, InMap, Entity, Vec<Entity>) {
    let (_directory, path) = prepare_intro2_copy();
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let (mut h, map, herald, sentries) = ordinary_ramparts_at(difficulty);
    load_prepared_templates(&mut h, &conn);
    h.app.insert_resource(crate::test_data::ground_map());
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

fn prepared_ramparts(difficulty: u32) -> (Harness, InMap, Entity, Vec<Entity>) {
    let (mut h, map, herald, sentries) = prepared_source_ramparts(difficulty);
    for (_, connection, _) in &h.players {
        h.app.world_mut().entity_mut(*connection).insert((
            MessageSender::<shared::protocol::EmoteEvent>::default(),
            MessageReceiver::<shared::protocol::EmoteEvent>::default(),
        ));
    }
    (h, map, herald, sentries)
}

fn received_casts(h: &Harness, caster: Entity, spell_id: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == caster.to_bits() && go.spell_id == spell_id)
        .count()
}

fn assert_summon_at_floor(world: &mut World, map: InMap, actor: Entity, pre_clamp_y: f32) {
    let position = *world.get::<Position>(actor).unwrap();
    let probe = world
        .resource_mut::<crate::ground::GroundMap>()
        .creature_ground_in(map, Vec3::new(position.x, pre_clamp_y, position.z));
    let crate::ground::GroundProbe::Supported(floor) = probe else {
        panic!("actual Ramparts source floor required: {probe:?}");
    };
    assert_eq!(
        position.y.to_bits(),
        floor.to_bits(),
        "30717 must assign the exact source floor at final XZ with its observed pre-clamp caster Y/ray"
    );
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

#[test]
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
        let cast_origin = *h.app.world().get::<Position>(herald).unwrap();
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
        assert_eq!(
            h.app.world().get::<Position>(herald),
            Some(&cast_origin),
            "30717 changes destination Z, not caster position; first-flight movement starts after this cast tick"
        );
        assert_summon_at_floor(h.app.world_mut(), map, child, cast_origin.y);
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
            let fireball = if difficulty == 2 { 32491 } else { 30691 };
            assert_eq!(
                h.casts(0, herald, fireball),
                0,
                "source Passive suppresses due victim Fireball during the first flight"
            );
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
        let threat_target = *h.app.world().get::<Position>(h.player()).unwrap();
        let facing = (WEST.z - threat_target.z).atan2(threat_target.x - WEST.x)
            + std::f32::consts::FRAC_PI_2;
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::Rotation>(herald)
                .unwrap()
                .y
                .to_bits(),
            facing.to_bits(),
            "30700 faces the sole living same-copy threat target"
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
        for spell in [30717, 30698, 30694, 30700, 18430] {
            assert_eq!(
                received_casts(&h, herald, spell),
                1,
                "one source transition cast for{spell}"
            );
        }
        assert_no_land_substitute(&mut h, herald, pose);
    }
}

#[test]
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
        received_casts(&h, herald, 30717),
        1,
        "no replayed summon on duplicate arrival/action"
    );
}
