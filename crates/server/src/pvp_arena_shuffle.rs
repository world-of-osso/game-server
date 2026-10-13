//! Unrated six-round realm policy. ChrSpecialization.Role: healer1, damage2;
//! BattlemasterList1065: six solo entrants, levels60–90. No invented rating.
use super::{PlayerPvpState, PvpServerState, arena};
use bevy::prelude::*;
use shared::battleground::{BgOutcome, BgPhase, BgTeam};
use shared::components::{ActiveSpec, Player};
use shared::protocol::{ArenaQueueMode, SoloShuffleSnapshot};

#[cfg(test)]
#[path = "pvp_arena_shuffle_tests.rs"]
mod tests;

const HEALER: u32 = 1;
const DAMAGE: u32 = 2;
const PAIRS: [[usize; 2]; 6] = [[0, 1], [0, 2], [0, 3], [1, 2], [1, 3], [2, 3]];

#[derive(Debug)]
pub(super) struct Shuffle {
    healers: [u64; 2],
    damage: [u64; 4],
    pub reset_pending: bool,
}

impl Shuffle {
    fn teams(&self, round: u8) -> [Vec<u64>; 2] {
        let pair = PAIRS[usize::from(round - 1)];
        let a = vec![self.healers[0], self.damage[pair[0]], self.damage[pair[1]]];
        let b = std::iter::once(self.healers[1])
            .chain(
                self.damage
                    .iter()
                    .enumerate()
                    .filter(|(index, _)| !pair.contains(index))
                    .map(|(_, id)| *id),
            )
            .collect();
        [a, b]
    }
}

fn role(world: &World, player: Entity) -> Result<u32, String> {
    let spec = world
        .get::<ActiveSpec>(player)
        .ok_or("Solo Shuffle requires an active specialization")?;
    let class = world
        .get::<Player>(player)
        .ok_or("arena player class missing")?
        .class;
    let classes = world.resource::<crate::spell_info::ClassSpells>();
    let source = classes
        .classes
        .iter()
        .find(|source| source.class_id == class)
        .and_then(|source| source.specs.iter().find(|source| source.spec_id == spec.0))
        .ok_or("active specialization has no source role")?;
    match source.role {
        HEALER | DAMAGE => Ok(source.role),
        _ => Err("Solo Shuffle requires healer or damage specialization".into()),
    }
}

pub(super) fn validate_entrant(world: &World, player: Entity, id: u64) -> Result<(), String> {
    role(world, player)?;
    if world
        .get_resource::<crate::group::GroupRegistry>()
        .is_some_and(|groups| groups.group_members(id).len() > 1)
    {
        return Err("Solo Shuffle accepts solo entrants only".into());
    }
    Ok(())
}

pub(super) fn form_match(
    world: &mut World,
    state: &mut PvpServerState,
    catalog: &arena::Catalog,
) -> Option<u64> {
    let queue = state.arena_queues.get(&ArenaQueueMode::SoloShuffle)?;
    let mut query = world.query::<(Entity, &crate::persistence::ClientId)>();
    let online: std::collections::BTreeMap<_, _> = query
        .iter(world)
        .map(|(entity, id)| (id.0, entity))
        .collect();
    let eligible: Vec<_> = queue
        .iter()
        .filter_map(|(id, level)| {
            let player = *online.get(id)?;
            validate_entrant(world, player, *id).ok()?;
            Some((*id, *level, role(world, player).ok()?))
        })
        .collect();
    let healers: Vec<_> = eligible
        .iter()
        .filter(|(_, _, role)| *role == HEALER)
        .take(2)
        .copied()
        .collect();
    let damage: Vec<_> = eligible
        .iter()
        .filter(|(_, _, role)| *role == DAMAGE)
        .take(4)
        .copied()
        .collect();
    if healers.len() != 2 || damage.len() != 4 {
        return None;
    }
    let shuffle = Shuffle {
        healers: [healers[0].0, healers[1].0],
        damage: [damage[0].0, damage[1].0, damage[2].0, damage[3].0],
        reset_pending: false,
    };
    let selected: Vec<_> = healers.iter().chain(&damage).copied().collect();
    let levels: Vec<_> = selected.iter().map(|(_, level, _)| *level).collect();
    let id = arena::create_match(
        state,
        catalog,
        ArenaQueueMode::SoloShuffle,
        shuffle.teams(1),
        &levels,
    )?;
    let arena = state.battleground_matches.get_mut(&id)?.arena.as_mut()?;
    arena.snapshot.shuffle = Some(SoloShuffleSnapshot {
        round: 1,
        total_rounds: 6,
        rounds_won: selected.iter().map(|(id, _, _)| (*id, 0)).collect(),
    });
    arena.shuffle = Some(shuffle);
    state
        .arena_queues
        .get_mut(&ArenaQueueMode::SoloShuffle)?
        .retain(|(id, _)| !selected.iter().any(|(entrant, _, _)| entrant == id));
    Some(id)
}

