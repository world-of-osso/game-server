use super::*;
use shared::battleground::{BgOutcome, BgPhase, BgTeam, BgTemplate, node_rules};
use shared::protocol::BattlegroundSnapshot;

struct EnteredMatch {
    h: Harness,
    players: Vec<Traveler>,
    teams: Vec<BgTeam>,
    match_id: u64,
    copy: InMap,
    origin: Vec3,
}

#[test]
fn deephaul_channel_real_carts_crystal_drop_capture_victory_and_origin_return() {
    let template = shared::battleground::deephaul_ravine();
    let mut entered = enter_source_match(&template, shared::battleground::DEEPHAUL_STARTS);
    let alliance: Vec<_> = entered
        .players
        .iter()
        .zip(&entered.teams)
        .filter(|(_, team)| **team == BgTeam::A)
        .map(|(who, _)| who)
        .collect();
    let (h, copy) = (&mut entered.h, entered.copy);
    use_second_frames(h);
    assert_eq!(deephaul_carts_in(h, copy).len(), 2);
    assert!(objects_in(h, copy, shared::battleground::DEEPHAUL_FLAG_ENTRY).is_empty());
    h.advance(15);
    let actor = alliance[0];
    let (flag, home) = objects_in(h, copy, shared::battleground::DEEPHAUL_FLAG_ENTRY)[0];
    h.walk_to(actor, home);
    h.app.world_mut().entity_mut(flag).insert(InMap {
        instance_id: copy.instance_id + 1,
        ..copy
    });
    use_object(h, actor, flag);
    assert!(battleground(h, actor).world_states.contains(&(1545, 1)));
    h.app.world_mut().entity_mut(flag).insert(copy);
    use_object(h, actor, flag);
    assert!(battleground(h, actor).world_states.contains(&(1545, 2)));
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(actor.player)
        .unwrap()
        .current = 0.0;
    h.advance(1);
    let dropped = objects_in(h, copy, shared::battleground::DEEPHAUL_DROP_ENTRY);
    assert_eq!(dropped.len(), 1);
    assert!(close(dropped[0].1, home));
    assert_eq!(
        h.app
            .world()
            .get::<GameObjectInfo>(dropped[0].0)
            .unwrap()
            .go_type,
        37
    );
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(actor.player)
        .unwrap()
        .current = 100.0;
    let teammate = alliance[1];
    h.walk_to(teammate, dropped[0].1);
    use_object(h, teammate, dropped[0].0);
    assert!(battleground(h, teammate).world_states.contains(&(1545, 2)));
    assert!(objects_in(h, copy, shared::battleground::DEEPHAUL_DROP_ENTRY).is_empty());
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(teammate.player)
        .unwrap()
        .current = 0.0;
    h.advance(1);
    assert_eq!(
        objects_in(h, copy, shared::battleground::DEEPHAUL_DROP_ENTRY).len(),
        1
    );
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(teammate.player)
        .unwrap()
        .current = 100.0;
    h.advance(10);
    assert!(objects_in(h, copy, shared::battleground::DEEPHAUL_DROP_ENTRY).is_empty());
    let (flag, home) = objects_in(h, copy, shared::battleground::DEEPHAUL_FLAG_ENTRY)[0];
    h.walk_to(actor, home);
    use_object(h, actor, flag);
    h.walk_to(
        actor,
        Vec3::from_array(shared::battleground::DEEPHAUL_CAPTURE_TRIGGERS[0]),
    );
    h.advance(1);
    assert!(
        battleground(h, actor)
            .players
            .iter()
            .find(|row| row.character_id == 1)
            .unwrap()
            .objective_scores
            .contains(&(1020, 1))
    );
    h.advance(30);
    assert_eq!(
        objects_in(h, copy, shared::battleground::DEEPHAUL_FLAG_ENTRY).len(),
        1
    );
    let mut delivered = false;
    let mut respawned = false;
    for _ in 0..2000 {
        let carts = deephaul_carts_in(h, copy);
        for (index, who) in alliance.iter().enumerate() {
            if let Some((_, at)) = carts.get(index % 2) {
                h.app.world_mut().entity_mut(who.player).insert(bevy(*at));
            }
        }
        h.advance(1);
        let count = deephaul_carts_in(h, copy).len();
        delivered |= count == 0;
        respawned |= delivered && count == 2;
        if battleground(h, actor).phase == BgPhase::Ended {
            break;
        }
    }
    assert!(
        delivered && respawned,
        "source paired delivery and two-second replacement"
    );
    assert!(battleground(h, actor).players.iter().any(|row| {
        row.team == BgTeam::A
            && row
                .objective_scores
                .iter()
                .any(|(stat, value)| *stat == 1021 && *value > 0)
    }));
    win_leave_and_return(&mut entered, 1500);
}

