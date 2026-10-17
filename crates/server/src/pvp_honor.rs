//! Concrete TC a352b1fa Player::RewardHonor/AddHonorXP and KillRewarder world-kill path.
//! HONOR_LEVEL_XP_CURVE: keep lifetime XP; never use TC's self-declared8800 guess.
use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use shared::battleground::{BgOutcome, BgTeam};
use shared::components::{CurrentPvpFaction, Health, Level, Player, Position, UnitScale};
use shared::currency::{CurrencyWallet, HONOR_CURRENCY_ID, currency_by_id};
use shared::protocol::{CurrencyChannel, CurrencyStateUpdate, PvpChannel, PvpStateUpdate};

use super::{PlayerPvpState, PvpServerState, pvp_support};
use crate::auras::ActiveAuras;
use crate::auth::{Authenticated, InWorld};
use crate::maps::InMap;
use crate::networking::PlayerLink;
use crate::persistence::{self, ClientId, GameDatabase};

const MIN_HONOR_PLAYER_LEVEL: u8 = 10;
const NO_PVP_CREDIT: u32 = 159;
const HONOR_GAIN_FROM_SOURCE: u32 = 212;
const HONOR_GAIN: u32 = 491;
const INACTIVE: u32 = 43681;
const PLAYER_COMBAT_REACH: f32 = 1.5;
// TC Isle HonorRewards193–197 / OnUnitKilled383–394, not a bonus-kill multiplier.
const ISLE_COMMANDER_HONOR: u32 = 500;
// TC Isle OnUpdate324–334: each source resource reinforcement also grants12.
const ISLE_RESOURCE_HONOR: u32 = 12;

pub(crate) fn reward_world_player_kill(world: &mut World, killer: Entity, victim: Entity) {
    let Some(map) = world_kill_map(world, killer, victim) else {
        return;
    };
    let Some(victim_level) = world.get::<Level>(victim).map(|level| level.0) else {
        return;
    };
    let recipients = read_world_kill_recipients(world, killer, victim, map);
    for &player in &recipients {
        let Some(level) = eligible_kill_level(world, player, victim, victim_level) else {
            continue;
        };
        // TC RewardHonor updates35 before AddHonorXP's minimum-level/zero-XP checks.
        crate::achievement::record_world_honorable_kill(world, player, victim);
        let amount = calculate_kill_xp(world, player, level, victim_level, recipients.len());
        award_honor(world, player, amount);
    }
}

/// BG_END_BONUS_ONCE_PER_MATCH normalizes TC's per-player OnEnd placement.
/// Arathi OnEnd574–585 / Eye OnEnd437–448: winner1, then everyone1.
pub(super) fn award_battleground_completion(
    world: &mut World,
    player: Entity,
    map_id: u32,
    bracket: shared::battleground::LevelBracket,
    won: bool,
) {
    if !matches!(map_id, 2107 | 566) {
        return; // Other per-map callbacks remain unimplemented, not generic rewards.
    }
    let bonus = (f32::from(bracket.max_level.min(80)) * 1.55).ceil() as u32;
    if won {
        award_battleground_honor(world, player, bonus);
    }
    award_battleground_honor(world, player, bonus);
}

fn award_battleground_honor(world: &mut World, player: Entity, amount: u32) {
    let Some(level) = world.get::<Level>(player).map(|level| level.0) else {
        return;
    };
    let inactive = world
        .get::<ActiveAuras>(player)
        .is_some_and(|auras| auras.has_spell(INACTIVE));
    if level < MIN_HONOR_PLAYER_LEVEL || inactive {
        return;
    }
    // RewardHonor(nullptr,...) does not apply victim-only group/aura/rest modifiers.
    award_honor(world, player, amount);
}

/// Called only after the source Isle battle accepts an actual commander death.
pub(super) fn award_isle_commander_honor(world: &mut World, match_id: u64) {
    let winner = match world.resource::<PvpServerState>().battleground_matches[&match_id]
        .instance
        .outcome
    {
        Some(BgOutcome::Winner(team)) => team,
        _ => panic!("accepted Isle commander death must declare a winner"),
    };
    award_isle_team_honor(world, match_id, winner, ISLE_COMMANDER_HONOR);
}

fn award_isle_team_honor(world: &mut World, match_id: u64, team: BgTeam, amount: u32) {
    let (map, roster) = read_isle_team_roster(world, match_id, team);
    let recipients: Vec<_> = world
        .query_filtered::<(&PlayerLink, &InWorld), With<Authenticated>>()
        .iter(world)
        .filter(|(link, owner)| {
            roster.contains(&owner.character_id) && world.get::<InMap>(link.0) == Some(&map)
        })
        .map(|(link, _)| link.0)
        .collect();
    for player in recipients {
        award_battleground_honor(world, player, amount);
    }
}

