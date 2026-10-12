use std::collections::BTreeMap;

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::battleground::{BgPhase, BgTeam, FlagState, template_by_id};
use shared::components::{Health, Level, Player, Position};
use shared::protocol::{
    BattlegroundPlayerScoreSnapshot, GameObjectInfo, WARSONG_ALLIANCE_FLAG_ENTRY,
    WARSONG_HORDE_FLAG_ENTRY,
};

use super::{InvitedBattleground, PlayerPvpState, PvpServerState};
use crate::auth::{Authenticated, InWorld};
use crate::game_objects::{GameObjectSpawn, GameObjectSpawns, game_object_bundle, wow_position};
use crate::interest::NetworkVisibility;
use crate::maps::InMap;
use crate::networking::PlayerLink;
use crate::persistence::ClientId;

// TDB1210 custom capture-create-properties30/31: sphere ShapeData0=5.
const CAPTURE_RADIUS: f32 = 5.0;

#[derive(Clone)]
struct Participant {
    map: InMap,
    position: Vec3,
    alive: bool,
    name: String,
    class: u8,
    level: u8,
}

pub(super) fn update_battleground_objectives(world: &mut World) {
    let actors = read_participants(world);
    let sources = world.resource::<GameObjectSpawns>().clone();
    let changed = world.resource_scope(|world, mut state: Mut<PvpServerState>| {
        state
            .battleground_matches
            .iter_mut()
            .filter_map(|(match_id, matched)| {
                update_match(world, matched, &actors, &sources).then_some(*match_id)
            })
            .collect::<Vec<_>>()
    });
    for match_id in changed {
        send_match_snapshot(world, match_id);
    }
}

fn read_participants(world: &mut World) -> BTreeMap<u64, Participant> {
    let mut query = world.query::<(&ClientId, &InMap, &Position, &Health, &Player, &Level)>();
    query
        .iter(world)
        .map(|(id, map, position, health, player, level)| {
            (
                id.0,
                Participant {
                    map: *map,
                    position: wow_position(position),
                    alive: health.current > 0.0,
                    name: player.name.clone(),
                    class: player.class,
                    level: level.0,
                },
            )
        })
        .collect()
}

fn update_match(
    world: &mut World,
    matched: &mut InvitedBattleground,
    actors: &BTreeMap<u64, Participant>,
    sources: &GameObjectSpawns,
) -> bool {
    let Some(map) = matched.map else { return false };
    let template =
        template_by_id(matched.instance.bg_id).expect("matched template remains registered");
    let mut changed = false;
    if matched.instance.phase == BgPhase::Waiting
        && roster_ready(matched, actors, map, template.min_players_per_team as usize)
    {
        matched.scoreboard = build_scoreboard(matched, actors);
        changed = matched.instance.start();
    }
    if matched.instance.phase != BgPhase::InProgress {
        return changed;
    }
    if let Some(nodes) = matched.nodes.as_mut() {
        let before = nodes.world_states(&matched.instance);
        let delta_ms = world.resource::<Time<Fixed>>().delta().as_millis() as u64;
        nodes.advance(delta_ms, &mut matched.instance, &template);
        return changed || before != nodes.world_states(&matched.instance);
    }
    let Some(homes) = flag_homes(sources, map) else {
        return changed;
    };
    for team in [BgTeam::A, BgTeam::B] {
        let enemy = enemy_team(team);
        let flag = flag_state(matched, enemy);
        let FlagState::Carried { carrier } = flag else {
            continue;
        };
        let actor = actors
            .get(&carrier)
            .filter(|actor| actor.map == map && actor.alive);
        let Some(actor) = actor else {
            matched.flags.drop_flag(team);
            if let Some(position) = matched.flag_positions[team_index(enemy)] {
                spawn_flag(world, map, &homes[team_index(enemy)], position);
            }
            changed = true;
            continue;
        };
        matched.flag_positions[team_index(enemy)] = Some(actor.position);
        if actor.position.distance(homes[team_index(team)].position) <= CAPTURE_RADIUS
            && matched
                .flags
                .capture(team, carrier, &mut matched.instance, &template)
        {
            if let Some(score) = matched
                .scoreboard
                .iter_mut()
                .find(|score| score.character_id == carrier)
            {
                score.flag_captures += 1;
            }
            let home = &homes[team_index(enemy)];
            spawn_flag(world, map, home, home.position);
            matched.flag_positions[team_index(enemy)] = None;
            changed = true;
        }
    }
    changed
}