fn deephaul_carts_in(h: &mut Harness, copy: InMap) -> Vec<(Entity, Vec3)> {
    let world = h.app.world_mut();
    let mut query = world.query::<(Entity, &shared::components::Npc, &InMap, &Position)>();
    query
        .iter(world)
        .filter(|(_, npc, map, _)| [214690, 217346].contains(&npc.template_id) && **map == copy)
        .map(|(entity, _, _, at)| (entity, wow(at)))
        .collect()
}

#[test]
fn seething_channel_grounded_fixture_gathering_cancellation_victory_and_return() {
    let template = shared::battleground::seething_shore();
    let mut entered = enter_source_match(&template, shared::battleground::SEETHING_STARTS);
    let actor = &entered.players[0];
    let h = &mut entered.h;
    use_second_frames(h);
    h.advance(42);
    let nodes = seething_objects(h, entered.copy);
    assert_eq!(nodes.len(), 3);
    let (object, at) = nodes[0];
    h.walk_to(actor, at);
    h.app.world_mut().entity_mut(object).insert(InMap {
        instance_id: entered.copy.instance_id + 1,
        ..entered.copy
    });
    use_object(h, actor, object);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_none()
    );
    assert_eq!(battleground(h, actor).score_a, 0);
    h.app.world_mut().entity_mut(object).insert(entered.copy);
    use_object(h, actor, object);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_some()
    );
    assert_eq!(battleground(h, actor).score_a, 0);
    h.app
        .world_mut()
        .entity_mut(actor.player)
        .insert(crate::networking::Moving(true));
    h.advance(1);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_none()
    );
    h.app
        .world_mut()
        .entity_mut(actor.player)
        .insert(crate::networking::Moving(false));
    h.advance(6);
    assert_eq!(battleground(h, actor).score_a, 0);
    use_object(h, actor, object);
    h.advance(6);
    assert_eq!(battleground(h, actor).score_a, 100);
    use_object(h, actor, object);
    h.advance(6);
    assert_eq!(battleground(h, actor).score_a, 100);
    for _ in 0..20 {
        h.advance(42);
        for (object, at) in seething_objects(h, entered.copy) {
            if battleground(h, actor).phase != BgPhase::InProgress {
                break;
            }
            h.walk_to(actor, at);
            use_object(h, actor, object);
            h.advance(6);
        }
        if battleground(h, actor).phase == BgPhase::Ended {
            break;
        }
    }
    let result = battleground(h, actor);
    let own = result
        .players
        .iter()
        .find(|row| row.character_id == 1)
        .unwrap();
    assert!(own.objective_scores.contains(&(744, 15)));
    win_leave_and_return(&mut entered, 1500);
}

#[test]
fn seething_channel_refuses_unimplemented_transport_relative_entry() {
    let template = shared::battleground::seething_shore();
    let file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(file.path()).unwrap();
    crate::persistence::create_tables(&db);
    let mut h = harness_with(|app| {
        app.insert_resource(crate::persistence::GameDatabase { db });
        app.insert_resource(crate::faction::FactionTemplates::default());
        app.insert_resource(crate::game_objects::GameObjectSpawns::default());
        app.add_plugins(crate::pvp::PvpPlugin);
    })
    .unwrap();
    let players: Vec<_> = (1..=10)
        .map(|id| h.spawn(id, STORMWIND, STOCKADE_ENTRANCE_TRIGGER))
        .collect();
    for who in &players {
        h.app.world_mut().entity_mut(who.player).insert((
            crate::pvp::PlayerPvpState::default(),
            shared::components::Level(40),
        ));
        h.app.world_mut().entity_mut(who.connection).insert((
            pair::<QueueForBattleground>(),
            pair::<RespondToBattlegroundInvite>(),
            pair::<PvpStateUpdate>(),
        ));
        h.app
            .world_mut()
            .get_mut::<MessageSender<QueueForBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(QueueForBattleground {
                battleground_id: template.id,
            });
    }
    h.receive_pvp_updates(5);
    let who = &players[0];
    let match_id = battleground(&h, who).match_id;
    h.app
        .world_mut()
        .get_mut::<MessageSender<RespondToBattlegroundInvite>>(who.connection)
        .unwrap()
        .send::<PvpChannel>(RespondToBattlegroundInvite {
            match_id,
            accepted: true,
        });
    let updates = h.receive_pvp_updates(5);
    assert!(
        updates[&who.connection].iter().any(|update| update
            .error
            .as_deref()
            .is_some_and(|error| error.contains("transport 35"))),
        "{:#?}",
        updates[&who.connection]
    );
    assert!(h.new_worlds(who).is_empty());
    assert_eq!(h.map_of(who), Some(STORMWIND));
}