fn read_isle_team_roster(world: &World, match_id: u64, team: BgTeam) -> (InMap, Vec<u64>) {
    let state = world.resource::<PvpServerState>();
    let matched = &state.battleground_matches[&match_id];
    let members = match team {
        BgTeam::A => &matched.instance.team_a_players,
        BgTeam::B => &matched.instance.team_b_players,
    };
    let roster = members
        .iter()
        .filter(|character| matched.accepted.contains(character))
        .copied()
        .collect();
    (matched.map.expect("accepted Isle match has a map"), roster)
}

/// A/B counters around the exclusive objective advance, not across combat/death systems.
pub(super) fn read_isle_resource_counters(world: &World) -> Vec<(u64, [u32; 2])> {
    world
        .resource::<PvpServerState>()
        .battleground_matches
        .iter()
        .filter(|(_, matched)| matched.map.is_some_and(|map| map.map_id == 628))
        .map(|(id, matched)| {
            (
                *id,
                [
                    matched.instance.score_a.reinforcements,
                    matched.instance.score_b.reinforcements,
                ],
            )
        })
        .collect()
}

pub(super) fn award_isle_resource_gains(world: &mut World, counters: Vec<(u64, [u32; 2])>) {
    for (match_id, before) in counters {
        let after = {
            let state = world.resource::<PvpServerState>();
            let instance = &state.battleground_matches[&match_id].instance;
            [
                instance.score_a.reinforcements,
                instance.score_b.reinforcements,
            ]
        };
        award_isle_resource_team(world, match_id, BgTeam::A, before[0], after[0]);
        award_isle_resource_team(world, match_id, BgTeam::B, before[1], after[1]);
    }
}

fn award_isle_resource_team(
    world: &mut World,
    match_id: u64,
    team: BgTeam,
    before: u32,
    after: u32,
) {
    let grants = after
        .checked_sub(before)
        .expect("Isle objective advance only adds resource reinforcements");
    for _ in 0..grants {
        award_isle_team_honor(world, match_id, team, ISLE_RESOURCE_HONOR);
    }
}

fn world_kill_map(world: &World, killer: Entity, victim: Entity) -> Option<InMap> {
    if killer == victim || world.get::<Player>(victim).is_none() {
        return None;
    }
    let map = *world.get::<InMap>(killer)?;
    let victim_refuses_credit = world
        .get::<ActiveAuras>(victim)
        .is_some_and(|auras| auras.has_type(NO_PVP_CREDIT));
    if world.get::<InMap>(victim) != Some(&map)
        || super::read_player_battleground(world, killer).is_some()
        || victim_refuses_credit
    {
        return None;
    }
    Some(map)
}

fn read_world_kill_recipients(
    world: &mut World,
    killer: Entity,
    victim: Entity,
    map: InMap,
) -> Vec<Entity> {
    let Some(character) = world.get::<ClientId>(killer).map(|id| id.0) else {
        return vec![];
    };
    let members = world
        .get_resource::<crate::group::GroupRegistry>()
        .map(|groups| groups.group_members(character))
        .unwrap_or_default();
    world
        .query::<(Entity, &ClientId, &Player)>()
        .iter(world)
        .filter(|(entity, id, _)| *entity == killer || members.contains(&id.0))
        .filter(|(entity, _, _)| world.get::<InMap>(*entity) == Some(&map))
        .filter(|(entity, _, _)| *entity == killer || in_reward_range(world, *entity, victim))
        .filter(|(entity, _, _)| {
            world
                .get::<Health>(*entity)
                .is_some_and(|health| health.current > 0.0)
        })
        .map(|(entity, _, _)| entity)
        .collect()
}

fn eligible_kill_level(
    world: &World,
    player: Entity,
    victim: Entity,
    victim_level: u8,
) -> Option<u8> {
    let killer_team = world.get::<CurrentPvpFaction>(player)?;
    let victim_team = world.get::<CurrentPvpFaction>(victim)?;
    if killer_team == victim_team {
        return None;
    }
    let level = world.get::<Level>(player)?.0;
    let inactive = world
        .get::<ActiveAuras>(player)
        .is_some_and(|aura| aura.has_spell(INACTIVE));
    (!inactive && victim_level > gray_level(level)).then_some(level)
}

