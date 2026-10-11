//! User-approved player-spec combat bridge for authenticated Retail followers.
use bevy::prelude::*;
use shared::group::GroupRole;

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

pub(crate) fn select_party(
    _profiles: &[Profile],
    _role: GroupRole,
) -> Result<Vec<Profile>, String> {
    Ok(Vec::new())
}

pub(crate) fn update_ai(_world: &mut World) {}

#[cfg(test)]
mod tests;
