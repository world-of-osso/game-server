use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;
use lightyear::prelude::*;
use shared::battleground::{
    BgPhase, BgTeam, BgTemplate, FlagObjective, FlagSite, FlagState, template_by_id,
};
use shared::components::{Health, Level, Player, Position};
use shared::protocol::{
    BattlegroundPlayerScoreSnapshot, GameObjectInfo, WARSONG_ALLIANCE_FLAG_ENTRY,
    WARSONG_HORDE_FLAG_ENTRY,
};

use super::{InvitedBattleground, PlayerPvpState, PvpServerState, Scoreboard};
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
    let arathi_resources = super::honor::read_arathi_resource_counters(world);
    let isle_resources = super::honor::read_isle_resource_counters(world);
    let alterac_towers = super::honor::read_alterac_tower_counters(world);
    let changed = world.resource_scope(|world, mut state: Mut<PvpServerState>| {
        state
            .battleground_matches
            .iter_mut()
            .filter_map(|(match_id, matched)| {
                update_match(world, matched, &actors).then_some(*match_id)
            })
            .collect::<Vec<_>>()
    });
    super::honor::award_arathi_resource_gains(world, arathi_resources);
    super::honor::award_isle_resource_gains(world, isle_resources);
    super::honor::award_alterac_tower_changes(world, alterac_towers);
    credit_objective_events(world);
    for match_id in changed {
        super::honor::award_alterac_survivor_honor(world, match_id);
        credit_completed_match(world, match_id);
        send_match_snapshot(world, match_id);
    }
}

