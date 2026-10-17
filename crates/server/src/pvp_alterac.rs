//! Alterac Valley and Isle of Conquest spawn-group banners, normal Opening (Lock 1479,
//! spell 21651) casts and authoritative creature and player deaths.
use super::{InvitedBattleground, PlayerPvpState, PvpServerState};
use crate::game_objects::{GameObjectBounds, GameObjectSpawns, wow_position};
use crate::maps::InMap;
use crate::spell_cast::CastFailure;
use bevy::prelude::*;
use shared::battleground::{
    ALTERAC_OPENING_SPELL as OPENING_SPELL, AlteracBattle, BgInstance, BgPhase, BgTeam, IsleBattle,
};
use shared::components::{Health, Npc, Position};
use shared::protocol::GameObjectInfo;
use std::collections::BTreeSet;

#[derive(Component)]
struct AlteracBanner;

/// TC `DATA_DEFENDER_TIER_HORDE` / `DATA_DEFENDER_TIER_ALLIANCE`.
const DEFENDER_TIER_DATA: [u32; 2] = [1, 2];

/// TC `getDefenderTierForTeam`: `_teamQuestStatus` starts at 0 and only AV quest
/// turn-ins raise it. Those quests are not implemented, so both teams keep
/// `BG_AV_DEFENDER_TIER_DEFENDER` (0).
fn defender_tier(_data: u32) -> u32 {
    0
}

/// The source rules of a banner battleground whose nodes TDB spawn groups present.
#[derive(Debug)]
pub(super) enum BannerBattle {
    Alterac(AlteracBattle),
    Isle(IsleBattle),
}

impl BannerBattle {
    pub fn interact(
        &mut self,
        entry: u32,
        team: BgTeam,
        instance: &BgInstance,
    ) -> Result<u32, &'static str> {
        match self {
            Self::Alterac(battle) => battle.interact(entry, team, instance),
            Self::Isle(battle) => battle.interact(entry, team, instance),
        }
    }

    pub fn check_interact(
        &self,
        entry: u32,
        team: BgTeam,
        instance: &BgInstance,
    ) -> Result<(), &'static str> {
        match self {
            Self::Alterac(battle) => battle.check_interact(entry, team, instance),
            Self::Isle(battle) => battle.check_interact(entry, team, instance),
        }
    }

    pub fn advance(&mut self, delta_ms: u64, instance: &mut BgInstance) {
        match self {
            Self::Alterac(battle) => battle.advance(delta_ms, instance),
            Self::Isle(battle) => battle.advance(delta_ms, instance),
        }
    }

    pub fn npc_killed(
        &mut self,
        entry: u32,
        killer: Option<BgTeam>,
        instance: &mut BgInstance,
    ) -> bool {
        match self {
            Self::Alterac(battle) => battle.npc_killed(entry, killer, instance),
            Self::Isle(battle) => battle.npc_killed(entry, instance),
        }
    }

    pub fn world_states(&self, instance: &BgInstance) -> Vec<(u32, i32)> {
        match self {
            Self::Alterac(battle) => battle.world_states(instance),
            Self::Isle(battle) => battle.world_states(instance),
        }
    }
}

#[derive(Debug)]
pub(super) struct BannerRuntime {
    pub battle: BannerBattle,
    banners: BTreeSet<u32>,
}

impl BannerRuntime {
    pub fn from_source(world: &World, map_id: u32) -> Result<Self, String> {
        let catalog = world
            .get_resource::<GameObjectSpawns>()
            .ok_or("battleground source objects missing")?;
        let world_data = world
            .get_resource::<crate::world_data::WorldData>()
            .ok_or("battleground creature catalog missing")?;
        let (battle, banners) = match map_id {
            30 => (
                BannerBattle::Alterac(AlteracBattle::default()),
                shared::battleground::alterac_banner_entries(),
            ),
            628 => (
                BannerBattle::Isle(IsleBattle::default()),
                shared::battleground::isle_banner_entries(),
            ),
            other => return Err(format!("map {other} has no banner battle")),
        };
        let banners: BTreeSet<u32> = banners.into_iter().collect();
        for &entry in &banners {
            // Every banner variant must be an authored spawn its TDB spawn group gates.
            catalog
                .0
                .iter()
                .find(|spawn| {
                    spawn.map_id == map_id
                        && spawn.info.entry == entry
                        && world_data.spawn_groups.is_object_member(spawn.guid)
                })
                .ok_or_else(|| format!("map {map_id} source banner {entry} missing"))?;
        }
        let data_ids = world_data.spawn_groups.instance_data_ids();
        if let Some(unknown) = data_ids.iter().find(|id| !DEFENDER_TIER_DATA.contains(id)) {
            return Err(format!(
                "battleground spawn group reads unknown instance data {unknown}"
            ));
        }
        Ok(Self { battle, banners })
    }