fn calculate_kill_xp(
    world: &World,
    player: Entity,
    level: u8,
    victim_level: u8,
    group_size: usize,
) -> u32 {
    if level < MIN_HONOR_PLAYER_LEVEL {
        return 0;
    }
    let gray = gray_level(level);
    let base =
        (f32::from(level) * 1.55 * f32::from(victim_level - gray) / f32::from(level - gray)).ceil();
    let divided = base / group_size as f32;
    let source_bonus = maximum_honor_modifier(world, player, HONOR_GAIN_FROM_SOURCE, true);
    let general_bonus = maximum_honor_modifier(world, player, HONOR_GAIN, false);
    (divided * (1.0 + source_bonus / 100.0) * (1.0 + general_bonus / 100.0)) as u32
}

fn in_reward_range(world: &World, player: Entity, victim: Entity) -> bool {
    let (Some(position), Some(target)) =
        (world.get::<Position>(player), world.get::<Position>(victim))
    else {
        return false;
    };
    let reach =
        |entity| PLAYER_COMBAT_REACH * world.get::<UnitScale>(entity).map_or(1.0, |scale| scale.0);
    (crate::creature_tap::distance(position, target) - reach(player) - reach(victim)).max(0.0)
        <= crate::creature_tap::GROUP_REWARD_DISTANCE
}

fn gray_level(level: u8) -> u8 {
    if level < 7 {
        0
    } else if level < 35 {
        level - 6 - (level / 5).saturating_sub(2)
    } else {
        level - 10
    }
}

fn maximum_honor_modifier(world: &World, player: Entity, kind: u32, kill_only: bool) -> f32 {
    let Some(auras) = world.get::<ActiveAuras>(player) else {
        return 0.0;
    };
    auras
        .iter()
        .flat_map(|aura| {
            aura.effects
                .iter()
                .filter(move |effect| {
                    effect.aura_type == kind && (!kill_only || effect.misc_values[0] & 1 != 0)
                })
                .map(move |effect| effect.amount * aura.stack_multiplier())
        })
        .fold(0.0, f32::max)
}

/// HONOR_WALLET_MIRRORS_HONOR_XP is our binding, not TC wallet behavior.
/// Callers supply the final TC-derived amount; the holding cap never reduces XP.
pub(super) fn award_honor(world: &mut World, player: Entity, amount: u32) {
    if amount == 0 || world.get::<PlayerPvpState>(player).is_none() {
        return;
    }
    let (wallet, added) = credit_honor_wallet(world, player, amount);
    let progress = {
        let mut state = world
            .get_mut::<PlayerPvpState>(player)
            .expect("honor recipient state");
        state.progress.lifetime_honor_xp += u64::from(amount);
        super::sync_progress_currencies_from_wallet(&mut state.progress, &wallet);
        state.progress.clone()
    };
    let character = world
        .get::<ClientId>(player)
        .expect("reward recipient identity")
        .0;
    let db = world.resource::<GameDatabase>();
    persistence::save_currencies(&db.db, character, &wallet);
    persistence::save_pvp(&db.db, character, &progress);
    send_honor_updates(world, player, &wallet, added);
    if added > 0 {
        crate::achievement::record_player_event(
            world,
            player,
            crate::achievement::CriteriaEvent::CurrencyGained(HONOR_CURRENCY_ID, added),
        );
    }
}

fn credit_honor_wallet(world: &mut World, player: Entity, amount: u32) -> (CurrencyWallet, u32) {
    let mut wallet = world
        .get_mut::<crate::currency::PlayerCurrencies>(player)
        .expect("honor recipient currency wallet");
    let definition = currency_by_id(HONOR_CURRENCY_ID).expect("source Honor1792 definition");
    let added = wallet
        .0
        .add(HONOR_CURRENCY_ID, amount, definition)
        .expect("known Honor identity must accept an authoritative award");
    (wallet.0.clone(), added)
}

fn send_honor_updates(world: &mut World, player: Entity, wallet: &CurrencyWallet, added: u32) {
    let Some(connection) = crate::spell_cast::owner_connection(world, player) else {
        return;
    };
    let snapshot = pvp_support::build_pvp_snapshot(
        world
            .get::<PlayerPvpState>(player)
            .expect("honor recipient state"),
    );
    if let Some(mut sender) = world.get_mut::<MessageSender<PvpStateUpdate>>(connection) {
        sender.send::<PvpChannel>(PvpStateUpdate {
            snapshot: Some(snapshot),
            message: None,
            error: None,
        });
    }
    if added == 0 {
        return;
    }
    if let Some(mut sender) = world.get_mut::<MessageSender<CurrencyStateUpdate>>(connection) {
        sender.send::<CurrencyChannel>(CurrencyStateUpdate {
            snapshot: Some(crate::currency::build_currency_snapshot(wallet)),
            message: Some(format!("earned {added} Honor")),
            error: None,
        });
    }
}