pub(super) fn finish_round(matched: &mut super::InvitedBattleground, winner: BgTeam) {
    let arena = matched.arena.as_mut().expect("shuffle arena");
    let snapshot = arena.snapshot.shuffle.as_mut().expect("shuffle round");
    let winners = match winner {
        BgTeam::A => &matched.instance.team_a_players,
        BgTeam::B => &matched.instance.team_b_players,
    };
    for (id, wins) in &mut snapshot.rounds_won {
        if winners.contains(id) {
            *wins += 1;
        }
    }
    if snapshot.round == snapshot.total_rounds {
        // Individual round wins are the result; final rotating teams are not match winners.
        matched.instance.end(BgOutcome::Draw);
        return;
    }
    snapshot.round += 1;
    let shuffle = arena.shuffle.as_mut().expect("shuffle roster");
    let [a, b] = shuffle.teams(snapshot.round);
    matched.instance.team_a_players = a;
    matched.instance.team_b_players = b;
    for unit in &mut arena.snapshot.units {
        unit.team = if matched.instance.team_a_players.contains(&unit.character_id) {
            BgTeam::A
        } else {
            BgTeam::B
        };
    }
    for score in &mut matched.scoreboard {
        score.team = if matched
            .instance
            .team_a_players
            .contains(&score.character_id)
        {
            BgTeam::A
        } else {
            BgTeam::B
        };
    }
    matched.instance.phase = BgPhase::Waiting;
    arena.preparation_started = None;
    arena.started = None;
    arena.snapshot.preparation_remaining_secs = Some(60);
    arena.snapshot.shadow_sight_available = false;
    shuffle.reset_pending = true;
}

pub(super) fn reset_pending_rounds(world: &mut World) {
    let resets: Vec<_> = world
        .resource_mut::<PvpServerState>()
        .battleground_matches
        .iter_mut()
        .filter_map(|(id, matched)| {
            let arena = matched.arena.as_mut()?;
            let shuffle = arena.shuffle.as_mut()?;
            if !std::mem::take(&mut shuffle.reset_pending) {
                return None;
            }
            Some((
                *id,
                matched.map.expect("started shuffle copy"),
                arena.source.starts,
                matched.instance.team_a_players.clone(),
                matched.instance.team_b_players.clone(),
            ))
        })
        .collect();
    for (id, map, starts, a, b) in resets {
        reset_round_players(world, id, map, starts, &a, &b);
        super::arena_objects::reset_map(world, map);
        super::arena_sight::prepare_map(world, map);
        super::arena_cage::reset_map(world, map);
        super::arena_dalaran::reset_map(world, map);
    }
}

fn reset_round_players(
    world: &mut World,
    match_id: u64,
    map: crate::maps::InMap,
    starts: [u32; 2],
    a: &[u64],
    b: &[u64],
) {
    let mut query = world.query::<(Entity, &crate::persistence::ClientId)>();
    let players: Vec<_> = query
        .iter(world)
        .filter_map(|(player, id)| {
            let team = if a.contains(&id.0) {
                BgTeam::A
            } else if b.contains(&id.0) {
                BgTeam::B
            } else {
                return None;
            };
            Some((player, team))
        })
        .collect();
    for (player, team) in players {
        crate::combat::combat_stop(world, player);
        crate::spell_cast::cancel_cast(world, player);
        if world
            .get::<shared::components::Health>(player)
            .is_some_and(|health| health.current <= 0.0)
        {
            crate::death::resurrect_player_fully(world, player);
        }
        super::arena_preparation::reset_entry(world, player);
        world
            .get_mut::<PlayerPvpState>(player)
            .expect("shuffle entrant")
            .battleground
            .as_mut()
            .filter(|bg| bg.match_id == match_id)
            .expect("shuffle match identity")
            .team = team;
        let start = world.resource::<crate::maps::MapStore>().safe_locs
            [&starts[if team == BgTeam::A { 0 } else { 1 }]];
        assert_eq!(start.map_id, map.map_id, "Shuffle source start map");
        let destination = start;
        assert!(
            crate::maps::teleport_to(
                world,
                player,
                destination,
                crate::maps::TeleportOptions::default()
            ),
            "same-copy Shuffle reset teleport refused"
        );
    }
}