fn seething_objects(h: &mut Harness, copy: InMap) -> Vec<(Entity, Vec3)> {
    shared::battleground::SEETHING_OBJECTS
        .into_iter()
        .flat_map(|entry| objects_in(h, copy, entry))
        .collect()
}

/// Source-minimum roster queue, invitation, acceptance and safe-location entry over the channel.
fn enter_source_match(template: &BgTemplate, starts: [u32; 2]) -> EnteredMatch {
    let objects = if template.map_id == 998 {
        source_kotmogu_objects()
    } else if matches!(template.map_id, 727 | 1803) {
        crate::game_objects::GameObjectSpawns::default()
    } else {
        source_battleground_objects(template.map_id)
    };
    let database_file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(database_file.path()).unwrap();
    crate::persistence::create_tables(&db);
    let mut h = harness_with(|app| {
        app.insert_resource(crate::persistence::GameDatabase { db });
        app.insert_resource(objects);
        if template.map_id == 727 {
            let (world_data, spellclick) = silvershard_source_world();
            app.insert_resource(world_data);
            app.insert_resource(spellclick);
            let data = crate::spell_cast::test_harness::world_db().unwrap();
            app.insert_resource(crate::spell_info::SpellStore::fixture(
                [124491, 124492, 97372]
                    .into_iter()
                    .map(|id| data.store.get(id).unwrap().clone())
                    .collect(),
            ));
            app.insert_resource(crate::spell_cast::test_harness::power_table(data));
            app.add_plugins((crate::spell_cast::SpellCastPlugin, crate::auras::AuraPlugin));
        }
        if template.map_id == 1803 {
            let (world_data, templates) = seething_source_world();
            // Partial objective proof only: the parent's approved grounded fixture
            // starts at an actual authored fissure, not a fabricated airship transform.
            // The separate refusal test retains genuine TransportSpawnId35/36.
            let fissure = world_data
                .spawns
                .iter()
                .find(|spawn| spawn.map == 1803 && spawn.template_entry == 125253)
                .unwrap();
            let ground_start =
                Vec3::new(fissure.position_x, fissure.position_y, fissure.position_z);
            {
                let mut store = app.world_mut().resource_mut::<MapStore>();
                assert_eq!(store.safe_loc_transports.get(&6337), Some(&35));
                assert_eq!(store.safe_loc_transports.get(&6380), Some(&36));
                for start in starts {
                    store.safe_loc_transports.remove(&start);
                    store.safe_locs.get_mut(&start).unwrap().position = ground_start;
                }
            }
            app.insert_resource(world_data);
            app.insert_resource(templates);
            app.insert_resource(crate::ground::GroundMap::new(
                &std::env::var_os("GAME_SERVER_GROUND_DIR")
                    .map(std::path::PathBuf::from)
                    .expect("source Seething terrain root"),
                std::collections::HashMap::from([(1803, 1686724)]),
            ));
            let data = crate::spell_cast::test_harness::world_db().unwrap();
            app.insert_resource(crate::spell_info::SpellStore::fixture(vec![
                data.store.get(261857).unwrap().clone(),
            ]));
            app.insert_resource(crate::spell_cast::test_harness::power_table(data));
            app.add_plugins((crate::spell_cast::SpellCastPlugin, crate::auras::AuraPlugin));
        }
        if template.map_id == 2656 {
            let (world_data, templates) = deephaul_source_world();
            app.insert_resource(world_data);
            app.insert_resource(templates);
        }
        app.insert_resource(crate::faction::FactionTemplates::default());
        app.add_plugins((
            crate::pvp::PvpPlugin,
            crate::npc_interaction::NpcInteractionPlugin,
        ));
    })
    .unwrap();
    let origin = STOCKADE_ENTRANCE_TRIGGER + Vec3::new(0.0, 0.0, 20.0);
    let count = u64::from(template.min_players_per_team) * 2;
    let players: Vec<_> = (1..=count)
        .map(|id| h.spawn(id, STORMWIND, origin))
        .collect();
    for who in &players {
        h.app.world_mut().entity_mut(who.player).insert((
            crate::pvp::PlayerPvpState::default(),
            shared::components::Level(template.bracket.min_level),
            shared::components::UnitFactionTemplate(crate::faction::faction_template_for_race(1)),
        ));
        h.app.world_mut().entity_mut(who.connection).insert((
            pair::<QueueForBattleground>(),
            pair::<RespondToBattlegroundInvite>(),
            pair::<LeaveBattleground>(),
            pair::<UseGameObject>(),
            pair::<shared::protocol::InteractNpc>(),
            pair::<shared::protocol::InteractionFailed>(),
            pair::<PvpStateUpdate>(),
        ));
        h.app
            .world_mut()
            .get_mut::<MessageSender<QueueForBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(QueueForBattleground {
                battleground_id: template.id,
            });
    }
    h.receive_pvp_updates(5);
    let invitations: Vec<_> = players
        .iter()
        .map(|who| battleground(&h, who).clone())
        .collect();
    let match_id = invitations[0].match_id;
    for (who, invitation) in players.iter().zip(&invitations) {
        assert_eq!(invitation.match_id, match_id);
        assert_eq!(invitation.map_id, template.map_id);
        h.app
            .world_mut()
            .get_mut::<MessageSender<RespondToBattlegroundInvite>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(RespondToBattlegroundInvite {
                match_id,
                accepted: true,
            });
    }
    let updates = h.receive_pvp_updates(5);
    for (who, invitation) in players.iter().zip(&invitations) {
        assert!(
            updates[&who.connection]
                .iter()
                .all(|update| update.error.is_none()),
            "{:?}",
            updates[&who.connection]
        );
        let destinations = h.new_worlds(who);
        assert_eq!(destinations.len(), 1);
        let start = starts[if invitation.team == BgTeam::A { 0 } else { 1 }];
        let source = h.app.world().resource::<MapStore>().safe_locs[&start];
        assert_eq!(destinations[0].map_id, source.map_id);
        let position = destinations[0].position;
        assert!(close(
            wow(&Position {
                x: position[0],
                y: position[1],
                z: position[2]
            }),
            source.position
        ));
        h.ack(who);
    }
    h.receive_pvp_updates(5);
    let copy = h.map_of(&players[0]).unwrap();
    assert!(players.iter().all(|who| h.map_of(who) == Some(copy)));
    EnteredMatch {
        h,
        teams: invitations
            .iter()
            .map(|invitation| invitation.team)
            .collect(),
        players,
        match_id,
        copy,
        origin,
    }
}

