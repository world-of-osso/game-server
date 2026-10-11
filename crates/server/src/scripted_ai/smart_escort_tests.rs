//! Real TDB Erland quest-accept → timed-list → waypoint-start boundary.
use super::*;
use shared::components::{Npc, Position};
use shared::protocol::NpcFlags;

fn erland() -> (tempfile::TempDir, World, SmartAi, Entity, Entity) {
    let source = crate::test_data::world_db_path();
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("escort.sqlite");
    let fixture = Connection::open(&path).unwrap();
    fixture
        .execute("ATTACH DATABASE ?1 AS tdb", [source.to_str().unwrap()])
        .unwrap();
    fixture.execute_batch(
        "CREATE TABLE content_creature_template AS SELECT * FROM tdb.tdb_creature_template WHERE entry=1978;
         CREATE TABLE tdb_smart_scripts AS SELECT * FROM tdb.tdb_smart_scripts WHERE entryorguid=1978 OR entryorguid BETWEEN 197800 AND 197805;
         CREATE TABLE tdb_waypoint_path AS SELECT * FROM tdb.tdb_waypoint_path WHERE PathId=15825;
         CREATE TABLE tdb_waypoint_path_node AS SELECT * FROM tdb.tdb_waypoint_path_node WHERE PathId=15825;"
    ).unwrap();
    drop(fixture);
    let scripts = load_smart_scripts(&path).unwrap();
    let events = scripts.0[&1978].clone();
    let mut world = World::new();
    world.insert_resource(scripts);
    let creature = world
        .spawn((
            Npc {
                template_id: 1978,
                name: "Deathstalker Erland".into(),
            },
            Position {
                x: 0.0,
                y: 0.0,
                z: 0.0,
            },
            NpcFlags(2),
        ))
        .id();
    let player = world.spawn_empty().id();
    let mut ai = SmartAi {
        states: vec![EventState::default(); events.len()],
        events,
        phase: 0,
        boss: None,
        stored_targets: HashMap::new(),
        timed_list: None,
        escort: None,
    };
    ai.reset(&mut Ctx {
        world: &mut world,
        me: creature,
    });
    (temp, world, ai, creature, player)
}

