//! Concrete TC a352b1fa Player::RewardHonor/AddHonorXP and KillRewarder world-kill path.
//! HONOR_LEVEL_XP_CURVE: keep lifetime XP; never use TC's self-declared8800 guess.
use bevy::prelude::*;
use lightyear::prelude::MessageSender;
use shared::components::{CurrentPvpFaction, Health, Level, Player, Position, UnitScale};
use shared::protocol::{PvpChannel, PvpStateUpdate};

use super::{PlayerPvpState, pvp_support};
use crate::auras::ActiveAuras;
use crate::maps::InMap;
use crate::persistence::{self, ClientId, GameDatabase};

const MIN_HONOR_PLAYER_LEVEL: u8 = 10;
const NO_PVP_CREDIT: u32 = 159;
const HONOR_GAIN_FROM_SOURCE: u32 = 212;
const HONOR_GAIN: u32 = 491;
const INACTIVE: u32 = 43681;
const PLAYER_COMBAT_REACH: f32 = 1.5;

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
        crate::achievement::record_player_event(
            world,
            player,
            crate::achievement::CriteriaEvent::EarnHonorableKill,
        );
        let amount = calculate_kill_xp(world, player, level, victim_level, recipients.len());
        award_lifetime_xp(world, player, amount);
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

fn award_lifetime_xp(world: &mut World, player: Entity, amount: u32) {
    if amount == 0 {
        return;
    }
    let Some(mut state) = world.get_mut::<PlayerPvpState>(player) else {
        return;
    };
    state.progress.lifetime_honor_xp += u64::from(amount);
    let progress = state.progress.clone();
    let update = PvpStateUpdate {
        snapshot: Some(pvp_support::build_pvp_snapshot(&state)),
        message: None,
        error: None,
    };
    let character = world
        .get::<ClientId>(player)
        .expect("reward recipient identity")
        .0;
    let db = world.resource::<GameDatabase>();
    persistence::save_pvp(&db.db, character, &progress);
    if let Some(connection) = crate::spell_cast::owner_connection(world, player)
        && let Some(mut sender) = world.get_mut::<MessageSender<PvpStateUpdate>>(connection)
    {
        sender.send::<PvpChannel>(update);
    }
}
