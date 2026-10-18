//! Real TDB Erland quest-accept → timed-list → waypoint-start boundary.

#[path = "smart_feero_tests.rs"]
mod feero;
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
    let player = world
        .spawn(shared::components::Player {
            name: "fb_escort".into(),
            race: 5,
            class: 1,
            appearance: Default::default(),
        })
        .id();
    let mut ai = SmartAi {
        states: vec![EventState::default(); events.len()],
        events,
        phase: 0,
        boss: None,
        stored_targets: HashMap::new(),
        counters: HashMap::new(),
        timed_list: None,
        processing_timed_list: false,
        escort: None,
        path_event_id: None,
        follow: None,
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
    world.insert_resource(crate::persistence::GameDatabase::new(db));
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
    let conn = Connection::open(temp.path().join("escort.sqlite")).unwrap();
    conn.execute(
        "UPDATE tdb_smart_scripts SET action_type=999,target_type=998 WHERE source_type=9",
        [],
    )
    .unwrap();
    let capture = WarningCapture::default();
    bevy::log::tracing::subscriber::with_default(capture.clone(), || {
        load_smart_scripts(&temp.path().join("escort.sqlite")).unwrap();
    });
    let warnings = capture.0.lock().unwrap();
    assert_eq!(
        warnings
            .iter()
            .filter(|line| line.contains("action type 999"))
            .count(),
        1,
        "unsupported action must produce one actionable warning: {warnings:?}"
    );
    assert_eq!(
        warnings
            .iter()
            .filter(|line| line.contains("target type 998"))
            .count(),
        1,
        "timed-list targets must be diagnosed: {warnings:?}"
    );
}

#[test]
fn smartai_coverage_ooc_timed_list_waits_through_combat() {
    let (_temp, mut world, mut ai, creature, player) = erland();
    world.entity_mut(creature).insert((
        shared::components::Health {
            current: 100.0,
            max: 100.0,
        },
        crate::creature_ai::CreatureState::Pursuing {
            target: player.to_bits(),
        },
    ));
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.quest_accepted(&mut ctx, player, 435);
    ai.update_ai(&mut ctx, 1000);
    assert_eq!(ctx.world.get::<NpcFlags>(creature).unwrap().0, 2);
    ctx.world
        .entity_mut(creature)
        .insert(crate::creature_ai::CreatureState::Idle);
    ai.update_ai(&mut ctx, 50);
    assert_eq!(ctx.world.get::<NpcFlags>(creature).unwrap().0, 0);
}

