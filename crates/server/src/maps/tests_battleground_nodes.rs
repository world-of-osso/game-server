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

/// Source-minimum roster queue, invitation, acceptance and safe-location entry over the channel.
fn enter_source_match(template: &BgTemplate, starts: [u32; 2]) -> EnteredMatch {
    let objects = if template.map_id == 998 {
        source_kotmogu_objects()
    } else {
        source_battleground_objects(template.map_id)
    };
    let database_file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(database_file.path()).unwrap();
    crate::persistence::create_tables(&db);
    let mut h = harness_with(|app| {
        app.insert_resource(crate::persistence::GameDatabase { db });
        app.insert_resource(objects);
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
        let source = map_store().unwrap().safe_locs[&start];
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
        let score_state = if copy.map_id == 998 { 6303 } else { 1776 };
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
