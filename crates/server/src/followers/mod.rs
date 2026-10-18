//! User-approved player-spec combat bridge for authenticated Retail followers.
mod ai;
mod catalog;
mod hazards;
mod kit;
mod kit_ai;
mod kit_area;
mod party;
#[cfg(test)]
mod pull_budget;
#[cfg(test)]
mod pull_budget_tests;
#[cfg(test)]
mod pull_forecast_tests;
pub(crate) mod pull_history;
#[cfg(test)]
mod pull_history_tests;
#[cfg(test)]
mod pull_readiness;
#[cfg(test)]
mod pull_readiness_tests;
mod spawning;
mod stockade_diagnostic;
#[cfg(test)]
mod tank_planner;
#[cfg(test)]
mod tank_planner_tests;

use bevy::prelude::*;
use shared::group::GroupRole;

pub(crate) use ai::FollowerReach;
pub(crate) use catalog::FollowerCatalog;
pub(crate) use party::queue_player;

#[derive(Clone, Debug)]
pub(crate) struct Profile {
    pub entry: u32,
    pub name: String,
    pub class_id: u8,
    pub spec_id: u32,
    pub role: GroupRole,
    pub attack_spell: u32,
    pub cooldown_spell: u32,
    pub heal_spell: u32,
    pub taunt_spell: u32,
    pub form_spell: u32,
    pub melee: bool,
}

#[derive(Component, Clone)]
pub(crate) struct Follower {
    pub owner: Entity,
    pub profile: Profile,
}

impl Follower {
    /// Runtime-only companion identities occupy the high half of the group ID space;
    /// they are not persisted characters and never enter auth/character storage.
    pub(crate) fn member_id(entity: Entity) -> u64 {
        entity.to_bits() | (1 << 63)
    }

    pub(crate) fn online(
        &self,
        entity: Entity,
        level: u8,
        dungeon_difficulty: u32,
    ) -> crate::group::OnlineMember {
        crate::group::OnlineMember {
            entity: entity.to_bits(),
            name: self.profile.name.clone(),
            class: self.profile.class_id,
            level,
            portrait: Default::default(),
            dungeon_difficulty,
            raid_difficulty: Default::default(),
        }
    }
}

pub(crate) fn select_party(profiles: &[Profile], role: GroupRole) -> Result<Vec<Profile>, String> {
    let mut party = Vec::with_capacity(4);
    for (required, count) in [
        (GroupRole::Tank, 1),
        (GroupRole::Healer, 1),
        (GroupRole::Dps, 3),
    ] {
        let needed = count - usize::from(role == required);
        let available: Vec<_> = profiles
            .iter()
            .filter(|profile| profile.role == required)
            .take(needed)
            .cloned()
            .collect();
        if available.len() != needed {
            return Err(format!("Follower roster lacks {required:?} members"));
        }
        party.extend(available);
    }
    Ok(party)
}

/// Approved content bridge: follower copies use the same map's Normal population.
/// Instance identity, creature difficulty, spell execution and loot remain difficulty205.
/// Retire when local follower populations and entrances are complete.
pub(crate) fn population_difficulty(difficulty: u32) -> u32 {
    if difficulty == catalog::FOLLOWER_DIFFICULTY {
        crate::maps::store::DIFFICULTY_NORMAL
    } else {
        difficulty
    }
}

pub(crate) fn update_ai(world: &mut World) {
    ai::update(world);
}

pub(crate) struct FollowerPlugin;
impl Plugin for FollowerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FollowerCatalog>()
            .add_systems(
                Startup,
                (catalog::load_startup, stockade_diagnostic::load_request),
            )
            .add_systems(
                Update,
                (stockade_diagnostic::advance, party::advance_runs).chain(),
            )
            .add_systems(
                FixedUpdate,
                update_ai
                    .after(crate::creature_aggro::update_victim_system)
                    .before(crate::creature_ai::chase_system),
            );
    }
}

#[cfg(test)]
mod catalog_tests;
#[cfg(test)]
mod cc_tests;
#[cfg(test)]
mod defensive_tests;
#[cfg(test)]
mod hazard_tests;
#[cfg(test)]
mod kit_interrupt_tests;
#[cfg(test)]
mod kit_source_tests;
#[cfg(test)]
mod kit_tests;
#[cfg(test)]
mod kit_utility_tests;
#[cfg(test)]
mod navigation_tests;
#[cfg(test)]
mod passive_stats_tests;
#[cfg(test)]
mod pet_tests;
#[cfg(test)]
mod profile_behavior_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod utility_tests;
#[cfg(test)]
mod wire_tests;
