//! Arena queue and source-map selection. DB2 12.1.0.69933 / TDB1210.
use bevy::prelude::*;
use lightyear::prelude::*;
use rusqlite::Connection;
use shared::battleground::{BgInstance, BgMatch, BgObjective, BgTemplate, LevelBracket};
use shared::components::Level;
use shared::protocol::{ArenaMatchSnapshot, ArenaQueueMode, BattlegroundSnapshot, QueueForArena};

pub(super) use super::arena_lifecycle::update_matches;
// BattlemasterList6 (All Arenas), build12.1.0.69933.
const SKIRMISH_MIN_LEVEL: u8 = 20;
use super::{InvitedBattleground, PlayerPvpQueue, PlayerPvpState, PvpServerState};
use crate::auth::{Authenticated, InWorld};
use crate::networking::PlayerLink;

#[derive(Clone, Debug)]
pub(super) struct SourceMap {
    pub id: u32,
    pub map_id: u32,
    pub name: String,
    pub starts: [u32; 2],
    pub levels: LevelBracket,
    pub weight: u32,
}
#[derive(Resource, Default)]
pub(crate) struct Catalog(pub(super) Vec<SourceMap>);
impl Catalog {
    pub(crate) fn contains_map(&self, map_id: u32) -> bool {
        self.0.iter().any(|source| source.map_id == map_id)
    }