#[test]
fn smartai_coverage_ranked_orientation_turns_actor_not_selected_target() {
    use shared::components::Rotation;
    let (_temp, mut world, mut ai, creature, player) = erland();
    world.entity_mut(creature).insert(Rotation {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    world.entity_mut(player).insert((
        Position {
            x: 10.0,
            y: 0.0,
            z: 0.0,
        },
        Rotation {
            x: 0.0,
            y: 0.7,
            z: 0.0,
        },
    ));
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    let mut action = tdb_action(-650127, 0, 1);
    ai.execute_action(&mut ctx, &action, None);
    let expected = action.target_position[3] + std::f32::consts::FRAC_PI_2;
    assert!((ctx.world.get::<Rotation>(creature).unwrap().y - expected).abs() < 0.0001);
    action.target_type = TARGET_ACTION_INVOKER;
    ai.execute_action(&mut ctx, &action, Some(player));
    assert!(
        (ctx.world.get::<Rotation>(creature).unwrap().y - std::f32::consts::FRAC_PI_2).abs()
            < 0.0001
    );
    assert_eq!(ctx.world.get::<Rotation>(player).unwrap().y, 0.7);
    ctx.world
        .entity_mut(creature)
        .insert(crate::creature_ai::HomeOrientation(4.25));
    action.target_type = TARGET_SELF;
    ai.execute_action(&mut ctx, &action, None);
    assert_eq!(ctx.world.get::<Rotation>(creature).unwrap().y, 4.25);
}

#[test]
fn smartai_coverage_ranked_emote_state_changes_selected_unit_and_preserves_stance() {
    use shared::components::UnitPose;
    let (_temp, mut world, mut ai, creature, player) = erland();
    let mut pose = UnitPose::default();
    pose.stand_state = shared::components::StandState::Sit;
    world.entity_mut(creature).insert(pose);
    world.entity_mut(player).insert(pose);
    let mut action = tdb_action(-313655, 0, 0);
    action.target_type = TARGET_ACTION_INVOKER;
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.execute_action(&mut ctx, &action, Some(player));
    assert_eq!(ctx.world.get::<UnitPose>(player).unwrap().emote_state, 233);
    assert_eq!(
        ctx.world.get::<UnitPose>(player).unwrap().stand_state,
        shared::components::StandState::Sit
    );
    assert_eq!(ctx.world.get::<UnitPose>(creature).unwrap().emote_state, 0);
    action.action_params[0] = 0;
    ai.execute_action(&mut ctx, &action, Some(player));
    assert_eq!(ctx.world.get::<UnitPose>(player).unwrap().emote_state, 0);
}

#[test]
fn smartai_coverage_timed_list_chance_controls_observable_flag_action() {
    let (_temp, mut world, mut ai, creature, player) = erland();
    let mut action = tdb_action(197800, 9, 2);
    action.event_params = [0; 5];
    let mut call = tdb_action(1978, 0, 1);
    call.action_params = [900002, 2, 1, 0, 0, 0, 0];
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    for chance in [0, 50, 100] {
        action.chance = chance;
        ctx.world
            .resource_mut::<SmartScripts>()
            .1
            .insert(900002, Arc::new(vec![action.clone()]));
        let mut executed = 0;
        for _ in 0..256 {
            ctx.world.get_mut::<NpcFlags>(creature).unwrap().0 = 2;
            ai.execute_action(&mut ctx, &call, Some(player));
            ai.update_timed_list(&mut ctx, 1);
            executed += usize::from(ctx.world.get::<NpcFlags>(creature).unwrap().0 == 0);
        }
        if chance == 50 {
            assert!(
                executed > 0 && executed < 256,
                "50-percent actions must both execute and skip: {executed}"
            );
        } else {
            assert_eq!(
                executed, 256,
                "pinned zero chance means unconditional, like100"
            );
        }
    }
}

#[test]
fn smartai_coverage_timed_list_random_initial_delay_respects_both_endpoints() {
    let (_temp, mut world, mut ai, creature, player) = erland();
    let mut action = tdb_action(197800, 9, 2);
    action.event_params = [20, 40, 0, 0, 0];
    let mut call = tdb_action(1978, 0, 1);
    call.action_params = [900003, 2, 1, 0, 0, 0, 0];
    world
        .resource_mut::<SmartScripts>()
        .1
        .insert(900003, Arc::new(vec![action]));
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    let mut delayed_past_minimum = false;
    for _ in 0..128 {
        ctx.world.get_mut::<NpcFlags>(creature).unwrap().0 = 2;
        ai.execute_action(&mut ctx, &call, Some(player));
        let mut observed = None;
        for elapsed in 1..=40 {
            ai.update_timed_list(&mut ctx, 1);
            if ctx.world.get::<NpcFlags>(creature).unwrap().0 == 0 {
                observed = Some(elapsed);
                break;
            }
        }
        let elapsed = observed.expect("an authored20..40ms flag action must run by its maximum");
        assert!(elapsed >= 20, "action ran before its minimum: {elapsed}");
        delayed_past_minimum |= elapsed > 20;
    }
    assert!(
        delayed_past_minimum,
        "variable delay must not always select its minimum"
    );
}

#[test]
fn smartai_coverage_nested_timed_list_does_not_start_after_outer_finishes() {
    let (_temp, mut world, mut ai, creature, player) = erland();
    let mut nested = tdb_action(1978, 0, 1);
    nested.event_params = [0; 5];
    world
        .resource_mut::<SmartScripts>()
        .1
        .insert(900001, Arc::new(vec![nested.clone()]));
    let mut outer = nested;
    outer.action_params[0] = 900001;
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.execute_action(&mut ctx, &outer, Some(player));
    ai.update_ai(&mut ctx, 50);
    ai.update_ai(&mut ctx, 50);
    assert_eq!(ctx.world.get::<NpcFlags>(creature).unwrap().0, 2);
    assert!(
        ctx.world
            .get::<super::super::movement::PointMovement>(creature)
            .is_none()
    );
}

fn start_quest_escort(ctx: &mut Ctx, ai: &mut SmartAi, player: Entity) {
    ctx.world
        .entity_mut(player)
        .insert(shared::components::Player {
            name: "fb_escort".into(),
            race: 5,
            class: 1,
            appearance: Default::default(),
        });
    let mut start = tdb_action(197800, 9, 3);
    start.action_params[3] = 435;
    start.target_type = TARGET_ACTION_INVOKER;
    ai.execute_action(ctx, &start, Some(player));
}

#[test]
fn smartai_coverage_escort_death_fails_quest_without_explicit_death_row() {
    let (temp, mut world, mut ai, creature, player) = erland();
    install_quest_log(&temp, &mut world, player);
    ai.events = Arc::new(
        ai.events
            .iter()
            .filter(|e| e.event_type != EVENT_DEATH)
            .cloned()
            .collect(),
    );
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    start_quest_escort(&mut ctx, &mut ai, player);
    ai.just_died(&mut ctx, None);
    let db = &ctx.world.resource::<crate::persistence::GameDatabase>().db;
    assert!(
        crate::persistence::quest_store::load_quest_log(db, 1)
            .active(435)
            .unwrap()
            .failed
    );
    assert!(
        ctx.world
            .get::<super::super::movement::PointMovement>(creature)
            .is_none()
    );
}

#[test]
fn smartai_coverage_out_of_range_escort_fails_without_explicit_death_row() {
    let (temp, mut world, mut ai, creature, player) = erland();
    install_quest_log(&temp, &mut world, player);
    world.entity_mut(player).insert(Position {
        x: 200.0,
        y: 0.0,
        z: 0.0,
    });
    ai.events = Arc::new(Vec::new());
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    start_quest_escort(&mut ctx, &mut ai, player);
    ai.update_ai(&mut ctx, 1001);
    let db = &ctx.world.resource::<crate::persistence::GameDatabase>().db;
    assert!(
        crate::persistence::quest_store::load_quest_log(db, 1)
            .active(435)
            .unwrap()
            .failed
    );
    assert!(
        ctx.world
            .get::<super::super::movement::PointMovement>(creature)
            .is_none()
    );
}

#[test]
fn smartai_coverage_node_delay_orientation_and_repeat_are_data_driven() {
    let (temp, mut world, mut ai, creature, player) = erland();
    let conn = Connection::open(temp.path().join("escort.sqlite")).unwrap();
    conn.execute("DELETE FROM tdb_waypoint_path_node WHERE NodeId>2", [])
        .unwrap();
    conn.execute(
        "UPDATE tdb_waypoint_path_node SET Delay=700,Orientation=1.25 WHERE NodeId=1",
        [],
    )
    .unwrap();
    world.insert_resource(load_smart_scripts(&temp.path().join("escort.sqlite")).unwrap());
    world
        .entity_mut(creature)
        .insert(shared::components::Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
    // No scripted pauses: this fixture exercises waypoint table delays, not action54.
    ai.events = Arc::new(Vec::new());
    let mut start = tdb_action(197800, 9, 3);
    start.action_params[2] = 1;
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.execute_action(&mut ctx, &start, Some(player));
    ctx.world
        .entity_mut(creature)
        .remove::<super::super::movement::PointMovement>();
    ai.movement_inform(&mut ctx, super::super::MotionType::Point, 1);
    assert_eq!(
        ctx.world
            .get::<shared::components::Rotation>(creature)
            .unwrap()
            .y,
        1.25 + std::f32::consts::FRAC_PI_2
    );
    ai.update_ai(&mut ctx, 699);
    assert!(
        ctx.world
            .get::<super::super::movement::PointMovement>(creature)
            .is_none()
    );
    ai.update_ai(&mut ctx, 1);
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
    assert_eq!(
        ctx.world
            .get::<super::super::movement::PointMovement>(creature)
            .unwrap()
            .id,
        1
    );
}

#[test]
fn smartai_coverage_party_targets_include_same_map_members_once() {
    let (_temp, mut world, mut ai, creature, player) = erland();
    let mut online = crate::group::OnlineView::new();
    let mut members = Vec::new();
    for (id, name, map) in [(1, "fb_one", 0), (2, "fb_two", 0), (3, "fb_othermap", 1)] {
        let entity = if id == 1 {
            player
        } else {
            world.spawn_empty().id()
        };
        world.entity_mut(entity).insert((
            crate::persistence::ClientId(id),
            shared::components::Player {
                name: name.into(),
                race: 5,
                class: 1,
                appearance: Default::default(),
            },
            crate::maps::InMap::continent(map),
            NpcFlags(2),
        ));
        online.insert(
            id,
            crate::group::OnlineMember {
                entity: entity.to_bits(),
                name: name.into(),
                class: 1,
                level: 10,
                portrait: Default::default(),
                dungeon_difficulty: 1,
                raid_difficulty: Default::default(),
            },
        );
        members.push(entity);
    }
    let mut groups = crate::group::GroupRegistry::default();
    groups.invite(1, "fb_two", &online);
    groups.respond_invite(2, true);
    groups.invite(1, "fb_othermap", &online);
    groups.respond_invite(3, true);
    world.insert_resource(groups);
    let mut action = tdb_action(197800, 9, 2);
    action.target_type = TARGET_INVOKER_PARTY;
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.execute_action(&mut ctx, &action, Some(player));
    assert_eq!(ctx.world.get::<NpcFlags>(members[0]).unwrap().0, 0);
    assert_eq!(ctx.world.get::<NpcFlags>(members[1]).unwrap().0, 0);
    assert_eq!(ctx.world.get::<NpcFlags>(members[2]).unwrap().0, 2);
}

#[test]
fn smartai_coverage_force_despawn_uses_authored_delay_and_respawn_without_death_credit() {
    let (_temp, mut world, mut ai, creature, player) = erland();
    world.entity_mut(creature).insert((
        crate::combat::RespawnDelay(120.0),
        crate::combat::CorpseDelay(60.0),
    ));
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.execute_action(&mut ctx, &tdb_action(197805, 9, 3), Some(player));
    lifetimes::tick_lifetimes(ctx.world, 29999);
    assert!(
        ctx.world
            .get::<crate::combat::DeathState>(creature)
            .is_none()
    );
    lifetimes::tick_lifetimes(ctx.world, 1);
    assert_eq!(
        ctx.world
            .get::<crate::combat::DeathState>(creature)
            .expect("authored despawn must leave a spawn awaiting respawn")
            .respawn_timer,
        60.0
    );
}

#[test]
fn smartai_coverage_despawning_a_native_corpse_interrupts_its_dead_start_cast() {
    use crate::spell_cast::test_harness::{harness, prepare_class_world};
    use shared::casting::CastState;

    const DEATH_TOUCH: u32 = 5;
    const NETHER_EXPLOSION: u32 = 35058;
    let data = prepare_class_world().world_data();
    let spawn = data
        .spawns
        .iter()
        .find(|spawn| spawn.guid == 20_004_147)
        .expect("native Brown Ram world spawn");
    let template = data.templates.get(&spawn.template_entry).unwrap();
    let mut h = harness(Vec::new(), &[DEATH_TOUCH, NETHER_EXPLOSION], 2).unwrap();
    let mut queue = bevy::ecs::world::CommandQueue::default();
    let creature = {
        let mut commands = bevy::prelude::Commands::new(&mut queue, h.app.world());
        crate::zones::spawn_creature(
            &mut commands,
            &data,
            template,
            &mut crate::zones::CreatureSpawnContext {
                map: crate::maps::InMap::continent(u32::from(spawn.map)),
                difficulty_id: 0,
                ground: Some(&mut crate::test_data::ground_map()),
                temporary: false,
            },
            false,
            spawn,
            &mut rand::thread_rng(),
        )
        .expect("native Brown Ram level context")
    };
    queue.apply(h.app.world_mut());
    assert!(
        h.app
            .world()
            .get::<crate::combat::RespawnDelay>(creature)
            .is_some()
    );
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        h.caster,
        DEATH_TOUCH,
        Some(creature),
        false,
    )
    .unwrap();
    assert!(
        h.app
            .world()
            .get::<crate::combat::Corpse>(creature)
            .is_some()
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), creature, NETHER_EXPLOSION, None, false)
        .unwrap();
    assert!(h.app.world().get::<CastState>(creature).is_some());
    // Respect the native world's existing respawn/corpse delays; no clock override.
    lifetimes::despawn_creature(h.app.world_mut(), creature, 0);
    assert!(
        h.app
            .world()
            .get::<crate::combat::Corpse>(creature)
            .is_none()
    );
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(creature)
            .is_some()
    );
    assert!(h.app.world().get::<CastState>(creature).is_none());
}

