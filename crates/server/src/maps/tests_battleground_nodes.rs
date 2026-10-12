use super::*;
use shared::battleground::{BgOutcome, BgPhase, BgTeam, BgTemplate, node_rules};

fn node_match(template: BgTemplate) {
    let points = source_battleground_objects(template.map_id);
    let source_points = points.clone();
    let rules = node_rules(template.map_id).unwrap();
    assert_eq!(points.0.len(), rules.nodes.len());
    let database_file = tempfile::NamedTempFile::new().unwrap();
    let db = redb::Database::create(database_file.path()).unwrap();
    crate::persistence::create_tables(&db);
    let mut h = harness_with(|app| {
        app.insert_resource(crate::persistence::GameDatabase { db });
        app.insert_resource(points);
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
        .map(|who| {
            h.app
                .world()
                .get::<crate::pvp::PlayerPvpState>(who.player)
                .unwrap()
                .battleground
                .clone()
                .expect("authoritative invitation")
        })
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
        let start = rules.starts[if invitation.team == BgTeam::A { 0 } else { 1 }];
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
    let actor = &players[0];
    for point in source_points.0 {
        let object = {
            let world = h.app.world_mut();
            let mut query = world.query::<(Entity, &GameObjectInfo, &InMap)>();
            query
                .iter(world)
                .find_map(|(entity, info, map)| {
                    (info.entry == point.info.entry && *map == copy).then_some(entity)
                })
                .expect("physical authored capture point")
        };
        h.walk_to(actor, point.position);
        h.app
            .world_mut()
            .get_mut::<MessageSender<UseGameObject>>(actor.connection)
            .unwrap()
            .send::<InteractionChannel>(UseGameObject {
                object: object.to_bits(),
            });
        h.receive_pvp_updates(5);
    }
    let before = h
        .app
        .world()
        .get::<crate::pvp::PlayerPvpState>(actor.player)
        .unwrap()
        .battleground
        .as_ref()
        .unwrap();
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
    h.advance(120);
    for who in &players {
        let result = h
            .app
            .world()
            .get::<crate::pvp::PlayerPvpState>(who.player)
            .unwrap()
            .battleground
            .as_ref()
            .unwrap();
        assert_eq!(result.phase, BgPhase::Ended);
        assert_eq!(result.outcome, Some(BgOutcome::Winner(BgTeam::A)));
        assert_eq!(result.score_a, 1500);
        assert_eq!(result.score_b, 0);
        assert_eq!(result.honor_reward, None);
        assert_eq!(result.players.len(), count as usize);
        assert!(result.world_states.contains(&(1776, 1500)));
        h.app
            .world_mut()
            .get_mut::<MessageSender<LeaveBattleground>>(who.connection)
            .unwrap()
            .send::<PvpChannel>(LeaveBattleground { match_id });
    }
    h.receive_pvp_updates(5);
    for who in &players {
        let returns = h.new_worlds(who);
        assert_eq!(returns.len(), 1);
        assert_eq!(returns[0].map_id, 0);
        h.ack(who);
        assert_eq!(h.map_of(who), Some(STORMWIND));
        assert!(close(h.position(who), origin));
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
            .copy_difficulty(copy)
            .is_none()
    );
}

#[test]
fn arathi_channel_objectives_scoreboard_victory_and_origin_return() {
    node_match(shared::battleground::arathi_basin());
}
#[test]
fn gilneas_channel_objectives_scoreboard_victory_and_origin_return() {
    node_match(shared::battleground::battle_for_gilneas());
}
