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

pub(crate) fn reward_world_player_kill(world: &mut World, killer: Entity, victim: Entity) {
    if killer == victim || world.get::<Player>(victim).is_none() {
        return;
    }
    let Some(map) = world.get::<InMap>(killer).copied() else {
        return;
    };
    if world.get::<InMap>(victim) != Some(&map)
        || super::read_player_battleground(world, killer).is_some()
        || world
            .get::<ActiveAuras>(victim)
            .is_some_and(|auras| auras.has_type(159))
    {
        return;
    }
    let Some(victim_level) = world.get::<Level>(victim).map(|level| level.0) else {
        return;
    };
    let Some(character) = world.get::<ClientId>(killer).map(|id| id.0) else {
        return;
    };
    let members = world
        .get_resource::<crate::group::GroupRegistry>()
        .map(|groups| groups.group_members(character))
        .unwrap_or_default();
    let recipients: Vec<_> = world
        .query::<(Entity, &ClientId, &Player)>()
        .iter(world)
        .filter(|(entity, id, _)| *entity == killer || members.contains(&id.0))
        .filter(|(entity, _, _)| world.get::<InMap>(*entity) == Some(&map))
        .filter(|(entity, _, _)| *entity == killer || in_reward_range(world, *entity, victim))
        .filter(|(entity, _, _)| {
            world
                .get::<Health>(*entity)
                .is_some_and(|h| h.current > 0.0)
        })
        .map(|(entity, _, _)| entity)
        .collect();
    for &player in &recipients {
        if effective_team(world, player) == effective_team(world, victim) {
            continue;
        }
        let Some(level) = world.get::<Level>(player).map(|level| level.0) else {
            continue;
        };
        if level < 10
            || world
                .get::<ActiveAuras>(player)
                .is_some_and(|a| a.has_spell(43681))
        {
            continue;
        }
        let gray = gray_level(level);
        if victim_level <= gray {
            continue;
        }
        let base = (f32::from(level) * 1.55 * f32::from(victim_level - gray)
            / f32::from(level - gray))
        .ceil();
        let mut amount = base / recipients.len() as f32;
        amount *= 1.0 + maximum_honor_modifier(world, player, 212, true) / 100.0;
        amount *= 1.0 + maximum_honor_modifier(world, player, 491, false) / 100.0;
        award_lifetime_xp(world, player, amount as u32);
    }
}

fn effective_team(world: &World, player: Entity) -> u8 {
    world
        .get::<CurrentPvpFaction>(player)
        .map(|team| team.0)
        .unwrap_or_else(|| {
            crate::faction::current_pvp_faction_for_race(
                world.get::<Player>(player).expect("player recipient").race,
            )
            .0
        })
}

fn in_reward_range(world: &World, player: Entity, victim: Entity) -> bool {
    let (Some(a), Some(b)) = (world.get::<Position>(player), world.get::<Position>(victim)) else {
        return false;
    };
    let reach = |entity| 1.5 * world.get::<UnitScale>(entity).map_or(1.0, |scale| scale.0);
    (crate::creature_tap::distance(a, b) - reach(player) - reach(victim)).max(0.0)
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
    world
        .get::<ActiveAuras>(player)
        .into_iter()
        .flat_map(|auras| auras.iter())
        .flat_map(|aura| aura.effects.iter().map(move |effect| (aura, effect)))
        .filter(|(_, effect)| {
            effect.aura_type == kind && (!kill_only || effect.misc_values[0] & 1 != 0)
        })
        .map(|(aura, effect)| effect.amount * aura.stack_multiplier())
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