fn battleground<'a>(h: &'a Harness, who: &Traveler) -> &'a BattlegroundSnapshot {
    h.app
        .world()
        .get::<crate::pvp::PlayerPvpState>(who.player)
        .unwrap()
        .battleground
        .as_ref()
        .expect("authoritative battleground state")
}

/// One-second frames so source heartbeats, ticks and capture clocks run on whole seconds.
fn use_second_frames(h: &mut Harness) {
    h.app
        .world_mut()
        .resource_mut::<Time<Virtual>>()
        .set_max_delta(Duration::from_secs(1));
    h.app
        .world_mut()
        .resource_mut::<Time<Fixed>>()
        .set_timestep(Duration::from_secs(1));
    h.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
            Duration::from_secs(1),
        ));
}

fn objects_in(h: &mut Harness, copy: InMap, entry: u32) -> Vec<(Entity, Vec3)> {
    let world = h.app.world_mut();
    let mut query = world.query::<(Entity, &GameObjectInfo, &InMap, &Position)>();
    query
        .iter(world)
        .filter(|(_, info, map, _)| info.entry == entry && **map == copy)
        .map(|(entity, _, _, position)| (entity, wow(position)))
        .collect()
}

fn use_object(h: &mut Harness, who: &Traveler, object: Entity) {
    h.app
        .world_mut()
        .get_mut::<MessageSender<UseGameObject>>(who.connection)
        .unwrap()
        .send::<InteractionChannel>(UseGameObject {
            object: object.to_bits(),
        });
    h.receive_pvp_updates(5);
}