    /// TDB spawn group creatures and objects (banner variants included) for the battle's
    /// world states; the spawned banners accept the normal Opening.
    pub fn synchronize(&self, world: &mut World, map: InMap, states: &[(u32, i32)]) {
        crate::spawn_groups::synchronize(world, map, states, defender_tier);
        super::isle_siege::initialize_gates(world, map);
        let mut objects =
            world.query_filtered::<(Entity, &GameObjectInfo, &InMap), Without<AlteracBanner>>();
        let unmarked: Vec<_> = objects
            .iter(world)
            .filter(|(_, info, copy)| **copy == map && self.banners.contains(&info.entry))
            .map(|(entity, _, _)| entity)
            .collect();
        for banner in unmarked {
            world.entity_mut(banner).insert(AlteracBanner);
        }
    }
}

pub(crate) fn is_banner(world: &World, object: Entity) -> bool {
    world.get::<AlteracBanner>(object).is_some()
}

fn read_context(
    world: &World,
    player: Entity,
    object: Entity,
) -> Result<(BgTeam, u64, &InvitedBattleground), CastFailure> {
    let snapshot = world
        .get::<PlayerPvpState>(player)
        .and_then(|state| state.battleground.as_ref())
        .ok_or(CastFailure::InvalidTarget)?;
    let character = world
        .get::<crate::persistence::ClientId>(player)
        .ok_or(CastFailure::InvalidTarget)?
        .0;
    let state = world.resource::<PvpServerState>();
    let matched = state
        .battleground_matches
        .get(&snapshot.match_id)
        .ok_or(CastFailure::InvalidTarget)?;
    let map = matched.map.ok_or(CastFailure::InvalidTarget)?;
    if matched.instance.phase != BgPhase::InProgress
        || !matched.accepted.contains(&character)
        || world.get::<InMap>(player) != Some(&map)
        || world.get::<InMap>(object) != Some(&map)
        || world
            .get::<Health>(player)
            .is_none_or(|health| health.current <= 0.0)
    {
        return Err(CastFailure::InvalidTarget);
    }
    Ok((snapshot.team, character, matched))
}

pub(crate) fn check_banner_target(
    world: &World,
    player: Entity,
    object: Entity,
) -> Result<(), CastFailure> {
    if !is_banner(world, object) {
        return Err(CastFailure::InvalidTarget);
    }
    let (team, _, matched) = read_context(world, player, object)?;
    let entry = world
        .get::<GameObjectInfo>(object)
        .ok_or(CastFailure::InvalidTarget)?
        .entry;
    matched
        .alterac
        .as_ref()
        .ok_or(CastFailure::InvalidTarget)?
        .battle
        .check_interact(entry, team, &matched.instance)
        .map_err(|_| CastFailure::InvalidTarget)?;
    check_reach(world, player, object)
}

fn check_reach(world: &World, player: Entity, object: Entity) -> Result<(), CastFailure> {
    let at = world
        .get::<Position>(object)
        .ok_or(CastFailure::InvalidTarget)?;
    let from = world
        .get::<Position>(player)
        .ok_or(CastFailure::InvalidTarget)?;
    let bounds = world
        .get::<GameObjectBounds>(object)
        .ok_or(CastFailure::InvalidTarget)?;
    if !bounds.reaches(
        wow_position(at),
        wow_position(from),
        crate::game_objects::GAME_OBJECT_INTERACT_DISTANCE,
    ) {
        return Err(CastFailure::OutOfRange);
    }
    Ok(())
}

pub(super) fn begin_opening(
    world: &mut World,
    player: Entity,
    object: Entity,
) -> Result<String, String> {
    check_banner_target(world, player, object)
        .map_err(|error| format!("Alterac Opening refused: {error:?}"))?;
    crate::spell_cast::cast_granted(world, player, OPENING_SPELL, Some(object))
        .map_err(|error| format!("Alterac Opening cast refused: {error:?}"))?;
    Ok("Opening battleground banner".into())
}