fn roster_ready(
    matched: &InvitedBattleground,
    actors: &BTreeMap<u64, Participant>,
    map: InMap,
    minimum: usize,
) -> bool {
    [
        &matched.instance.team_a_players,
        &matched.instance.team_b_players,
    ]
    .iter()
    .all(|team| {
        team.iter()
            .filter(|id| {
                matched.accepted.contains(id)
                    && actors.get(id).is_some_and(|actor| actor.map == map)
            })
            .count()
            >= minimum
    })
}

fn build_scoreboard(
    matched: &InvitedBattleground,
    actors: &BTreeMap<u64, Participant>,
) -> Vec<BattlegroundPlayerScoreSnapshot> {
    [
        (BgTeam::A, &matched.instance.team_a_players),
        (BgTeam::B, &matched.instance.team_b_players),
    ]
    .into_iter()
    .flat_map(|(team, ids)| {
        ids.iter().filter_map(move |id| {
            actors.get(id).map(|actor| BattlegroundPlayerScoreSnapshot {
                character_id: *id,
                name: actor.name.clone(),
                class: actor.class,
                level: actor.level,
                team,
                killing_blows: 0,
                deaths: 0,
                damage_done: 0,
                healing_done: 0,
                flag_captures: 0,
                flag_returns: 0,
                objective_scores: shared::battleground::node_rules(map_id(matched))
                    .map(|rules| vec![(rules.assault_stat, 0), (rules.defense_stat, 0)])
                    .unwrap_or_default(),
                honorable_kills: None,
                honor_level: None,
            })
        })
    })
    .collect()
}

fn flag_homes(sources: &GameObjectSpawns, map: InMap) -> Option<[GameObjectSpawn; 2]> {
    let find = |entry| {
        sources
            .0
            .iter()
            .find(|spawn| spawn.map_id == map.map_id && spawn.info.entry == entry)
            .cloned()
    };
    Some([
        find(WARSONG_ALLIANCE_FLAG_ENTRY)?,
        find(WARSONG_HORDE_FLAG_ENTRY)?,
    ])
}

fn spawn_flag(world: &mut World, map: InMap, source: &GameObjectSpawn, position: Vec3) {
    let mut spawn = source.clone();
    spawn.position = position;
    world.spawn((
        game_object_bundle(&spawn),
        map,
        Replicate::to_clients(NetworkTarget::All),
        NetworkVisibility::default(),
    ));
}

fn map_id(matched: &InvitedBattleground) -> u32 {
    template_by_id(matched.instance.bg_id)
        .expect("registered match template")
        .map_id
}

pub(crate) fn use_battleground_object(
    world: &mut World,
    connection: Entity,
    player: Entity,
    object: Entity,
) {
    let result = world.resource_scope(|world, mut state: Mut<PvpServerState>| {
        if world
            .get::<GameObjectInfo>(object)
            .is_some_and(GameObjectInfo::is_battleground_capture_point)
        {
            assault_capture_point(world, &mut state, player, object)
        } else {
            pickup_or_return_flag(world, &mut state, player, object)
        }
    });
    super::pvp_invites::send_response(world, connection, player, result.clone());
    if result.is_ok() {
        if let Some(match_id) = world
            .get::<PlayerPvpState>(player)
            .and_then(|player| player.battleground.as_ref())
            .map(|bg| bg.match_id)
        {
            send_match_snapshot(world, match_id);
        }
    }
}

fn assault_capture_point(
    world: &World,
    state: &mut PvpServerState,
    player: Entity,
    object: Entity,
) -> Result<String, String> {
    let snapshot = world
        .get::<PlayerPvpState>(player)
        .and_then(|state| state.battleground.as_ref())
        .ok_or("player is not in a battleground")?;
    let character_id = world
        .get::<ClientId>(player)
        .ok_or("player character missing")?
        .0;
    let entry = world
        .get::<GameObjectInfo>(object)
        .ok_or("capture point is no longer available")?
        .entry;
    let matched = state
        .battleground_matches
        .get_mut(&snapshot.match_id)
        .ok_or("match missing")?;
    let map = matched.map.ok_or("battleground map missing")?;
    if world.get::<InMap>(player) != Some(&map)
        || world.get::<InMap>(object) != Some(&map)
        || matched.instance.phase != BgPhase::InProgress
        || !matched.accepted.contains(&character_id)
    {
        return Err("capture point is not usable in this player's active match".into());
    }
    let nodes = matched
        .nodes
        .as_mut()
        .ok_or("match has no capture points")?;
    let stat = nodes.assault(entry, snapshot.team)?;
    let score = matched
        .scoreboard
        .iter_mut()
        .find(|score| score.character_id == character_id)
        .ok_or("participant scoreboard row missing")?;
    let counter = score
        .objective_scores
        .iter_mut()
        .find(|(id, _)| *id == stat)
        .ok_or("source objective scoreboard column missing")?;
    counter.1 += 1;
    Ok("capture point state updated".into())
}