/// Every player ends on Team A's victory, leaves, returns to its origin and the copy unloads.
fn win_leave_and_return(entered: &mut EnteredMatch, score: u32) {
    let EnteredMatch {
        h,
        players,
        match_id,
        copy,
        origin,
        ..
    } = entered;
    for who in players.iter() {
        let result = battleground(h, who);
        assert_eq!(result.phase, BgPhase::Ended);
        assert_eq!(result.outcome, Some(BgOutcome::Winner(BgTeam::A)));
        assert_eq!(result.score_a, score);
        assert_eq!(result.honor_reward, None);
        assert_eq!(result.players.len(), players.len());
        let score_state = match copy.map_id {
            998 => 6303,
            727 => 6437,
            1803 => 13845,
            2656 => 24958,
            _ => 1776,
        };
        assert!(result.world_states.contains(&(score_state, score as i32)));
        h.app
            .world_mut()
            .get_mut::<MessageSender<LeaveBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(LeaveBattleground {
                match_id: *match_id,
            });
    }
    h.receive_pvp_updates(5);
    for who in players.iter() {
        let returns = h.new_worlds(who);
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].map_id, 0);
        h.ack(who);
        assert_eq!(h.map_of(who), Some(STORMWIND));
        assert!(close(h.position(who), *origin));
        assert!(
            h.app
                .world()
                .get::<crate::pvp::PlayerPvpState>(who.player)
                .unwrap()
                .battleground
                .is_none()
        );
    }
    assert!(
        h.app
            .world()
            .resource::<InstanceManager>()
            .copy_difficulty(*copy)
            .is_none()
    );
}

fn node_match(template: BgTemplate) {
    let rules = node_rules(template.map_id).unwrap();
    let source_points = source_battleground_objects(template.map_id);
    assert_eq!(source_points.0.len(), rules.nodes.len());
    let mut entered = enter_source_match(&template, rules.starts);
    let (h, copy) = (&mut entered.h, entered.copy);
    let actor = &entered.players[0];
    for point in source_points.0 {
        let (object, _) = objects_in(h, copy, point.info.entry)[0];
        h.walk_to(actor, point.position);
        use_object(h, actor, object);
    }
    let before = battleground(h, actor);
    assert_eq!(before.phase, BgPhase::InProgress);
    assert_eq!(before.score_a, 0, "assaults are not instant captures");
    assert!(before.world_states.contains(&(rules.nodes[0].poi, 3)));
    let score = before
        .players
        .iter()
        .find(|row| row.character_id == 1)
        .unwrap();
    assert!(
        score
            .objective_scores
            .contains(&(rules.assault_stat, rules.nodes.len() as u32))
    );
    assert!(score.objective_scores.contains(&(rules.defense_stat, 0)));
    use_second_frames(h);
    h.advance(120);
    win_leave_and_return(&mut entered, 1500);
}

#[test]
fn arathi_channel_objectives_scoreboard_victory_and_origin_return() {
    node_match(shared::battleground::arathi_basin());
}
#[test]
fn gilneas_channel_objectives_scoreboard_victory_and_origin_return() {
    node_match(shared::battleground::battle_for_gilneas());
}