#[test]
fn smartai_coverage_erland_accept_starts_authored_path_and_removes_questgiver() {
    let (temp, mut world, mut ai, creature, player) = erland();
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.process_events_for(&mut ctx, 19, Some(player), 435);
    ai.update_ai(&mut ctx, 50);
    assert_eq!(ctx.world.get::<NpcFlags>(creature).unwrap().0 & 2, 0);
    let movement = ctx
        .world
        .get::<super::super::movement::PointMovement>(creature)
        .expect("TDB action list 197800 must start waypoint path 15825");
    assert_eq!(movement.id, 1);
    let conn = Connection::open_with_flags(
        temp.path().join("escort.sqlite"),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let wow: (f32, f32, f32) = conn.query_row(
        "SELECT PositionX, PositionY, PositionZ FROM tdb_waypoint_path_node WHERE PathId=15825 AND NodeId=1",
        [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
    ).unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(wow.0, wow.1, wow.2);
    assert_eq!(movement.target, Position { x, y, z });
}

#[test]
fn smartai_coverage_erland_waypoint_two_pauses_then_resumes() {
    let (_temp, mut world, mut ai, creature, player) = erland();
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.process_events_for(&mut ctx, 19, Some(player), 435);
    ai.update_ai(&mut ctx, 50);
    ctx.world
        .entity_mut(creature)
        .remove::<super::super::movement::PointMovement>();
    ai.movement_inform(&mut ctx, super::super::MotionType::Point, 1);
    assert_eq!(
        ctx.world
            .get::<super::super::movement::PointMovement>(creature)
            .unwrap()
            .id,
        2
    );
    ctx.world
        .entity_mut(creature)
        .remove::<super::super::movement::PointMovement>();
    ai.movement_inform(&mut ctx, super::super::MotionType::Point, 2);
    ai.update_ai(&mut ctx, 50);
    assert!(
        ctx.world
            .get::<super::super::movement::PointMovement>(creature)
            .is_none(),
        "authored pause at node two"
    );
    ai.update_ai(&mut ctx, 950);
    assert_eq!(
        ctx.world
            .get::<super::super::movement::PointMovement>(creature)
            .unwrap()
            .id,
        3
    );
}

fn install_quest_log(temp: &tempfile::TempDir, world: &mut World, player: Entity) {
    use crate::persistence::quest_store::{ActiveQuestData, QuestLogData, save_quest_log};
    let db = redb::Database::create(temp.path().join("quest.redb")).unwrap();
    crate::persistence::create_tables(&db);
    let quests = crate::quest_data::test_quest_db().unwrap().clone();
    assert!(quests.quests[&435].objectives.iter().any(|objective| objective.kind == crate::quest_data::ObjectiveKind::ExplorationOrEvent));
    save_quest_log(
        &db,
        1,
        &QuestLogData {
            active: vec![ActiveQuestData {
                quest_id: 435,
                counts: vec![0; quests.quests[&435].objectives.len()],
                ..Default::default()
            }],
            ..Default::default()
        },
    );
    world.insert_resource(crate::persistence::GameDatabase { db });
    world.insert_resource(quests);
    world.insert_resource(crate::quest::QuestDirty::default());
    world
        .entity_mut(player)
        .insert(crate::persistence::ClientId(1));
}

#[test]
fn smartai_coverage_erland_death_fails_the_stored_players_quest() {
    let (temp, mut world, mut ai, creature, player) = erland();
    install_quest_log(&temp, &mut world, player);
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.quest_accepted(&mut ctx, player, 435);
    ai.update_ai(&mut ctx, 50);
    ai.just_died(&mut ctx, None);
    let db = &ctx.world.resource::<crate::persistence::GameDatabase>().db;
    assert!(
        crate::persistence::quest_store::load_quest_log(db, 1)
            .active(435)
            .unwrap()
            .failed
    );
}

#[test]
fn smartai_coverage_erland_node_fourteen_credits_only_the_event_objective() {
    let (temp, mut world, mut ai, creature, player) = erland();
    install_quest_log(&temp, &mut world, player);
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.quest_accepted(&mut ctx, player, 435);
    ai.update_ai(&mut ctx, 50);
    // Deliver the real movement boundary, honoring every authored pause.
    for node_id in 1..=14 {
        ctx.world
            .entity_mut(creature)
            .remove::<super::super::movement::PointMovement>();
        ai.movement_inform(&mut ctx, super::super::MotionType::Point, node_id);
        ai.update_ai(&mut ctx, 1000);
    }
    let db = &ctx.world.resource::<crate::persistence::GameDatabase>().db;
    let log = crate::persistence::quest_store::load_quest_log(db, 1);
    assert_eq!(log.active(435).unwrap().counts, vec![1]);
    assert!(!log.active(435).unwrap().failed);
}

#[derive(Clone, Default)]
struct WarningCapture(std::sync::Arc<std::sync::Mutex<Vec<String>>>);

impl bevy::log::tracing::Subscriber for WarningCapture {
    fn enabled(&self, _: &bevy::log::tracing::Metadata<'_>) -> bool {
        true
    }
    fn register_callsite(
        &self,
        _: &'static bevy::log::tracing::Metadata<'static>,
    ) -> bevy::log::tracing::subscriber::Interest {
        bevy::log::tracing::subscriber::Interest::always()
    }
    fn max_level_hint(&self) -> Option<bevy::log::tracing::level_filters::LevelFilter> {
        Some(bevy::log::tracing::level_filters::LevelFilter::TRACE)
    }
    fn new_span(
        &self,
        _: &bevy::log::tracing::span::Attributes<'_>,
    ) -> bevy::log::tracing::span::Id {
        bevy::log::tracing::span::Id::from_u64(1)
    }
    fn record(&self, _: &bevy::log::tracing::span::Id, _: &bevy::log::tracing::span::Record<'_>) {}
    fn record_follows_from(
        &self,
        _: &bevy::log::tracing::span::Id,
        _: &bevy::log::tracing::span::Id,
    ) {
    }
    fn enter(&self, _: &bevy::log::tracing::span::Id) {}
    fn exit(&self, _: &bevy::log::tracing::span::Id) {}
    fn event(&self, event: &bevy::log::tracing::Event<'_>) {
        if *event.metadata().level() == bevy::log::tracing::Level::WARN {
            let mut visitor = WarningFields(String::new());
            event.record(&mut visitor);
            self.0.lock().unwrap().push(visitor.0);
        }
    }
}

struct WarningFields(String);
impl bevy::log::tracing::field::Visit for WarningFields {
    fn record_debug(
        &mut self,
        field: &bevy::log::tracing::field::Field,
        value: &dyn std::fmt::Debug,
    ) {
        self.0.push_str(&format!("{}={value:?};", field.name()));
    }
}

#[test]
fn smartai_coverage_warns_once_per_unsupported_type_including_timed_lists() {
    const PROBE: &str = "SMARTAI_WARNING_PROBE";
    if std::env::var_os(PROBE).is_none() {
        // tracing callsite interest is process-global; isolate from other tests'
        // subscribers so this proves startup logging rather than their cache races.
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "smartai_coverage_warns_once_per_unsupported_type_including_timed_lists",
                "--nocapture",
            ])
            .env(PROBE, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }
    let (temp, _world, _ai, _creature, _player) = erland();
    let capture = WarningCapture::default();
    bevy::log::tracing::subscriber::with_default(capture.clone(), || {
        load_smart_scripts(&temp.path().join("escort.sqlite")).unwrap();
    });
    let warnings = capture.0.lock().unwrap();
    assert_eq!(
        warnings
            .iter()
            .filter(|line| line.contains("action type 41"))
            .count(),
        1,
        "unsupported DESPAWN must produce one actionable warning: {warnings:?}"
    );
    assert_eq!(
        warnings
            .iter()
            .filter(|line| line.contains("target type 19"))
            .count(),
        1,
        "timed-list targets must be diagnosed: {warnings:?}"
    );
}

fn tdb_action(entry: i64, source: u32, id: u32) -> SmartEvent {
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    conn.query_row(
        "SELECT * FROM tdb_smart_scripts WHERE entryorguid=?1 AND source_type=?2 AND id=?3",
        (entry, source, id),
        |row| {
            Ok(SmartEvent {
                id: row.get("id")?,
                link: row.get("link")?,
                event_type: row.get("event_type")?,
                phase_mask: row.get("event_phase_mask")?,
                chance: row.get("event_chance")?,
                flags: row.get("event_flags")?,
                event_params: std::array::from_fn(|i| {
                    row.get(format!("event_param{}", i + 1).as_str()).unwrap()
                }),
                action_type: row.get("action_type")?,
                action_params: std::array::from_fn(|i| {
                    row.get(format!("action_param{}", i + 1).as_str()).unwrap()
                }),
                target_type: row.get("target_type")?,
                target_params: std::array::from_fn(|i| {
                    row.get(format!("target_param{}", i + 1).as_str()).unwrap()
                }),
                target_position: [
                    row.get("target_x")?,
                    row.get("target_y")?,
                    row.get("target_z")?,
                    row.get("target_o")?,
                ],
            })
        },
    )
    .unwrap()
}

#[test]
fn smartai_coverage_authored_faction_and_npc_flags_change_observable_state() {
    let (_temp, mut world, mut ai, creature, _player) = erland();
    world
        .entity_mut(creature)
        .insert(shared::components::UnitFactionTemplate(35));
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.execute_action(&mut ctx, &tdb_action(775, 0, 2), None);
    assert_eq!(
        ctx.world
            .get::<shared::components::UnitFactionTemplate>(creature)
            .unwrap()
            .0,
        46
    );
    ai.execute_action(&mut ctx, &tdb_action(4963, 0, 1), None);
    assert_eq!(ctx.world.get::<NpcFlags>(creature).unwrap().0, 1);
    ai.execute_action(&mut ctx, &tdb_action(2610, 0, 2), None);
    assert_eq!(ctx.world.get::<NpcFlags>(creature).unwrap().0, 3);
}

#[test]
fn smartai_coverage_wp_stop_cancels_movement() {
    let (_temp, mut world, mut ai, creature, player) = erland();
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.quest_accepted(&mut ctx, player, 435);
    ai.update_ai(&mut ctx, 50);
    ai.execute_action(&mut ctx, &tdb_action(14323, 0, 9), None);
    assert!(
        ctx.world
            .get::<super::super::movement::PointMovement>(creature)
            .is_none()
    );
    ai.update_ai(&mut ctx, 1000);
    assert!(
        ctx.world
            .get::<super::super::movement::PointMovement>(creature)
            .is_none()
    );
}

#[test]
fn smartai_coverage_erland_set_run_survives_the_next_leg() {
    let (_temp, mut world, mut ai, creature, player) = erland();
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.quest_accepted(&mut ctx, player, 435);
    ai.update_ai(&mut ctx, 50);
    ai.execute_action(&mut ctx, &tdb_action(197802, 9, 3), None);
    ctx.world
        .entity_mut(creature)
        .remove::<super::super::movement::PointMovement>();
    ai.movement_inform(&mut ctx, super::super::MotionType::Point, 1);
    assert!(
        ctx.world
            .get::<super::super::movement::Walking>(creature)
            .is_none()
    );
}