/// TC Battleground::EndBattleground (Battleground.cpp:751,779): every accepted player
/// still in the copy participates; the winning roster also wins. Draws have no winner.
fn credit_completed_match(world: &mut World, match_id: u64) {
    use shared::battleground::BgOutcome;
    let (map, roster, winners, bracket) = {
        let state = world.resource::<PvpServerState>();
        let matched = &state.battleground_matches[&match_id];
        if matched.instance.phase != BgPhase::Ended {
            return;
        }
        let (Some(outcome), Some(map)) = (matched.instance.outcome, matched.map) else {
            return;
        };
        let instance = &matched.instance;
        let winners = match outcome {
            BgOutcome::Winner(BgTeam::A) => instance.team_a_players.clone(),
            BgOutcome::Winner(BgTeam::B) => instance.team_b_players.clone(),
            BgOutcome::Draw => Vec::new(),
        };
        let roster: Vec<_> = instance
            .team_a_players
            .iter()
            .chain(&instance.team_b_players)
            .filter(|id| matched.accepted.contains(id))
            .copied()
            .collect();
        (map, roster, winners, instance.bracket)
    };
    let players: Vec<_> = world
        .query::<(&Authenticated, &PlayerLink, &InWorld)>()
        .iter(world)
        .filter(|(_, link, owner)| {
            roster.contains(&owner.character_id) && world.get::<InMap>(link.0) == Some(&map)
        })
        .map(|(_, link, owner)| (link.0, winners.contains(&owner.character_id)))
        .collect();
    for (player, won) in players {
        // TC Battleground.cpp675–695: clean up life/combat before the result snapshot.
        let dead = world
            .get::<Health>(player)
            .is_some_and(|health| health.current <= 0.0);
        if dead {
            crate::death::resurrect_player_fully(world, player);
        }
        // TC Battleground.cpp772–773: ResetAllPowers precedes CombatStopWithPets.
        super::arena_preparation::reset_resources(world, player);
        crate::combat::combat_stop(world, player);
        if let Some(pet) = crate::hunter_pet::summoned_pet(world, player) {
            crate::combat::combat_stop(world, pet);
        }
        crate::achievement::record_battleground_end(world, player, map.map_id, won);
        super::honor::award_battleground_completion(world, player, map.map_id, bracket, won);
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
) -> bool {
    if matched.arena.is_some() {
        return false;
    }
    let Some(map) = matched.map else { return false };
    let template =
        template_by_id(matched.instance.bg_id).expect("matched template remains registered");
    let now = world.resource::<Time<Fixed>>().elapsed_secs_f64();
    let mut changed = false;
    if matched.instance.phase == BgPhase::Waiting {
        let arrived = matched
            .accepted
            .iter()
            .any(|id| actors.get(id).is_some_and(|actor| actor.map == map));
        let ready = roster_ready(matched, actors, map, template.min_players_per_team as usize);
        if matched.preparation.advance(now, arrived, ready) {
            changed = matched.instance.start();
        }
    }
    super::bg_preparation::update_gates(world, map, &matched.preparation, now);
    if matched.instance.phase != BgPhase::InProgress {
        return changed;
    }
    changed |= append_arrived_scores(matched, actors, map);
    let delta_ms = world.resource::<Time<Fixed>>().delta().as_millis() as u64;
    if matched.alterac.is_some() {
        return super::pvp_alterac::update_alterac(world, matched, map, delta_ms) || changed;
    }
    if let Some(nodes) = matched.nodes.as_mut() {
        let before = nodes.world_states(&matched.instance);
        nodes.advance(delta_ms, &mut matched.instance, &template);
        return changed || before != nodes.world_states(&matched.instance);
    }
    if matched.eye.is_some() {
        return update_eye(world, matched, actors, map, &template, delta_ms) || changed;
    }
    if matched.kotmogu.is_some() {
        return update_kotmogu(world, matched, actors, map, &template, delta_ms) || changed;
    }
    if matched.silvershard.is_some() {
        return update_silvershard(world, matched, actors, map, &template, delta_ms) || changed;
    }
    if matched.deephaul.is_some() {
        return update_deephaul(world, matched, actors, map, &template, delta_ms) || changed;
    }
    if matched.seething.is_some() {
        super::pvp_seething::update_seething(world, matched, map, delta_ms);
        return changed;
    }
    let Some(battle) = matched.flags.as_mut() else {
        return changed;
    };
    battle.advance(delta_ms, &mut matched.instance);
    if matched.instance.phase != BgPhase::InProgress {
        return true;
    }
    // Only flag maps need two authored homes; preserve master's catalog-clone removal.
    let Some(homes) = flag_homes(world.resource::<GameObjectSpawns>(), map) else {
        return changed;
    };
    for team in [BgTeam::A, BgTeam::B] {
        let enemy = enemy_team(team);
        let FlagState::Carried { carrier } = flag_state(&battle.flags, enemy) else {
            continue;
        };
        let actor = actors
            .get(&carrier)
            .filter(|actor| actor.map == map && actor.alive);
        let Some(actor) = actor else {
            battle.flags.drop_flag(team);
            if let Some(position) = matched.flag_positions[team_index(enemy)] {
                spawn_flag(world, map, &homes[team_index(enemy)], position);
            }
            changed = true;
            continue;
        };
        matched.flag_positions[team_index(enemy)] = Some(actor.position);
        // Capture triggers 30/31 sit exactly on the authored flag homes (TDB areatrigger).
        if actor.position.distance(homes[team_index(team)].position) <= CAPTURE_RADIUS
            && battle.capture(team, carrier, &mut matched.instance, &template)
        {
            credit(
                &mut matched.scoreboard,
                carrier,
                battle.rules.capture_stat,
                1,
            );
            let home = &homes[team_index(enemy)];
            spawn_flag(world, map, home, home.position);
            matched.flag_positions[team_index(enemy)] = None;
            changed = true;
        }
    }
    changed
}

/// Presence heartbeats and timers, then the carrier: alive on the map it may score at a
/// trigger, otherwise the flag drops where it was last carried.
fn update_eye(
    world: &mut World,
    matched: &mut InvitedBattleground,
    actors: &BTreeMap<u64, Participant>,
    map: InMap,
    template: &BgTemplate,
    delta_ms: u64,
) -> bool {
    let present: Vec<_> = [
        (BgTeam::A, &matched.instance.team_a_players),
        (BgTeam::B, &matched.instance.team_b_players),
    ]
    .into_iter()
    .flat_map(|(team, ids)| {
        ids.iter().filter_map(move |id| {
            let actor = actors.get(id).filter(|a| a.map == map && a.alive)?;
            Some((team, actor.position.to_array()))
        })
    })
    .collect();
    let eye = matched.eye.as_mut().expect("Eye of the Storm match");
    let before = (eye.world_states(&matched.instance), eye.flag_site());
    eye.advance(delta_ms, &present, &mut matched.instance, template);
    if let Some(carrier) = eye
        .carrier()
        .filter(|_| matched.instance.phase == BgPhase::InProgress)
    {
        let actor = actors.get(&carrier).filter(|actor| actor.map == map);
        match actor {
            Some(actor) if actor.alive => {
                matched.flag_positions[0] = Some(actor.position);
                let team = if matched.instance.team_a_players.contains(&carrier) {
                    BgTeam::A
                } else {
                    BgTeam::B
                };
                let at = actor.position.to_array();
                if eye.capture(carrier, team, at, &mut matched.instance, template) {
                    credit(&mut matched.scoreboard, carrier, eye.rules.capture_stat, 1);
                }
            }
            _ => {
                let at = actor
                    .map(|actor| actor.position)
                    .or(matched.flag_positions[0]);
                eye.drop_flag(at.expect("carried position recorded at pickup").to_array());
            }
        }
    }
    if eye.flag_site() != before.1 {
        place_objective(world, map, eye.rules.flag_entry, eye.flag_site());
    }
    before.0 != eye.world_states(&matched.instance)
}

fn update_kotmogu(
    world: &mut World,
    matched: &mut InvitedBattleground,
    actors: &BTreeMap<u64, Participant>,
    map: InMap,
    template: &BgTemplate,
    delta_ms: u64,
) -> bool {
    let players: Vec<_> = actors
        .iter()
        .filter(|(_, actor)| actor.map == map && actor.alive)
        .map(|(id, actor)| (*id, actor.position.to_array()))
        .collect();
    let orbs = matched.kotmogu.as_mut().expect("Kotmogu match");
    let before = (orbs.world_states(&matched.instance), orbs.holders());
    let points = orbs.advance(delta_ms, &players, &mut matched.instance, template);
    for (character, points) in points {
        credit(
            &mut matched.scoreboard,
            character,
            shared::battleground::ORB_VICTORY_POINTS_STAT,
            points,
        );
    }
    let held = orbs.holders();
    for (entry, _, _) in before.1 {
        if !held.iter().any(|(taken, _, _)| *taken == entry) {
            place_objective(world, map, entry, Some(FlagSite::Home));
        }
    }
    before.0 != orbs.world_states(&matched.instance)
}

fn update_silvershard(
    world: &mut World,
    matched: &mut InvitedBattleground,
    actors: &BTreeMap<u64, Participant>,
    map: InMap,
    template: &BgTemplate,
    delta_ms: u64,
) -> bool {
    let players = living_participants(matched, actors, map);
    let carts = matched.silvershard.as_mut().expect("Silvershard match");
    let before = carts.battle.world_states(&matched.instance);
    for event in carts.advance(
        world,
        map,
        delta_ms,
        &players,
        &mut matched.instance,
        template,
    ) {
        if let shared::battleground::CartEvent::Controlled(id) = event {
            credit(
                &mut matched.scoreboard,
                id,
                shared::battleground::CARTS_CONTROLLED_STAT,
                1,
            );
        }
    }
    before != carts.battle.world_states(&matched.instance)
}

fn living_participants(
    matched: &InvitedBattleground,
    actors: &BTreeMap<u64, Participant>,
    map: InMap,
) -> Vec<(u64, BgTeam, [f32; 3])> {
    [
        (BgTeam::A, &matched.instance.team_a_players),
        (BgTeam::B, &matched.instance.team_b_players),
    ]
    .into_iter()
    .flat_map(|(team, ids)| {
        ids.iter().filter_map(move |id| {
            let actor = actors
                .get(id)
                .filter(|actor| actor.map == map && actor.alive)?;
            Some((*id, team, actor.position.to_array()))
        })
    })
    .collect()
}

fn update_deephaul(
    world: &mut World,
    matched: &mut InvitedBattleground,
    actors: &BTreeMap<u64, Participant>,
    map: InMap,
    template: &BgTemplate,
    delta_ms: u64,
) -> bool {
    let players = living_participants(matched, actors, map);
    let runtime = matched.deephaul.as_mut().expect("Deephaul match");
    let before = (
        runtime.battle.world_states(&matched.instance),
        runtime.battle.flag_site(),
    );
    for event in runtime.advance(
        world,
        map,
        delta_ms,
        &players,
        &mut matched.instance,
        template,
    ) {
        if let shared::battleground::CartEvent::Controlled(id) = event {
            credit(
                &mut matched.scoreboard,
                id,
                shared::battleground::DEEPHAUL_CONTROL_STAT,
                1,
            );
        }
    }
    update_crystal_carrier(matched, actors, map, template);
    let runtime = matched.deephaul.as_ref().expect("Deephaul match");
    if runtime.battle.flag_site() != before.1 {
        runtime.synchronize_crystal(world, map);
    }
    before.0 != runtime.battle.world_states(&matched.instance)
}

fn update_crystal_carrier(
    matched: &mut InvitedBattleground,
    actors: &BTreeMap<u64, Participant>,
    map: InMap,
    template: &BgTemplate,
) {
    let runtime = matched.deephaul.as_mut().expect("Deephaul match");
    let Some(carrier) = runtime
        .battle
        .carrier()
        .filter(|_| matched.instance.phase == BgPhase::InProgress)
    else {
        return;
    };
    let actor = actors.get(&carrier).filter(|actor| actor.map == map);
    if let Some(actor) = actor.filter(|actor| actor.alive) {
        matched.flag_positions[0] = Some(actor.position);
        let team = if matched.instance.team_a_players.contains(&carrier) {
            BgTeam::A
        } else {
            BgTeam::B
        };
        if runtime.battle.capture(
            carrier,
            team,
            actor.position.to_array(),
            &mut matched.instance,
            template,
        ) {
            credit(
                &mut matched.scoreboard,
                carrier,
                shared::battleground::DEEPHAUL_FLAG_STAT,
                1,
            );
        }
    } else {
        let at = actor
            .map(|actor| actor.position)
            .or(matched.flag_positions[0])
            .expect("carried crystal position recorded at pickup");
        runtime.battle.drop_flag(at.to_array());
    }
}

pub(super) fn credit(scoreboard: &mut Scoreboard, character_id: u64, stat: u32, amount: u32) {
    let counter = scoreboard
        .rows
        .iter_mut()
        .find(|score| score.character_id == character_id)
        .and_then(|score| {
            score
                .objective_scores
                .iter_mut()
                .find(|(id, _)| *id == stat)
        })
        .expect("participant row carries every source column of its map");
    counter.1 += amount;
    scoreboard.stat_credits.push((character_id, stat));
}

/// TC BattlegroundScore.cpp:78-80: each PvpStat update credits the connected player once,
/// whatever the stat amount. The same drained capture receipt dispatches per-map Honor.
fn credit_objective_events(world: &mut World) {
    let credits: Vec<_> = world
        .resource_mut::<PvpServerState>()
        .battleground_matches
        .iter_mut()
        .filter_map(|(id, matched)| {
            Some((
                *id,
                matched.map?.map_id,
                &mut matched.scoreboard.stat_credits,
            ))
        })
        .flat_map(|(id, map, credits)| {
            std::mem::take(credits)
                .into_iter()
                .map(move |c| (id, map, c))
        })
        .collect();
    if credits.is_empty() {
        return;
    }
    let players: BTreeMap<u64, Entity> = world
        .query_filtered::<(&PlayerLink, &InWorld), With<Authenticated>>()
        .iter(world)
        .map(|(link, owner)| (owner.character_id, link.0))
        .collect();
    for (match_id, map, (character, stat)) in credits {
        super::honor::award_flag_capture_honor(world, match_id, character, stat);
        if let Some(&player) = players.get(&character) {
            crate::achievement::record_pvp_stat(world, player, map, stat);
        }
    }
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

/// TC Battleground::AddPlayer creates PlayerScores on arrival, not only at start.
fn append_arrived_scores(
    matched: &mut InvitedBattleground,
    actors: &BTreeMap<u64, Participant>,
    map: InMap,
) -> bool {
    let existing: BTreeSet<_> = matched
        .scoreboard
        .rows
        .iter()
        .map(|row| row.character_id)
        .collect();
    let existing = &existing;
    let accepted = &matched.accepted;
    let battleground_map = map_id(matched);
    let arrived: Vec<_> = [
        (BgTeam::A, &matched.instance.team_a_players),
        (BgTeam::B, &matched.instance.team_b_players),
    ]
    .into_iter()
    .flat_map(|(team, ids)| {
        ids.iter().filter_map(move |id| {
            if existing.contains(id) || !accepted.contains(id) {
                return None;
            }
            actors
                .get(id)
                .filter(|actor| actor.map == map)
                .map(|actor| BattlegroundPlayerScoreSnapshot {
                    character_id: *id,
                    name: actor.name.clone(),
                    class: actor.class,
                    level: actor.level,
                    team,
                    killing_blows: 0,
                    deaths: 0,
                    damage_done: 0,
                    healing_done: 0,
                    objective_scores: shared::battleground::pvp_stat_columns(battleground_map)
                        .iter()
                        .map(|column| (column.stat, 0))
                        .collect(),
                    honorable_kills: None,
                    honor_level: None,
                })
        })
    })
    .collect();
    let changed = !arrived.is_empty();
    matched.scoreboard.rows.extend(arrived);
    changed
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

pub(super) fn spawn_flag(world: &mut World, map: InMap, source: &GameObjectSpawn, position: Vec3) {
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
    if super::pvp_alterac::is_banner(world, object) {
        let result = super::pvp_alterac::begin_opening(world, player, object);
        super::pvp_invites::send_response(world, connection, player, result);
        return;
    }
    if super::pvp_seething::is_azerite_object(world, object) {
        let result = super::pvp_seething::begin_gathering(world, player, object);
        debug!(player = ?player, object = ?object, result = ?result, "Azerite use acceptance result");
        super::pvp_invites::send_response(world, connection, player, result);
        return;
    }
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

/// The user's team and character in its accepted, in-progress match on the object's map.
pub(super) fn active_match<'a>(
    world: &World,
    state: &'a mut PvpServerState,
    player: Entity,
    object: Entity,
) -> Result<(BgTeam, u64, &'a mut InvitedBattleground), String> {
    let snapshot = world
        .get::<PlayerPvpState>(player)
        .and_then(|state| state.battleground.as_ref())
        .ok_or("player is not in a battleground")?;
    let character_id = world
        .get::<ClientId>(player)
        .ok_or("player character missing")?
        .0;
    let matched = state
        .battleground_matches
        .get_mut(&snapshot.match_id)
        .ok_or("battleground match missing")?;
    let map = matched.map.ok_or("battleground map missing")?;
    if world.get::<InMap>(player) != Some(&map)
        || world.get::<InMap>(object) != Some(&map)
        || matched.instance.phase != BgPhase::InProgress
        || !matched.accepted.contains(&character_id)
    {
        return Err("object is not usable in this player's active match".into());
    }
    Ok((snapshot.team, character_id, matched))
}

fn assault_capture_point(
    world: &World,
    state: &mut PvpServerState,
    player: Entity,
    object: Entity,
) -> Result<String, String> {
    let entry = world
        .get::<GameObjectInfo>(object)
        .ok_or("capture point is no longer available")?
        .entry;
    let (team, character_id, matched) = active_match(world, state, player, object)?;
    let nodes = matched
        .nodes
        .as_mut()
        .ok_or("match has no capture points")?;
    let stat = nodes.assault(entry, team)?;
    credit(&mut matched.scoreboard, character_id, stat, 1);
    Ok("capture point state updated".into())
}

fn pickup_or_return_flag(
    world: &mut World,
    state: &mut PvpServerState,
    player: Entity,
    object: Entity,
) -> Result<String, String> {
    let entry = world
        .get::<GameObjectInfo>(object)
        .ok_or("flag is no longer available")?
        .entry;
    let (team, character_id, matched) = active_match(world, state, player, object)?;
    let map = matched.map.expect("active match has a map");
    let position = wow_position(world.get::<Position>(player).expect("positioned player"));
    if let Some(orbs) = matched.kotmogu.as_mut() {
        orbs.pickup(entry, character_id, team)?;
        credit(
            &mut matched.scoreboard,
            character_id,
            shared::battleground::ORB_POSSESSIONS_STAT,
            1,
        );
        world.despawn(object);
        return Ok("Orb of Power picked up".into());
    }
    if let Some(deephaul) = matched.deephaul.as_mut() {
        if ![
            shared::battleground::DEEPHAUL_FLAG_ENTRY,
            shared::battleground::DEEPHAUL_DROP_ENTRY,
        ]
        .contains(&entry)
            || !deephaul.battle.pickup(team, character_id)
        {
            return Err("the Deephaul Crystal is not available".into());
        }
        matched.flag_positions[0] = Some(position);
        deephaul.synchronize_crystal(world, map);
        return Ok("Deephaul Crystal picked up".into());
    }
    if let Some(eye) = matched.eye.as_mut() {
        if entry != eye.rules.flag_entry || !eye.pickup(team, character_id) {
            return Err("the flag is not available".into());
        }
        matched.flag_positions[0] = Some(position);
        place_objective(world, map, eye.rules.flag_entry, eye.flag_site());
        return Ok("flag picked up".into());
    }
    let owner = match entry {
        WARSONG_ALLIANCE_FLAG_ENTRY => BgTeam::A,
        WARSONG_HORDE_FLAG_ENTRY => BgTeam::B,
        _ => return Err("not a battleground team flag".into()),
    };
    if owner == team {
        return return_own_flag(world, matched, map, object, owner, character_id);
    }
    let battle = matched.flags.as_mut().ok_or("match has no team flags")?;
    if !battle.flags.pickup(team, character_id) {
        return Err("enemy flag is not available".into());
    }
    matched.flag_positions[team_index(owner)] = Some(position);
    world.despawn(object);
    Ok("enemy flag picked up".into())
}

/// Show an authored objective where its rules place it: home, ground or nowhere.
fn place_objective(world: &mut World, map: InMap, entry: u32, site: Option<FlagSite>) {
    let mut objects = world.query::<(Entity, &GameObjectInfo, &InMap)>();
    let shown: Vec<_> = objects
        .iter(world)
        .filter(|(_, info, at)| info.entry == entry && **at == map)
        .map(|(object, ..)| object)
        .collect();
    for object in shown {
        world.despawn(object);
    }
    let Some(site) = site else { return };
    let home = world
        .resource::<GameObjectSpawns>()
        .0
        .iter()
        .find(|spawn| spawn.map_id == map.map_id && spawn.info.entry == entry)
        .cloned()
        .expect("entry requires the authored objective");
    let position = match site {
        FlagSite::Home => home.position,
        FlagSite::Dropped(at) => Vec3::from_array(at),
    };
    spawn_flag(world, map, &home, position);
}

fn return_own_flag(
    world: &mut World,
    matched: &mut InvitedBattleground,
    map: InMap,
    object: Entity,
    owner: BgTeam,
    character_id: u64,
) -> Result<String, String> {
    let battle = matched.flags.as_mut().ok_or("match has no team flags")?;
    if !battle.flags.return_flag(owner) {
        return Err("your flag is not dropped".into());
    }
    let return_stat = battle.rules.return_stat;
    let sources = world.resource::<GameObjectSpawns>();
    let homes = flag_homes(sources, map).ok_or("authored flag homes missing")?;
    world.despawn(object);
    let home = &homes[team_index(owner)];
    spawn_flag(world, map, home, home.position);
    matched.flag_positions[team_index(owner)] = None;
    credit(&mut matched.scoreboard, character_id, return_stat, 1);
    Ok("own flag returned".into())
}

fn flag_state(flags: &FlagObjective, owner: BgTeam) -> FlagState {
    match owner {
        BgTeam::A => flags.flag_a,
        BgTeam::B => flags.flag_b,
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

pub(super) fn send_match_snapshot(world: &mut World, match_id: u64) {
    let values = {
        let state = world.resource::<PvpServerState>();
        let Some(matched) = state.battleground_matches.get(&match_id) else {
            return;
        };
        (
            matched.instance.phase,
            matched.displayed_scores().0,
            matched.displayed_scores().1,
            matched.flag_objective(),
            matched.instance.outcome,
            matched.scoreboard.rows.clone(),
            matched.arena.as_ref().map(|arena| arena.snapshot.clone()),
            matched.world_states(),
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
        snapshot.arena = values.6.clone();
        if let Some(unit) = snapshot.arena.as_ref().and_then(|arena| {
            arena
                .units
                .iter()
                .find(|unit| unit.entity == player.to_bits())
        }) {
            snapshot.team = unit.team;
        }
        snapshot.world_states = values.7.clone();
        if snapshot.battleground_id == 894 {
            debug!(player = ?player, match_id, score_a = snapshot.score_a, score_b = snapshot.score_b, "Seething snapshot sent");
        }
        super::pvp_invites::send_response(
            world,
            connection,
            player,
            Ok("battleground state updated".into()),
        );
    }
}