fn source_kotmogu_objects() -> crate::game_objects::GameObjectSpawns {
    let conn = Connection::open_with_flags(
        crate::test_data::world_db_path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let objects = shared::battleground::KOTMOGU_ORBS.iter().map(|orb| {
        conn.query_row(
            "SELECT t.type,t.displayId,t.name,t.size,d.GeoBox_0,d.GeoBox_1,d.GeoBox_2,d.GeoBox_3,d.GeoBox_4,d.GeoBox_5
             FROM tdb_gameobject_template t JOIN game_object_display_info d ON d.ID=t.displayId WHERE t.entry=?",
            [orb.entry], |row| {
                let scale = row.get::<_, f64>(3)? as f32;
                let number = |index| row.get::<_, f64>(index).map(|value| value as f32);
                Ok(crate::game_objects::GameObjectSpawn {
                    guid: 90_000_000 + orb.entry,
                    map_id: 998,
                    info: GameObjectInfo { entry: orb.entry, go_type: row.get(0)?, display_id: row.get(1)?, name: row.get(2)?, scale },
                    position: Vec3::from_array(orb.position), orientation: 0.0,
                    bounds: crate::game_objects::GameObjectBounds {
                        min: Vec3::new(number(4)?,number(5)?,number(6)?),
                        max: Vec3::new(number(7)?,number(8)?,number(9)?),
                        scale, orientation: 0.0,
                    },
                    spawn_difficulties: vec![0], chair: None, quest_object: None,
                    script_name: String::new(),
                })
            },
        ).unwrap()
    }).collect();
    crate::game_objects::GameObjectSpawns(objects)
}

#[test]
fn kotmogu_channel_orbs_region_ticks_death_respawn_victory_and_origin_return() {
    let template = shared::battleground::temple_of_kotmogu();
    let mut entered = enter_source_match(&template, shared::battleground::KOTMOGU_STARTS);
    let actor = &entered.players[0];
    let (h, copy) = (&mut entered.h, entered.copy);
    use_second_frames(h);
    let orb = &shared::battleground::KOTMOGU_ORBS[0];
    let (object, home) = objects_in(h, copy, orb.entry)[0];
    h.walk_to(actor, home);
    use_object(h, actor, object);
    let taken = battleground(h, actor);
    assert!(taken.world_states.contains(&(6964, 1)));
    assert!(objects_in(h, copy, orb.entry).is_empty());
    let row = battleground(h, actor)
        .players
        .iter()
        .find(|row| row.character_id == 1)
        .unwrap();
    assert_eq!(
        row.objective_scores.iter().find(|(stat, _)| *stat == 418),
        Some(&(418, 1))
    );
    h.walk_to(actor, Vec3::new(1783.41, 1333.56, 6.0));
    h.advance(5);
    let result = battleground(h, actor);
    assert!(result.score_a >= 6);
    assert!(result.world_states.contains(&(6303, result.score_a as i32)));
    let row = result
        .players
        .iter()
        .find(|row| row.character_id == 1)
        .unwrap();
    assert_eq!(
        row.objective_scores
            .iter()
            .find(|(stat, _)| *stat == 419)
            .unwrap()
            .1,
        result.score_a
    );
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(actor.player)
        .unwrap()
        .current = 0.0;
    h.advance(1);
    let returned = objects_in(h, copy, orb.entry);
    assert_eq!(returned.len(), 1);
    assert!(close(returned[0].1, home));
    assert!(battleground(h, actor).world_states.contains(&(6717, 1)));
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(actor.player)
        .unwrap()
        .current = 100.0;
    for orb in &shared::battleground::KOTMOGU_ORBS {
        let (object, home) = objects_in(h, copy, orb.entry)[0];
        h.walk_to(actor, home);
        use_object(h, actor, object);
        // Only one orb: unavailable second pickup leaves the second orb at home.
        if orb.entry != 212094 {
            assert_eq!(objects_in(h, copy, orb.entry).len(), 1);
        }
    }
    h.walk_to(actor, Vec3::new(1783.41, 1333.56, 6.0));
    h.advance(1300);
    win_leave_and_return(&mut entered, 1500);
}

fn deephaul_source_world() -> (
    crate::world_data::WorldData,
    crate::pvp::BattlegroundObjectTemplates,
) {
    let copy = tempfile::NamedTempFile::new().unwrap();
    std::fs::copy(crate::test_data::world_db_path(), copy.path()).unwrap();
    for (script, maps) in [
        ("../../scripts/promote_battlegrounds.py", Some("2656")),
        ("../../scripts/add_battleground_summon_templates.py", None),
    ] {
        let mut command = std::process::Command::new("python3");
        command.arg(script);
        if let Some(maps) = maps {
            command.args(["--maps", maps]);
        }
        let output = command.arg("--db").arg(copy.path()).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let conn = Connection::open_with_flags(copy.path(), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap();
    let templates = crate::pvp::load_battleground_objects(&conn).unwrap();
    (crate::world_data::load_world_data(copy.path()), templates)
}

fn seething_source_world() -> (
    crate::world_data::WorldData,
    crate::pvp::BattlegroundObjectTemplates,
) {
    let copy = tempfile::NamedTempFile::new().unwrap();
    std::fs::copy(crate::test_data::world_db_path(), copy.path()).unwrap();
    let promoted = std::process::Command::new("python3")
        .args([
            "../../scripts/promote_battlegrounds.py",
            "--maps",
            "1803",
            "--db",
        ])
        .arg(copy.path())
        .output()
        .unwrap();
    assert!(
        promoted.status.success(),
        "{}",
        String::from_utf8_lossy(&promoted.stderr)
    );
    let conn = Connection::open_with_flags(copy.path(), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
        .unwrap();
    let templates = crate::pvp::load_battleground_objects(&conn).unwrap();
    (crate::world_data::load_world_data(copy.path()), templates)
}

fn silvershard_source_world() -> (
    crate::world_data::WorldData,
    crate::npc_interaction::SpellClickCatalog,
) {
    let copy = tempfile::NamedTempFile::new().unwrap();
    std::fs::copy(crate::test_data::world_db_path(), copy.path()).unwrap();
    let conn = Connection::open(copy.path()).unwrap();
    conn.execute_batch("CREATE TABLE IF NOT EXISTS tdb_npc_spellclick_spells (npc_entry INTEGER,spell_id INTEGER,cast_flags INTEGER,user_type INTEGER,PRIMARY KEY(npc_entry,spell_id));
        INSERT OR REPLACE INTO tdb_npc_spellclick_spells VALUES (60283,124491,1,0);").unwrap();
    drop(conn);
    let promoted = std::process::Command::new("python3")
        .args([
            "../../scripts/promote_battlegrounds.py",
            "--maps",
            "727",
            "--db",
        ])
        .arg(copy.path())
        .output()
        .unwrap();
    assert!(
        promoted.status.success(),
        "{}",
        String::from_utf8_lossy(&promoted.stderr)
    );
    let output = std::process::Command::new("python3")
        .args(["../../scripts/add_battleground_summon_templates.py", "--db"])
        .arg(copy.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let conn = Connection::open(copy.path()).unwrap();
    let catalog = crate::npc_interaction::load_spellclick_catalog(&conn).unwrap();
    (crate::world_data::load_world_data(copy.path()), catalog)
}

#[test]
fn silvershard_channel_spellclick_waits_three_seconds_then_switches_only_its_copy() {
    let template = shared::battleground::silvershard_mines();
    let mut entered = enter_source_match(&template, shared::battleground::SILVERSHARD_STARTS);
    let actor = &entered.players[0];
    let h = &mut entered.h;
    // Real promoted source4000143, spawned through the production copied-map loader.
    let at = Vec3::new(715.642, 100.165, 320.284);
    let switch = {
        let world = h.app.world_mut();
        let mut query = world.query::<(Entity, &shared::components::Npc, &InMap, &Position)>();
        query
            .iter(world)
            .find(|(_, npc, map, position)| {
                npc.template_id == 60283 && **map == entered.copy && close(wow(position), at)
            })
            .map(|(entity, _, _, _)| entity)
            .expect("authentic eastern Track Switch")
    };
    h.walk_to(actor, at);
    // Identical geometry in another copy cannot supply a spellclick target.
    let other_copy = InMap {
        instance_id: entered.copy.instance_id + 1,
        ..entered.copy
    };
    h.app.world_mut().entity_mut(switch).insert(other_copy);
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::InteractNpc>>(actor.connection)
        .unwrap()
        .send::<InteractionChannel>(shared::protocol::InteractNpc {
            npc: switch.to_bits(),
        });
    h.receive_pvp_updates(3);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_none()
    );
    assert!(battleground(h, actor).world_states.contains(&(6467, 2)));
    h.app.world_mut().entity_mut(switch).insert(entered.copy);
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::InteractNpc>>(actor.connection)
        .unwrap()
        .send::<InteractionChannel>(shared::protocol::InteractNpc {
            npc: switch.to_bits(),
        });
    h.receive_pvp_updates(3);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_some()
    );
    h.advance(40);
    crate::spell_cast::cancel_cast(h.app.world_mut(), actor.player);
    h.advance(24);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_none()
    );
    assert!(
        battleground(h, actor).world_states.contains(&(6467, 2)),
        "cancelled cast never changes track"
    );
    h.app
        .world_mut()
        .get_mut::<MessageSender<shared::protocol::InteractNpc>>(actor.connection)
        .unwrap()
        .send::<InteractionChannel>(shared::protocol::InteractNpc {
            npc: switch.to_bits(),
        });
    h.receive_pvp_updates(3);
    let cast = h
        .app
        .world()
        .get::<shared::casting::CastState>(actor.player)
        .expect("source124492 cast bar");
    assert_eq!(cast.spell_id, 124492);
    assert_eq!(cast.duration, 3.0);
    assert!(
        battleground(h, actor).world_states.contains(&(6467, 2)),
        "not instant"
    );
    h.advance(40);
    assert!(
        battleground(h, actor).world_states.contains(&(6467, 2)),
        "still casting at2s"
    );
    h.advance(24);
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(actor.player)
            .is_none()
    );
    assert!(
        battleground(h, actor).world_states.contains(&(6467, 1)),
        "97372 selects and changes eastern switch"
    );
}

fn carts_in(h: &mut Harness, copy: InMap) -> Vec<(Entity, Vec3)> {
    let world = h.app.world_mut();
    let mut query = world.query::<(Entity, &shared::components::Npc, &InMap, &Position)>();
    query
        .iter(world)
        .filter(|(_, npc, map, _)| npc.template_id == 60140 && **map == copy)
        .map(|(entity, _, _, position)| (entity, wow(position)))
        .collect()
}

#[test]
fn silvershard_channel_real_npc_carts_control_delivery_respawn_victory_and_origin_return() {
    let template = shared::battleground::silvershard_mines();
    let mut entered = enter_source_match(&template, shared::battleground::SILVERSHARD_STARTS);
    let alliance: Vec<_> = entered
        .players
        .iter()
        .zip(&entered.teams)
        .filter(|(_, team)| **team == BgTeam::A)
        .map(|(who, _)| who)
        .collect();
    let (h, copy) = (&mut entered.h, entered.copy);
    use_second_frames(h);
    let initial = carts_in(h, copy);
    assert_eq!(initial.len(), 3);
    for (entity, _) in &initial {
        assert!(h.app.world().get::<Replicate>(*entity).is_some());
    }
    let mut delivered = false;
    let mut respawned = false;
    for _ in 0..3000 {
        let carts = carts_in(h, copy);
        for (who, (_, position)) in alliance.iter().zip(&carts) {
            h.app
                .world_mut()
                .entity_mut(who.player)
                .insert(bevy(*position));
        }
        h.advance(1);
        let current = carts_in(h, copy);
        delivered |= current.len() < 3;
        respawned |= delivered && current.len() == 3;
        if battleground(h, alliance[0]).phase == BgPhase::Ended {
            break;
        }
    }
    assert!(
        delivered && respawned,
        "source depot disappearance and five-second replacement"
    );
    let result = battleground(h, alliance[0]);
    assert!(
        result
            .players
            .iter()
            .filter(|row| row.team == BgTeam::A)
            .all(|row| row
                .objective_scores
                .iter()
                .any(|(id, value)| *id == 428 && *value > 0))
    );
    assert!(result.world_states.contains(&(6437, 1500)));
    win_leave_and_return(&mut entered, 1500);
}

/// Four Alliance players hold the towers by presence; the carrier scores at a held tower's
/// trigger, the flag respawns, a dead carrier drops it and it expires home before 1500 wins.
#[test]
fn eye_of_the_storm_channel_towers_flag_drop_victory_and_origin_return() {
    let template = shared::battleground::eye_of_the_storm();
    let rules = shared::battleground::eye_rules(template.map_id).unwrap();
    let mut entered = enter_source_match(&template, rules.starts);
    let alliance: Vec<_> = entered
        .players
        .iter()
        .zip(&entered.teams)
        .filter(|(_, team)| **team == BgTeam::A)
        .map(|(who, _)| who)
        .collect();
    let (h, copy) = (&mut entered.h, entered.copy);
    use_second_frames(h);
    for (who, tower) in alliance.iter().zip(&rules.towers) {
        h.app
            .world_mut()
            .entity_mut(who.player)
            .insert(bevy(Vec3::from_array(tower.position)));
    }
    h.advance(40);
    let carrier = alliance[0];
    let held = battleground(h, carrier);
    assert!(
        held.world_states.contains(&(2752, 4)),
        "{:?}",
        held.world_states
    );
    assert!(held.world_states.contains(&(2726, 1)), "Fel Reaver held");

    let (flag, home) = objects_in(h, copy, rules.flag_entry)[0];
    h.walk_to(carrier, home);
    use_object(h, carrier, flag);
    let taken = battleground(h, carrier);
    assert!(taken.world_states.contains(&(8863, 2)) && taken.world_states.contains(&(9808, 2)));
    let before = taken.score_a;
    assert!(objects_in(h, copy, rules.flag_entry).is_empty());
    // Areatrigger spawn 38 beside Fel Reaver Ruins (TDB1210).
    h.walk_to(carrier, Vec3::new(2043.95, 1729.78, 1190.04));
    let scored = battleground(h, carrier);
    assert!(
        scored.score_a >= before + 500,
        "four bases: 500 per capture"
    );
    let row = scored.players.iter().find(|row| row.character_id == 1);
    assert_eq!(row.unwrap().objective_scores, [(183, 1)]);
    assert!(scored.world_states.contains(&(8863, 4)));
    h.advance(21);
    assert!(close(objects_in(h, copy, rules.flag_entry)[0].1, home));

    let (flag, _) = objects_in(h, copy, rules.flag_entry)[0];
    h.walk_to(carrier, home);
    use_object(h, carrier, flag);
    let fallen = home + Vec3::new(10.0, 0.0, 0.0);
    h.walk_to(carrier, fallen);
    h.app
        .world_mut()
        .get_mut::<shared::components::Health>(carrier.player)
        .unwrap()
        .current = 0.0;
    h.advance(1);
    let dropped = objects_in(h, copy, rules.flag_entry);
    assert_eq!(dropped.len(), 1);
    assert!(close(dropped[0].1, fallen));
    let states = &battleground(h, carrier).world_states;
    assert!(states.contains(&(8863, 3)) && states.contains(&(9809, 1)));
    h.advance(10);
    assert!(close(objects_in(h, copy, rules.flag_entry)[0].1, home));
    assert!(battleground(h, carrier).world_states.contains(&(8863, 1)));

    h.advance(300);
    win_leave_and_return(&mut entered, 1500);
}
