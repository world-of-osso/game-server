//! Who earns a creature's kill (TrinityCore `Creature::m_tapList`, `SetTappedBy`,
//! `KillRewarder`).
//!
//! A player's damage taps the creature for the player and its subgroup
//! (`Unit::DealDamage`, Unit.cpp:985-991; `Creature::SetTappedBy`, Creature.cpp:1367-1399),
//! up to `CREATURE_TAPPERS_SOFT_CAP`; another creature's damage taps nothing. When it
//! dies every tapper is rewarded (`Unit::Kill`, Unit.cpp:11305-11425), however the killing
//! blow landed: a soldier finishing a worg the player hit still credits the player, and a
//! worg only soldiers hit credits nobody. Players' damage also lowers
//! `m_PlayerDamageReq` (half the creature's health); kill XP scales down linearly while
//! players dealt less than half (Formulas.h:211-212). Respawn and evade clear both
//! (Creature.cpp:2278-2279, CreatureAI.cpp:313-316).

use bevy::prelude::*;
use shared::components::{Health, Npc, Position, UnitTap};

use crate::group::GroupRegistry;
use crate::persistence::ClientId;

/// `CREATURE_TAPPERS_SOFT_CAP` (Creature.h:75).
const CREATURE_TAPPERS_SOFT_CAP: usize = 5;

/// Group members this close to the creature share its kill rewards
/// (`Player::IsAtGroupRewardDistance`, `CONFIG_GROUP_XP_DISTANCE` default).
pub(crate) const GROUP_REWARD_DISTANCE: f32 = 74.0;

/// The creature's tap list (character ids, tap order) and the player damage still
/// needed for full XP.
#[derive(Component, Debug, Clone, PartialEq)]
pub(crate) struct CreatureTap {
    pub tappers: Vec<u64>,
    pub player_damage_req: f32,
}

impl CreatureTap {
    /// A creature players alone damaged, tapped by `character` (test fixtures).
    #[cfg(test)]
    pub fn tapped_by(character: u64) -> Self {
        Self {
            tappers: vec![character],
            player_damage_req: 0.0,
        }
    }

    /// `Creature::ResetPlayerDamageReq`: half the creature's health.
    fn new(health: f32) -> Self {
        Self {
            tappers: Vec::new(),
            player_damage_req: health / 2.0,
        }
    }

    /// `Trinity::XP::Gain`'s scaling when players dealt less than half the health:
    /// `1 - 2 * m_PlayerDamageReq / max health`.
    pub fn xp_factor(&self, max_health: f32) -> f32 {
        if self.player_damage_req > 0.0 && max_health > 0.0 {
            1.0 - 2.0 * self.player_damage_req / max_health
        } else {
            1.0
        }
    }

    /// `SetTappedBy(player, withGroup = true)`.
    fn tap(&mut self, player: u64, subgroup: Vec<u64>) {
        if self.tappers.len() >= CREATURE_TAPPERS_SOFT_CAP {
            return;
        }
        for member in std::iter::once(player).chain(subgroup) {
            if !self.tappers.contains(&member) {
                self.tappers.push(member);
            }
        }
    }
}

/// `Unit::DealDamage` on a creature victim: `attacker`'s damage that took `loss` of
/// the `health_before` it had taps it for a player attacker or the authenticated
/// owner of a dungeon follower or controlled pet, and lowers the player damage requirement.
pub(crate) fn record_damage(
    world: &mut World,
    victim: Entity,
    attacker: Entity,
    health_before: f32,
    loss: f32,
) {
    if world.get::<Npc>(victim).is_none() {
        return;
    }
    let credit_source = crate::spell_cast::combat_event_owner(world, attacker);
    let Some(player) = world.get::<ClientId>(credit_source).map(|id| id.0) else {
        return;
    };
    let subgroup = world
        .get_resource::<GroupRegistry>()
        .map(|groups| groups.subgroup_members(player))
        .unwrap_or_default();
    let max_health = world.get::<Health>(victim).map_or(0.0, |health| health.max);
    let mut entity = world.entity_mut(victim);
    if entity.get::<CreatureTap>().is_none() {
        entity.insert(CreatureTap::new(max_health));
    }
    let mut tap = entity.get_mut::<CreatureTap>().expect("inserted above");
    tap.tap(player, subgroup);
    tap.player_damage_req = (tap.player_damage_req - loss.min(health_before)).max(0.0);
    replicate_tap(world, victim);
}

/// Send tap membership only when it changes, never the continuously changing damage budget.
fn replicate_tap(world: &mut World, creature: Entity) {
    let tappers = world
        .get::<CreatureTap>(creature)
        .map(|tap| tap.tappers.clone())
        .unwrap_or_default();
    let tap = UnitTap(tappers);
    if world.get::<UnitTap>(creature) == Some(&tap) {
        return;
    }
    world.entity_mut(creature).insert(tap);
    if let Some(dirty) = world.get_resource::<lightyear::prelude::ReplicationDirtyQueue>() {
        dirty.mark(creature);
    }
}

/// `SetTappedBy(nullptr)` + `ResetPlayerDamageReq` (respawn, evade).
pub(crate) fn clear(world: &mut World, creature: Entity) {
    world.entity_mut(creature).remove::<CreatureTap>();
    replicate_tap(world, creature);
}

/// Player positions kill rewards read.
pub(crate) type RewardPlayers<'w, 's> = Query<
    'w,
    's,
    (
        &'static ClientId,
        &'static Position,
        &'static crate::maps::InMap,
    ),
    With<shared::components::Player>,
>;

pub(crate) fn distance(a: &Position, b: &Position) -> f32 {
    ((a.x - b.x).powi(2) + (a.y - b.y).powi(2) + (a.z - b.z).powi(2)).sqrt()
}

/// `KillRewarder::Reward`'s rewarded players: for each tapper in the creature's map copy,
/// a grouped tapper's group once (that tapper, and members at group reward distance
/// of the creature, `_RewardGroup`), an ungrouped tapper alone wherever it stands.
pub(crate) fn rewarded_characters(
    tappers: &[u64],
    (creature_map, creature): (&crate::maps::InMap, &Position),
    players: &RewardPlayers,
    groups: Option<&GroupRegistry>,
) -> Vec<u64> {
    let mut rewarded: Vec<u64> = Vec::new();
    let mut processed_groups: Vec<u64> = Vec::new();
    let in_map = |character: u64| {
        players
            .iter()
            .any(|(id, _, map)| id.0 == character && map == creature_map)
    };
    // `ObjectAccessor::GetPlayer(*creature, guid)`: tappers in the creature's map.
    for &tapper in tappers.iter().filter(|&&tapper| in_map(tapper)) {
        let members = groups.map_or_else(Vec::new, |groups| groups.group_members(tapper));
        let Some(&group_key) = members.iter().min() else {
            if !rewarded.contains(&tapper) {
                rewarded.push(tapper);
            }
            continue;
        };
        if processed_groups.contains(&group_key) {
            continue;
        }
        processed_groups.push(group_key);
        rewarded.extend(members.into_iter().filter(|&member| {
            member == tapper
                || players.iter().any(|(id, position, map)| {
                    id.0 == member
                        && map == creature_map
                        && distance(position, creature) <= GROUP_REWARD_DISTANCE
                })
        }));
    }
    rewarded
}