fn pickup_or_return_flag(
    world: &mut World,
    state: &mut PvpServerState,
    player: Entity,
    object: Entity,
) -> Result<String, String> {
    let snapshot = world
        .get::<PlayerPvpState>(player)
        .and_then(|player| player.battleground.clone())
        .ok_or("player is not in a battleground")?;
    let character_id = world
        .get::<ClientId>(player)
        .ok_or("player character missing")?
        .0;
    let info = world
        .get::<GameObjectInfo>(object)
        .ok_or("flag is no longer available")?;
    let owner = match info.entry {
        WARSONG_ALLIANCE_FLAG_ENTRY => BgTeam::A,
        WARSONG_HORDE_FLAG_ENTRY => BgTeam::B,
        _ => return Err("not a Warsong flag".into()),
    };
    let matched = state
        .battleground_matches
        .get_mut(&snapshot.match_id)
        .ok_or("battleground match missing")?;
    let map = matched.map.ok_or("battleground map missing")?;
    if world.get::<InMap>(player) != Some(&map)
        || world.get::<InMap>(object) != Some(&map)
        || matched.instance.phase != BgPhase::InProgress
    {
        return Err("flag is not usable in this player's active match".into());
    }
    if !matched.accepted.contains(&character_id) {
        return Err("player has not accepted this match".into());
    }
    if owner == snapshot.team {
        return return_own_flag(world, matched, map, object, owner, character_id);
    }
    if !matched.flags.pickup(snapshot.team, character_id) {
        return Err("enemy flag is not available".into());
    }
    matched.flag_positions[team_index(owner)] = Some(wow_position(
        world.get::<Position>(player).expect("positioned player"),
    ));
    world.despawn(object);
    Ok("enemy flag picked up".into())
}

fn return_own_flag(
    world: &mut World,
    matched: &mut InvitedBattleground,
    map: InMap,
    object: Entity,
    owner: BgTeam,
    character_id: u64,
) -> Result<String, String> {
    if !matched.flags.return_flag(owner) {
        return Err("your flag is not dropped".into());
    }
    let sources = world.resource::<GameObjectSpawns>();
    let homes = flag_homes(sources, map).ok_or("authored flag homes missing")?;
    world.despawn(object);
    let home = &homes[team_index(owner)];
    spawn_flag(world, map, home, home.position);
    matched.flag_positions[team_index(owner)] = None;
    if let Some(score) = matched
        .scoreboard
        .iter_mut()
        .find(|score| score.character_id == character_id)
    {
        score.flag_returns += 1;
    }
    Ok("own flag returned".into())
}

fn flag_state(matched: &InvitedBattleground, owner: BgTeam) -> FlagState {
    match owner {
        BgTeam::A => matched.flags.flag_a,
        BgTeam::B => matched.flags.flag_b,
    }
}
fn team_index(team: BgTeam) -> usize {
    match team {
        BgTeam::A => 0,
        BgTeam::B => 1,
    }
}
fn enemy_team(team: BgTeam) -> BgTeam {
    match team {
        BgTeam::A => BgTeam::B,
        BgTeam::B => BgTeam::A,
    }
}

fn send_match_snapshot(world: &mut World, match_id: u64) {
    let values = {
        let state = world.resource::<PvpServerState>();
        let Some(matched) = state.battleground_matches.get(&match_id) else {
            return;
        };
        (
            matched.instance.phase,
            matched.instance.score_a.score,
            matched.instance.score_b.score,
            matched.flags.clone(),
            matched.instance.outcome,
            matched.scoreboard.clone(),
            matched
                .nodes
                .as_ref()
                .map(|nodes| nodes.world_states(&matched.instance))
                .unwrap_or_default(),
        )
    };
    let mut connections =
        world.query_filtered::<(Entity, &PlayerLink, &InWorld), With<Authenticated>>();
    let recipients: Vec<_> = connections
        .iter(world)
        .map(|(connection, link, _)| (connection, link.0))
        .collect();
    for (connection, player) in recipients {
        let Some(mut state) = world.get_mut::<PlayerPvpState>(player) else {
            continue;
        };
        let Some(snapshot) = state
            .battleground
            .as_mut()
            .filter(|bg| bg.match_id == match_id)
        else {
            continue;
        };
        snapshot.phase = values.0;
        snapshot.score_a = values.1;
        snapshot.score_b = values.2;
        snapshot.flags = values.3.clone();
        snapshot.outcome = values.4;
        snapshot.players = values.5.clone();
        snapshot.world_states = values.6.clone();
        super::pvp_invites::send_response(
            world,
            connection,
            player,
            Ok("battleground state updated".into()),
        );
    }
}