    pub(crate) fn load(db: &Connection) -> rusqlite::Result<Self> {
        let mut query = db.prepare("SELECT battlemaster_id,map_id,name,start_a,start_b,min_level,max_level,weight FROM arena_map ORDER BY battlemaster_id")?;
        Ok(Self(
            query
                .query_map([], |row| {
                    Ok(SourceMap {
                        id: row.get(0)?,
                        map_id: row.get(1)?,
                        name: row.get(2)?,
                        starts: [row.get(3)?, row.get(4)?],
                        levels: LevelBracket::new(row.get(5)?, row.get(6)?),
                        weight: row.get(7)?,
                    })
                })?
                .collect::<rusqlite::Result<_>>()?,
        ))
    }
}
#[derive(Debug)]
pub(super) struct MatchState {
    pub source: SourceMap,
    pub mode: ArenaQueueMode,
    pub preparation_started: Option<f64>,
    pub started: Option<f64>,
    pub snapshot: ArenaMatchSnapshot,
    pub rated: Option<super::arena_rating::RatedMatch>,
    pub score_dirty: bool,
    pub shuffle: Option<super::arena_shuffle::Shuffle>,
}
impl MatchState {
    pub fn template(&self) -> BgTemplate {
        BgTemplate {
            id: self.source.id,
            name: self.source.name.clone(),
            map_id: self.source.map_id,
            team_size: team_size(self.mode).expect("validated arena mode"),
            min_players_per_team: team_size(self.mode).expect("validated arena mode"),
            objective: BgObjective::Elimination,
            max_duration_secs: 0,
            win_score: 0,
            starting_reinforcements: 0,
            bracket: self.source.levels,
        }
    }
}
pub(super) fn team_size(mode: ArenaQueueMode) -> Option<u8> {
    match mode {
        ArenaQueueMode::Skirmish2v2 | ArenaQueueMode::Rated2v2 => Some(2),
        ArenaQueueMode::Skirmish3v3 | ArenaQueueMode::Rated3v3 => Some(3),
        ArenaQueueMode::SoloShuffle => Some(3),
    }
}
pub(super) fn queue_player(
    player: &mut PlayerPvpState,
    state: &mut PvpServerState,
    catalog: &Catalog,
    id: u64,
    level: u8,
    mode: ArenaQueueMode,
) -> Result<String, String> {
    if matches!(mode, ArenaQueueMode::Rated2v2 | ArenaQueueMode::Rated3v3) {
        return Err(
            "rated arena requires a full group and its leader's rated queue request".into(),
        );
    }
    if mode == ArenaQueueMode::SoloShuffle && !(60..=90).contains(&level) {
        return Err("Solo Shuffle source level bracket is60–90".into());
    }
    if player.battleground.is_some() || player.queue.is_some() {
        return Err("already queued or invited to PvP".into());
    }
    if level < SKIRMISH_MIN_LEVEL
        || !catalog
            .0
            .iter()
            .any(|map| map.levels.contains(level) && map.weight > 0)
    {
        return Err("no source-backed arena supports this level".into());
    }
    if state
        .arena_queues
        .values()
        .any(|queue| queue.iter().any(|(other, _)| *other == id))
    {
        return Err("already queued for arena".into());
    }
    state
        .arena_queues
        .entry(mode)
        .or_default()
        .push((id, level));
    player.queue = Some(PlayerPvpQueue::Arena { mode });
    Ok("queued for arena skirmish".into())
}
pub(super) fn form_match(
    state: &mut PvpServerState,
    catalog: &Catalog,
    mode: ArenaQueueMode,
) -> Option<u64> {
    let size = usize::from(team_size(mode)?);
    let queue = state.arena_queues.get(&mode)?;
    if queue.len() < size * 2 {
        return None;
    }
    let participants: Vec<_> = queue[..size * 2].iter().map(|(id, _)| *id).collect();
    let levels: Vec<_> = queue[..size * 2].iter().map(|(_, level)| *level).collect();
    let teams = [
        participants.iter().step_by(2).copied().collect(),
        participants.iter().skip(1).step_by(2).copied().collect(),
    ];
    let id = create_match(state, catalog, mode, teams, &levels)?;
    state
        .arena_queues
        .get_mut(&mode)
        .expect("queued mode")
        .drain(..size * 2);
    Some(id)
}
pub(super) fn create_match(
    state: &mut PvpServerState,
    catalog: &Catalog,
    mode: ArenaQueueMode,
    teams: [Vec<u64>; 2],
    levels: &[u8],
) -> Option<u64> {
    let maps: Vec<_> = catalog
        .0
        .iter()
        .filter(|map| map.weight > 0 && levels.iter().all(|level| map.levels.contains(*level)))
        .collect();
    let weight: u32 = maps.iter().map(|map| map.weight).sum();
    if weight == 0 {
        return None;
    }
    let mut roll = rand::Rng::gen_range(&mut rand::thread_rng(), 0..weight);
    let source = maps
        .into_iter()
        .find(|map| {
            if roll < map.weight {
                true
            } else {
                roll -= map.weight;
                false
            }
        })?
        .clone();
    let arena = MatchState {
        source,
        mode,
        preparation_started: None,
        started: None,
        rated: None,
        score_dirty: false,
        shuffle: None,
        snapshot: ArenaMatchSnapshot {
            mode,
            preparation_remaining_secs: None,
            shadow_sight_available: false,
            units: Vec::new(),
            rating_changes: Vec::new(),
            shuffle: None,
        },
    };
    let template = arena.template();
    let matched = BgMatch {
        bg_id: template.id,
        bracket: template.bracket,
        team_a: teams[0].clone(),
        team_b: teams[1].clone(),
    };
    state.next_battleground_match += 1;
    let id = state.next_battleground_match;
    state.battleground_matches.insert(
        id,
        InvitedBattleground {
            instance: BgInstance::from_match(&matched, &template),
            preparation: Default::default(),
            accepted: Default::default(),
            map: None,
            origins: Default::default(),
            original_queues: Default::default(),
            flags: Default::default(),
            nodes: None,
            eye: None,
            kotmogu: None,
            silvershard: None,
            deephaul: None,
            seething: None,
            alterac: None,
            scoreboard: Default::default(),
            flag_positions: [None, None],
            arena: Some(arena),
        },
    );
    Some(id)
}
pub(super) fn receive_queue_requests(world: &mut World) {
    let mut receivers = world.query_filtered::<(
        Entity,
        &PlayerLink,
        &InWorld,
        &mut MessageReceiver<QueueForArena>,
    ), With<Authenticated>>();
    let requests: Vec<_> = receivers
        .iter_mut(world)
        .flat_map(|(connection, link, owner, mut receiver)| {
            receiver
                .receive()
                .map(|request| (connection, link.0, owner.character_id, request.mode))
                .collect::<Vec<_>>()
        })
        .collect();
    for (connection, player, id, mode) in requests {
        let level = world.get::<Level>(player).map(|level| level.0);
        let result = world.resource_scope(|world, mut state: Mut<PvpServerState>| {
            world.resource_scope(|world, catalog: Mut<Catalog>| {
                if mode == ArenaQueueMode::SoloShuffle {
                    super::arena_shuffle::validate_entrant(world, player, id)?;
                }
                let mut actor = world
                    .get_mut::<PlayerPvpState>(player)
                    .ok_or("PvP state unavailable")?;
                queue_player(
                    &mut actor,
                    &mut state,
                    &catalog,
                    id,
                    level.ok_or("player level unavailable")?,
                    mode,
                )
            })
        });
        super::pvp_invites::send_response(world, connection, player, result.clone());
        if result.is_ok() {
            while let Some(match_id) =
                world.resource_scope(|world, mut state: Mut<PvpServerState>| {
                    world.resource_scope(|world, catalog: Mut<Catalog>| {
                        if mode == ArenaQueueMode::SoloShuffle {
                            super::arena_shuffle::form_match(world, &mut state, &catalog)
                        } else {
                            form_match(&mut state, &catalog, mode)
                        }
                    })
                })
            {
                publish_invitation(world, match_id);
            }
        }
    }
}
pub(super) fn publish_invitation(world: &mut World, match_id: u64) {
    let snapshots = {
        let matched = &world.resource::<PvpServerState>().battleground_matches[&match_id];
        let arena = matched.arena.as_ref().expect("formed arena");
        [
            (
                shared::battleground::BgTeam::A,
                &matched.instance.team_a_players,
            ),
            (
                shared::battleground::BgTeam::B,
                &matched.instance.team_b_players,
            ),
        ]
        .into_iter()
        .flat_map(|(team, ids)| {
            ids.iter().map(move |id| {
                (
                    *id,
                    BattlegroundSnapshot {
                        match_id,
                        battleground_id: arena.source.id,
                        name: arena.source.name.clone(),
                        map_id: arena.source.map_id,
                        team,
                        phase: matched.instance.phase,
                        accepted: false,
                        invitation_time_left_secs: None,
                        score_a: 0,
                        score_b: 0,
                        flags: Default::default(),
                        world_states: Vec::new(),
                        outcome: None,
                        players: Vec::new(),
                        honor_reward: None,
                        arena: Some(arena.snapshot.clone()),
                    },
                )
            })
        })
        .collect::<Vec<_>>()
    };
    let mut query = world.query_filtered::<(Entity, &PlayerLink, &InWorld), With<Authenticated>>();
    let connections: Vec<_> = query
        .iter(world)
        .map(|(entity, link, owner)| (entity, link.0, owner.character_id))
        .collect();
    for (id, snapshot) in snapshots {
        if let Some((connection, player, _)) = connections.iter().find(|(_, _, owner)| *owner == id)
        {
            let mut actor = world
                .get_mut::<PlayerPvpState>(*player)
                .expect("queued player");
            actor.queue = None;
            actor.battleground = Some(snapshot);
            super::pvp_invites::send_response(
                world,
                *connection,
                *player,
                Ok("arena invitation ready".into()),
            );
        }
    }
}