pub(crate) fn complete_opening(world: &mut World, player: Entity, object: Entity) {
    if let Err(error) = check_banner_target(world, player, object) {
        error!("Alterac Opening completion refused: {error:?}");
        return;
    }
    let entry = world
        .get::<GameObjectInfo>(object)
        .expect("validated banner")
        .entry;
    let match_id = world
        .get::<PlayerPvpState>(player)
        .and_then(|state| state.battleground.as_ref())
        .expect("validated participant")
        .match_id;
    let result = world.resource_scope(|world, mut state: Mut<PvpServerState>| {
        let (team, character, matched) =
            super::pvp_objective::active_match(world, &mut state, player, object)?;
        let alterac = matched.alterac.as_mut().ok_or("Alterac runtime missing")?;
        let stat = alterac.battle.interact(entry, team, &matched.instance)?;
        let states = alterac.battle.world_states(&matched.instance);
        alterac.synchronize(world, matched.map.expect("active match map"), &states);
        super::pvp_objective::credit(&mut matched.scoreboard, character, stat, 1);
        Ok::<_, String>(match_id)
    });
    match result {
        Ok(match_id) => super::pvp_objective::send_match_snapshot(world, match_id),
        Err(error) => error!("Alterac Opening completion refused: {error}"),
    }
}

pub(super) fn update_alterac(
    world: &mut World,
    matched: &mut InvitedBattleground,
    map: InMap,
    delta_ms: u64,
) -> bool {
    let alterac = matched.alterac.as_mut().expect("Alterac runtime");
    let before = alterac.battle.world_states(&matched.instance);
    alterac.battle.advance(delta_ms, &mut matched.instance);
    let changed = before != alterac.battle.world_states(&matched.instance);
    if changed {
        alterac.synchronize(world, map, &alterac.battle.world_states(&matched.instance));
    }
    changed
}

pub(crate) fn record_creature_death(world: &mut World, creature: Entity, killer: Option<Entity>) {
    if world.get_resource::<PvpServerState>().is_none() {
        return;
    }
    let Some(map) = world.get::<InMap>(creature).copied() else {
        return;
    };
    let Some(entry) = world.get::<Npc>(creature).map(|npc| npc.template_id) else {
        return;
    };
    let team = killer.and_then(|player| accepted_team(world, player, map));
    let changed = world.resource_scope(|_, mut state: Mut<PvpServerState>| {
        state
            .battleground_matches
            .iter_mut()
            .filter_map(|(id, matched)| {
                if matched.map != Some(map) {
                    return None;
                }
                let alterac = matched.alterac.as_mut()?;
                alterac
                    .battle
                    .npc_killed(entry, team, &mut matched.instance)
                    .then_some(*id)
            })
            .collect::<Vec<_>>()
    });
    for id in changed {
        if map.map_id == 628 {
            super::honor::award_isle_commander_honor(world, id);
        } else if map.map_id == 30 {
            super::honor::award_alterac_npc_honor(world, id, entry);
        }
        super::pvp_objective::send_match_snapshot(world, id);
    }
}

fn accepted_team(world: &World, player: Entity, map: InMap) -> Option<BgTeam> {
    if world.get::<InMap>(player) != Some(&map) {
        return None;
    }
    let snapshot = world.get::<PlayerPvpState>(player)?.battleground.as_ref()?;
    let character = world.get::<crate::persistence::ClientId>(player)?.0;
    let matched = world
        .resource::<PvpServerState>()
        .battleground_matches
        .get(&snapshot.match_id)?;
    (matched.map == Some(map) && matched.accepted.contains(&character)).then_some(snapshot.team)
}

pub(crate) fn record_player_death(world: &mut World, player: Entity) {
    if world.get_resource::<PvpServerState>().is_none() {
        return;
    }
    let Some(map) = world.get::<InMap>(player).copied() else {
        return;
    };
    let Some(team) = accepted_team(world, player, map) else {
        return;
    };
    let character = world
        .get::<crate::persistence::ClientId>(player)
        .expect("accepted player")
        .0;
    let match_id = world
        .get::<PlayerPvpState>(player)
        .and_then(|state| state.battleground.as_ref())
        .expect("accepted player")
        .match_id;
    let changed = world.resource_scope(|_, mut state: Mut<PvpServerState>| {
        apply_player_loss(&mut state, match_id, character, team)
    });
    if changed {
        super::pvp_objective::send_match_snapshot(world, match_id);
    }
}

fn apply_player_loss(
    state: &mut PvpServerState,
    match_id: u64,
    character: u64,
    team: BgTeam,
) -> bool {
    let matched = state
        .battleground_matches
        .get_mut(&match_id)
        .expect("accepted match");
    if matched.alterac.is_none() || matched.instance.phase != BgPhase::InProgress {
        return false;
    }
    matched.instance.deduct_reinforcements(team, 1);
    if let Some(row) = matched
        .scoreboard
        .rows
        .iter_mut()
        .find(|row| row.character_id == character)
    {
        row.deaths += 1;
    }
    true
}