#[test]
fn smartai_coverage_wp_stop_completes_quest_then_hides_and_despawns() {
    let (temp, mut world, mut ai, creature, player) = erland();
    install_quest_log(&temp, &mut world, player);
    world.entity_mut(player).insert(Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    world.entity_mut(creature).insert((
        crate::combat::RespawnDelay(120.0),
        crate::combat::CorpseDelay(60.0),
        lightyear::prelude::Replicate::to_clients(lightyear::prelude::NetworkTarget::All),
    ));
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    start_quest_escort(&mut ctx, &mut ai, player);
    let mut stop = tdb_action(197800, 9, 3);
    stop.action_type = ACTION_WP_STOP;
    stop.action_params = [10, 435, 0, 0, 0, 0, 0];
    ai.execute_action(&mut ctx, &stop, Some(player));
    let db = &ctx.world.resource::<crate::persistence::GameDatabase>().db;
    assert_eq!(
        crate::persistence::quest_store::load_quest_log(db, 1)
            .active(435)
            .unwrap()
            .counts,
        vec![1]
    );
    lifetimes::tick_lifetimes(ctx.world, 10);
    assert!(
        ctx.world
            .get::<lightyear::prelude::Replicate>(creature)
            .is_none()
    );
    assert!(
        ctx.world
            .get::<crate::combat::DeathState>(creature)
            .is_none()
    );
    lifetimes::tick_lifetimes(ctx.world, 5000);
    assert!(
        ctx.world
            .get::<crate::combat::DeathState>(creature)
            .is_some()
    );
}

#[test]
fn smartai_coverage_summon_authentic_position_count_storage_and_expiry() {
    let (_temp, mut world, mut ai, creature, player) = erland();
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let expected = crate::world_data::load_expected_stats(&conn);
    let mut templates = crate::world_data::load_templates(&conn, &expected);
    templates.retain(|entry, _| *entry == 28636);
    crate::world_data::load_template_models(&conn, &mut templates);
    let mut query = conn.prepare("SELECT level,class,basehp0,basehp1,basehp2 FROM creature_classlevelstats WHERE class=1").unwrap();
    let class_level_base_hp = query
        .query_map([], |row| {
            Ok((
                (row.get(0)?, row.get(1)?),
                [row.get(2)?, row.get(3)?, row.get(4)?],
            ))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    world.insert_resource(crate::world_data::WorldData {
        templates,
        class_level_base_hp,
        ..Default::default()
    });
    world
        .entity_mut(creature)
        .insert(crate::maps::InMap::continent(0));
    let mut summoned_by = tdb_action(197800, 9, 2);
    summoned_by.event_type = 54;
    summoned_by.event_params = [0; 5];
    summoned_by.action_type = ACTION_SET_NPC_FLAG;
    summoned_by.action_params[0] = 8;
    summoned_by.target_type = TARGET_SELF;
    summoned_by.link = 0;
    world
        .resource_mut::<SmartScripts>()
        .0
        .insert(28636, Arc::new(vec![summoned_by]));
    let mut action = tdb_action(-98562, 0, 1);
    // Authored Heb'Jin position/type/duration; exercise generic count/storage params.
    action.action_params[4] = 2;
    action.action_params[5] = 9;
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.execute_action(&mut ctx, &action, Some(player));
    let summons: Vec<Entity> = ctx
        .world
        .query_filtered::<Entity, With<crate::zones::TempSummon>>()
        .iter(ctx.world)
        .collect();
    assert_eq!(summons.len(), 2);
    assert!(
        summons
            .iter()
            .all(|&summon| ctx.world.get::<NpcFlags>(summon).unwrap().0 == 8),
        "each actual spawned AI must receive IsSummonedBy after initialization"
    );
    let (x, y, z) = crate::terrain_height::wow_to_bevy(
        action.target_position[0],
        action.target_position[1],
        action.target_position[2],
    );
    for &summon in &summons {
        assert_eq!(
            *ctx.world.get::<Position>(summon).unwrap(),
            Position { x, y, z }
        );
        assert_eq!(ctx.world.get::<Npc>(summon).unwrap().template_id, 28636);
    }
    let mut flags = tdb_action(197800, 9, 2);
    flags.target_type = TARGET_STORED;
    flags.target_params[0] = 9;
    flags.action_type = ACTION_SET_NPC_FLAG;
    flags.action_params[0] = 8;
    ai.execute_action(&mut ctx, &flags, None);
    assert!(
        summons
            .iter()
            .all(|&s| ctx.world.get::<NpcFlags>(s).unwrap().0 == 8)
    );
    lifetimes::tick_lifetimes(ctx.world, 299999);
    assert!(summons.iter().all(|&s| ctx.world.get_entity(s).is_ok()));
    lifetimes::tick_lifetimes(ctx.world, 1);
    assert!(summons.iter().all(|&s| ctx.world.get_entity(s).is_err()));
}

#[test]
fn smartai_coverage_follow_updates_relative_destination_and_cancels() {
    let (_temp, mut world, mut ai, creature, player) = erland();
    world.entity_mut(player).insert((
        Position {
            x: 10.0,
            y: 0.0,
            z: 0.0,
        },
        shared::components::Rotation {
            x: 0.0,
            y: std::f32::consts::FRAC_PI_2,
            z: 0.0,
        },
    ));
    let mut action = tdb_action(-85822, 0, 1);
    action.target_type = TARGET_ACTION_INVOKER;
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.execute_action(&mut ctx, &action, Some(player));
    ai.update_ai(&mut ctx, 50);
    let target = ctx
        .world
        .get::<super::super::movement::PointMovement>(creature)
        .expect("FOLLOW must move toward the relative target")
        .target;
    let offset = 1.1 / 2.0_f32.sqrt();
    assert!((target.x - (10.0 + offset)).abs() < 0.001);
    assert!((target.z + offset).abs() < 0.001);
    ctx.world.get_mut::<Position>(player).unwrap().x = 20.0;
    ai.update_ai(&mut ctx, 50);
    assert!(
        (ctx.world
            .get::<super::super::movement::PointMovement>(creature)
            .unwrap()
            .target
            .x
            - (20.0 + offset))
            .abs()
            < 0.001
    );
    ai.execute_action(&mut ctx, &action, None);
    assert!(
        ctx.world
            .get::<super::super::movement::PointMovement>(creature)
            .is_none()
    );
}

#[test]
fn smartai_coverage_nearest_creature_filters_entry_map_alive_and_distance() {
    let (_temp, mut world, mut ai, creature, _player) = erland();
    world
        .entity_mut(creature)
        .insert(crate::maps::InMap::continent(0));
    let mut candidates = Vec::new();
    for (entry, x, map, health) in [
        (1950, 5.0, 0, 100.0),
        (1950, 10.0, 0, 100.0),
        (1950, 1.0, 1, 100.0),
        (1950, 2.0, 0, 0.0),
        (1951, 1.0, 0, 100.0),
    ] {
        candidates.push(
            world
                .spawn((
                    Npc {
                        template_id: entry,
                        name: "fixture".into(),
                    },
                    Position { x, y: 0.0, z: 0.0 },
                    crate::maps::InMap::continent(map),
                    shared::components::Health {
                        current: health,
                        max: 100.0,
                    },
                    NpcFlags(2),
                ))
                .id(),
        );
    }
    let mut action = tdb_action(197803, 9, 3);
    action.action_type = ACTION_REMOVE_NPC_FLAG;
    action.action_params[0] = 2;
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.execute_action(&mut ctx, &action, None);
    for (index, &candidate) in candidates.iter().enumerate() {
        assert_eq!(
            ctx.world.get::<NpcFlags>(candidate).unwrap().0,
            if index == 0 { 0 } else { 2 }
        );
    }
}

#[test]
fn smartai_coverage_tracking_completion_auto_rewards_without_accept_and_is_idempotent() {
    let (temp, mut world, mut ai, creature, player) = erland();
    install_quest_log(&temp, &mut world, player);
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let tracking = conn
        .query_row(
            "SELECT ID,Flags,LogTitle FROM tdb_quest_template WHERE ID=50804",
            [],
            |row| {
                Ok(crate::quest_data::QuestTemplate {
                    id: row.get(0)?,
                    flags: row.get(1)?,
                    title: row.get(2)?,
                    captured_reward_xp: Some(0),
                    ..Default::default()
                })
            },
        )
        .unwrap();
    world
        .resource_mut::<crate::quest_data::QuestDb>()
        .quests
        .insert(tracking.id, tracking);
    let mut action = tdb_action(197802, 9, 1);
    action.action_params[0] = 50804;
    action.target_type = TARGET_ACTION_INVOKER;
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.execute_action(&mut ctx, &action, Some(player));
    let db = &ctx.world.resource::<crate::persistence::GameDatabase>().db;
    let before = crate::persistence::quest_store::load_quest_log(db, 1);
    assert!(before.is_rewarded(50804));
    assert!(before.active(50804).is_none());
    assert_eq!(before.active(435).unwrap().counts, vec![0]);
    ctx.world
        .resource_mut::<crate::quest::QuestDirty>()
        .0
        .clear();
    ai.execute_action(&mut ctx, &action, Some(player));
    let db = &ctx.world.resource::<crate::persistence::GameDatabase>().db;
    assert_eq!(
        crate::persistence::quest_store::load_quest_log(db, 1),
        before
    );
    assert!(
        ctx.world
            .resource::<crate::quest::QuestDirty>()
            .0
            .is_empty()
    );
}

fn wizzle_movement_app() -> (App, Entity, Entity, Position, Position) {
    use crate::creature_ai::{
        CreatureAiPlugin, CreatureState, SpawnPoint, WanderPhase, WanderState,
    };
    use bevy::time::TimeUpdateStrategy;
    use std::time::Duration;

    let (_temp, mut world, mut ai, creature, player) = wizzle();
    let origin = Position {
        x: 1109.15,
        y: 82.4108,
        z: 3104.11,
    };
    world.entity_mut(creature).insert((
        origin,
        shared::components::Health {
            current: 417.0,
            max: 417.0,
        },
        CreatureState::Idle,
        SpawnPoint {
            x: origin.x,
            y: origin.y,
            z: origin.z,
            wander_distance: 5.0,
            speed_walk: 1.0,
            speed_run: 1.0,
        },
        WanderState {
            phase: WanderPhase::Walking,
            target_x: origin.x + 10.0,
            target_z: origin.z,
            idle_remaining: 0.0,
            seed: 42,
        },
    ));
    world.entity_mut(player).insert(origin);
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.reset(&mut ctx);
    ai.quest_accepted(&mut ctx, player, 863);
    ai.update_ai(&mut ctx, 50);
    ai.update_ai(&mut ctx, 6001);
    let destination = ctx
        .world
        .get::<super::super::movement::PointMovement>(creature)
        .unwrap()
        .target;
    ctx.world
        .entity_mut(creature)
        .insert(super::super::CreatureAiState {
            ai: Some(Box::new(ai)),
            pending: Default::default(),
        });

    let mut app = App::new();
    world.insert_resource(
        app.world_mut()
            .remove_resource::<bevy::ecs::schedule::Schedules>()
            .unwrap(),
    );
    world.insert_resource(
        app.world_mut()
            .remove_resource::<bevy::app::MainScheduleOrder>()
            .unwrap(),
    );
    world.insert_resource(
        app.world_mut()
            .remove_resource::<bevy::app::FixedMainScheduleOrder>()
            .unwrap(),
    );
    std::mem::swap(app.world_mut(), &mut world);
    app.init_resource::<bevy::ecs::reflect::AppTypeRegistry>();
    app.add_message::<bevy::app::AppExit>();
    app.add_plugins((bevy::MinimalPlugins, CreatureAiPlugin));
    app.insert_resource(crate::world_data::WorldData::default());
    app.insert_resource(TimeUpdateStrategy::ManualDuration(Duration::from_millis(
        50,
    )));
    app.insert_resource(Time::<Fixed>::from_duration(Duration::from_millis(50)));
    app.add_systems(
        FixedUpdate,
        (
            super::super::update_creature_ai,
            super::super::movement::point_movement_system,
        )
            .chain()
            .before(crate::creature_ai::chase_system),
    );
    (app, creature, player, origin, destination)
}

#[test]
fn smartai_coverage_wizzle_evade_returns_to_last_path_position_not_original_spawn() {
    let (mut app, creature, player, _, _) = wizzle_movement_app();
    for _ in 0..41 {
        app.update();
    }
    let anchor = *app.world().get::<Position>(creature).unwrap();
    super::super::dispatch(
        app.world_mut(),
        creature,
        super::super::Hook::JustEngagedWith(player),
    );
    let battle = Position {
        x: anchor.x + 10.0,
        ..anchor
    };
    app.world_mut().entity_mut(creature).insert((
        battle,
        crate::creature_ai::CreatureState::Pursuing {
            target: player.to_bits(),
        },
    ));
    crate::creature_aggro::enter_evade_mode(app.world_mut(), creature);
    app.update();
    let actual = *app.world().get::<Position>(creature).unwrap();
    let expected = Vec3::new(battle.x - 0.35, battle.y, battle.z);
    let error = Vec3::new(actual.x, actual.y, actual.z).distance(expected);
    assert!(
        error < 0.001,
        "escort returns toward its last waypoint position, not spawn: anchor={anchor:?}, actual={actual:?}, expected={expected:?}"
    );
}

#[test]
fn smartai_coverage_wizzle_combat_pauses_path_while_chase_owns_fixed_movement() {
    let (mut app, creature, player, origin, _) = wizzle_movement_app();
    app.world_mut().entity_mut(player).insert(Position {
        x: origin.x + 20.0,
        ..origin
    });
    app.world_mut()
        .entity_mut(creature)
        .insert(crate::creature_ai::CreatureState::Pursuing {
            target: player.to_bits(),
        });
    app.update();
    app.update();
    let actual = *app.world().get::<Position>(creature).unwrap();
    let expected = Vec3::new(origin.x + 0.35, origin.y, origin.z);
    let error = Vec3::new(actual.x, actual.y, actual.z).distance(expected);
    assert!(
        error < 0.001,
        "combat chase alone owns this tick; path must suspend: actual={actual:?}, expected={expected:?}, error={error}"
    );
}

#[test]
fn smartai_coverage_wizzle_point_movement_excludes_spawn_wander_in_fixed_schedule() {
    let (mut app, creature, _, origin, destination) = wizzle_movement_app();
    app.update();
    app.update();
    let actual = *app.world().get::<Position>(creature).unwrap();
    let direction = Vec3::new(
        destination.x - origin.x,
        destination.y - origin.y,
        destination.z - origin.z,
    )
    .normalize();
    let expected = Vec3::new(origin.x, origin.y, origin.z) + direction * 0.125;
    let error = Vec3::new(actual.x, actual.y, actual.z).distance(expected);
    assert!(
        error < 0.001,
        "only the authored point driver may move the cart: actual={actual:?}, expected={expected:?}, error={error}"
    );
}

fn wizzle() -> (tempfile::TempDir, World, SmartAi, Entity, Entity) {
    let (temp, mut world, mut ai, creature, player) = erland();
    let conn = Connection::open(temp.path().join("escort.sqlite")).unwrap();
    conn.execute(
        "ATTACH DATABASE ?1 AS tdb",
        [crate::test_data::world_db_path().to_str().unwrap()],
    )
    .unwrap();
    conn.execute_batch("DELETE FROM content_creature_template;
        INSERT INTO content_creature_template SELECT * FROM tdb.tdb_creature_template WHERE entry=3439;
        DELETE FROM tdb_smart_scripts;
        INSERT INTO tdb_smart_scripts SELECT * FROM tdb.tdb_smart_scripts WHERE entryorguid=3439 OR entryorguid BETWEEN 343900 AND 343903;
        DELETE FROM tdb_waypoint_path; DELETE FROM tdb_waypoint_path_node;
        INSERT INTO tdb_waypoint_path SELECT * FROM tdb.tdb_waypoint_path WHERE PathId IN (2751201,2751209,2751217);
        INSERT INTO tdb_waypoint_path_node SELECT * FROM tdb.tdb_waypoint_path_node WHERE PathId IN (2751201,2751209,2751217);").unwrap();
    let scripts = load_smart_scripts(&temp.path().join("escort.sqlite")).unwrap();
    ai.events = scripts.0[&3439].clone();
    ai.states = vec![EventState::default(); ai.events.len()];
    world.insert_resource(scripts);
    world.get_mut::<Npc>(creature).unwrap().template_id = 3439;
    (temp, world, ai, creature, player)
}

#[test]
fn smartai_coverage_wizzle_path_end_wildcard_runs_the_next_authored_leg() {
    let (temp, mut world, mut ai, creature, player) = wizzle();
    let conn = Connection::open(temp.path().join("escort.sqlite")).unwrap();
    let nodes: Vec<u32> = conn
        .prepare("SELECT NodeId FROM tdb_waypoint_path_node WHERE PathId=2751201 ORDER BY NodeId")
        .unwrap()
        .query_map([], |r| r.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let wow: (f32,f32,f32) = conn.query_row("SELECT PositionX,PositionY,PositionZ FROM tdb_waypoint_path_node WHERE PathId=2751209 AND NodeId=1", [], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
    let (x, y, z) = crate::terrain_height::wow_to_bevy(wow.0, wow.1, wow.2);
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.reset(&mut ctx);
    ai.quest_accepted(&mut ctx, player, 863);
    ai.update_ai(&mut ctx, 50);
    ai.update_ai(&mut ctx, 6001);
    for node in nodes {
        ctx.world
            .entity_mut(creature)
            .remove::<super::super::movement::PointMovement>();
        ai.movement_inform(&mut ctx, super::super::MotionType::Point, node);
    }
    ai.update_ai(&mut ctx, 3001);
    assert_eq!(
        ctx.world
            .get::<super::super::movement::PointMovement>(creature)
            .expect("UINT32_MAX waypoint event is the pinned wildcard, not point zero")
            .target,
        Position { x, y, z }
    );
}

#[test]
fn smartai_coverage_summon_dispatch_and_expiry_execute_parent_lifecycle_hooks() {
    let (_temp, mut world, mut ai, creature, player) = erland();
    let summon = world
        .spawn((
            Npc {
                template_id: 3282,
                name: "Venture Co. Mercenary".into(),
            },
            NpcFlags(0),
            crate::zones::TempSummon { summoner: creature },
        ))
        .id();
    let mut created = tdb_action(197800, 9, 2);
    created.event_type = 17;
    created.event_params = [3282, 10, 10, 0, 0];
    created.target_type = TARGET_ACTION_INVOKER;
    created.action_type = ACTION_SET_NPC_FLAG;
    created.action_params[0] = 8;
    created.link = 0;
    let mut removed = created.clone();
    removed.id += 1;
    removed.event_type = 35;
    removed.event_params = [3282, 0, 0, 0, 0];
    removed.target_type = TARGET_SELF;
    removed.action_params[0] = 4;
    ai.events = Arc::new(vec![created, removed]);
    ai.states = vec![EventState::default(); 2];
    ai.reset(&mut Ctx {
        world: &mut world,
        me: creature,
    });
    world
        .entity_mut(creature)
        .insert(super::super::CreatureAiState {
            ai: Some(Box::new(ai)),
            pending: Default::default(),
        });
    super::super::dispatch(
        &mut world,
        creature,
        super::super::Hook::JustSummoned(player),
    );
    assert_eq!(
        world.get::<NpcFlags>(creature).unwrap().0,
        2,
        "noncreature invokers must not match summoned-unit rows"
    );
    super::super::dispatch(
        &mut world,
        creature,
        super::super::Hook::JustSummoned(summon),
    );
    assert_eq!(
        world.get::<NpcFlags>(summon).unwrap().0,
        8,
        "the parent hook must receive the actual summon as invoker"
    );
    world.get_mut::<NpcFlags>(summon).unwrap().0 = 0;
    super::super::dispatch(
        &mut world,
        creature,
        super::super::Hook::JustSummoned(summon),
    );
    assert_eq!(
        world.get::<NpcFlags>(summon).unwrap().0,
        0,
        "cooldown blocks a repeated hook"
    );
    super::super::dispatch(&mut world, creature, super::super::Hook::UpdateAi(9));
    super::super::dispatch(
        &mut world,
        creature,
        super::super::Hook::JustSummoned(summon),
    );
    assert_eq!(world.get::<NpcFlags>(summon).unwrap().0, 0);
    super::super::dispatch(&mut world, creature, super::super::Hook::UpdateAi(2));
    super::super::dispatch(
        &mut world,
        creature,
        super::super::Hook::JustSummoned(summon),
    );
    assert_eq!(
        world.get::<NpcFlags>(summon).unwrap().0,
        8,
        "elapsed cooldown admits the next actual hook without firing it by itself"
    );
    lifetimes::schedule_despawn(&mut world, summon, 10, 0, false);
    lifetimes::tick_lifetimes(&mut world, 9);
    assert_eq!(world.get::<NpcFlags>(creature).unwrap().0, 2);
    lifetimes::tick_lifetimes(&mut world, 1);
    assert!(world.get_entity(summon).is_err());
    assert_eq!(
        world.get::<NpcFlags>(creature).unwrap().0,
        4,
        "despawn must dispatch while the summons entry is still readable"
    );
}

#[test]
fn smartai_coverage_wizzle_authored_summon_group_spawns_stores_and_expires_ambushers() {
    let (temp, mut world, mut ai, creature, player) = erland();
    let conn = Connection::open(temp.path().join("escort.sqlite")).unwrap();
    conn.execute_batch("CREATE TABLE tdb_creature_summon_groups(summonerId INTEGER,summonerType INTEGER,groupId INTEGER,entry INTEGER,position_x REAL,position_y REAL,position_z REAL,orientation REAL,summonType INTEGER,summonTime INTEGER,Comment TEXT);
        INSERT INTO tdb_creature_summon_groups VALUES (3439,0,0,3282,1075.12,-2983.98,92.9072,5.81195,4,60000,'Mercenary'),(3439,0,0,3282,1119.69,-2970.87,97.1581,4.01426,4,60000,'Mercenary');").unwrap();
    world.insert_resource(load_smart_scripts(&temp.path().join("escort.sqlite")).unwrap());
    let source = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let expected = crate::world_data::load_expected_stats(&source);
    let mut templates = crate::world_data::load_templates(&source, &expected);
    templates.retain(|entry, _| *entry == 3282);
    crate::world_data::load_template_models(&source, &mut templates);
    let mut query = source.prepare("SELECT level,class,basehp0,basehp1,basehp2 FROM creature_classlevelstats WHERE class=1").unwrap();
    let class_level_base_hp = query
        .query_map([], |r| {
            Ok(((r.get(0)?, r.get(1)?), [r.get(2)?, r.get(3)?, r.get(4)?]))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    world.insert_resource(crate::world_data::WorldData {
        templates,
        class_level_base_hp,
        ..Default::default()
    });
    world
        .entity_mut(creature)
        .insert(crate::maps::InMap::continent(1));
    world.get_mut::<Npc>(creature).unwrap().template_id = 3439;
    let mut action = tdb_action(343902, 9, 0);
    action.action_params[2] = 9;
    let mut ctx = Ctx {
        world: &mut world,
        me: creature,
    };
    ai.execute_action(&mut ctx, &action, Some(player));
    let summons: Vec<_> = ctx
        .world
        .query_filtered::<Entity, With<crate::zones::TempSummon>>()
        .iter(ctx.world)
        .collect();
    assert_eq!(summons.len(), 2);
    let mut positions: Vec<_> = summons
        .iter()
        .map(|&entity| {
            assert_eq!(ctx.world.get::<Npc>(entity).unwrap().template_id, 3282);
            let position = ctx.world.get::<Position>(entity).unwrap();
            (position.x, position.y, position.z)
        })
        .collect();
    positions.sort_by(|a, b| a.0.total_cmp(&b.0));
    assert_eq!(
        positions,
        vec![(1075.12, 92.9072, 2983.98), (1119.69, 97.1581, 2970.87)]
    );
    let mut flags = tdb_action(197800, 9, 2);
    flags.target_type = TARGET_STORED;
    flags.target_params[0] = 9;
    flags.action_type = ACTION_SET_NPC_FLAG;
    flags.action_params[0] = 8;
    ai.execute_action(&mut ctx, &flags, None);
    assert!(
        summons
            .iter()
            .all(|&s| ctx.world.get::<NpcFlags>(s).unwrap().0 == 8)
    );
    lifetimes::tick_lifetimes(ctx.world, 60000);
    assert!(summons.iter().all(|&s| ctx.world.get_entity(s).is_err()));
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

/// Actual Serpentis death action34 carries TC data3, stateDONE3, mode1.
/// Source data3 is encounter590; with legacy Cobrahn0 plus the Serpentis binding,
/// that encounter lives at runtime slot1. Test the action directly so generic
/// SmartAI BossAi::just_died cannot mask an ignored or misdirected source row.
#[test]
fn smartai_coverage_wailing_source_ordinal_updates_only_its_encounter() {
    use super::super::instance_script::{EncounterState, get_boss_state};
    use crate::maps::InMap;

    let path = crate::test_data::world_db_path();
    let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let mut maps = crate::maps::store::load_map_store(&conn).unwrap();
    maps.raid_creature_encounters
        .retain(|(map, _), _| *map != 43);
    maps.raid_creature_encounters.insert((43, 3673), 5);
    let scripts = load_smart_scripts(&path).unwrap();
    let events = scripts.0[&3673].clone();
    let action = events.iter().find(|event| event.id == 0).unwrap().clone();
    let mut ai = SmartAi {
        states: vec![EventState::default(); events.len()],
        events,
        phase: 0,
        boss: None,
        stored_targets: HashMap::new(),
        counters: HashMap::new(),
        timed_list: None,
        processing_timed_list: false,
        escort: None,
        path_event_id: None,
        follow: None,
    };
    let mut world = World::new();
    world.insert_resource(maps);
    world.insert_resource(Time::<()>::default());
    world.insert_resource(crate::maps::ServerClock {
        offset_secs: 0,
        fixed_unix: Some(1_791_705_600),
    });
    let map = InMap {
        map_id: 43,
        instance_id: 7,
    };
    let other = InMap {
        instance_id: 8,
        ..map
    };
    let actor = world
        .spawn((
            Npc {
                template_id: 3673,
                name: "Lord Serpentis".into(),
            },
            map,
        ))
        .id();
    assert_eq!(
        get_boss_state(&mut world, map, 1),
        Some(EncounterState::NotStarted)
    );
    ai.execute_action(
        &mut Ctx {
            world: &mut world,
            me: actor,
        },
        &action,
        None,
    );
    assert_eq!(
        get_boss_state(&mut world, map, 1),
        Some(EncounterState::Done)
    );
    assert_eq!(
        get_boss_state(&mut world, map, 0),
        Some(EncounterState::NotStarted)
    );
    assert_eq!(
        get_boss_state(&mut world, other, 1),
        Some(EncounterState::NotStarted)
    );

    // Actual Anacondra row5 is SetData mode0, not SetBossState. Its field3 must
    // not be silently reinterpreted as a Serpentis completion in another copy.
    let lady = world
        .spawn((
            Npc {
                template_id: 3671,
                name: "Lady Anacondra".into(),
            },
            other,
        ))
        .id();
    ai.execute_action(
        &mut Ctx {
            world: &mut world,
            me: lady,
        },
        &tdb_action(3671, 0, 5),
        None,
    );
    assert_eq!(
        get_boss_state(&mut world, other, 1),
        Some(EncounterState::NotStarted)
    );

    // A source ordinal alone must not authorize an unbound encounter.
    world
        .resource_mut::<crate::maps::MapStore>()
        .raid_creature_encounters
        .remove(&(43, 3673));
    let unbound = InMap {
        instance_id: 9,
        ..map
    };
    world.entity_mut(actor).insert(unbound);
    ai.execute_action(
        &mut Ctx {
            world: &mut world,
            me: actor,
        },
        &action,
        None,
    );
    assert_eq!(
        get_boss_state(&mut world, unbound, 0),
        Some(EncounterState::NotStarted)
    );
    assert_eq!(get_boss_state(&mut world, unbound, 1), None);
}
