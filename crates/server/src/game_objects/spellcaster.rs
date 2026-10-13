//! TC a352b1fa GameObject.cpp:3155-3185/3564-3571 and GameObjectData.h:409.
//! Spellcaster objects cast their authored spell as the user; partyOnly checks owner.
use bevy::prelude::*;
use shared::components::{Health, Player, Position};
use shared::protocol::InteractionError;

use super::{GAME_OBJECT_INTERACT_DISTANCE, GameObjectBounds, summoned::SummonedObject};
use crate::persistence::ClientId;

pub(crate) const GAMEOBJECT_TYPE_SPELLCASTER: u8 = 22;
const MOUNTED_AURA: u32 = 78;

#[derive(Component, Clone, Debug)]
pub(crate) struct SpellcasterObject {
    pub spell: u32,
    pub charges: i32,
    pub party_only: bool,
    pub(super) identity: Result<super::identity_conditions::IdentityCondition, String>,
    pub radius_override: u32,
    pub uses: u32,
}

fn may_use(
    world: &World,
    player: Entity,
    object: Entity,
    data: &SpellcasterObject,
) -> Result<(), InteractionError> {
    let player_data = world
        .get::<Player>(player)
        .ok_or(InteractionError::InvalidTarget)?;
    let identity = data
        .identity
        .as_ref()
        .map_err(|_| InteractionError::InvalidTarget)?;
    if !identity.permits(player_data) {
        return Err(InteractionError::InvalidTarget);
    }
    if world
        .get::<Health>(player)
        .is_none_or(|health| health.current <= 0.0)
    {
        return Err(InteractionError::PlayerDead);
    }
    if !crate::spell_cast::targets::same_map(world, player, object)
        || !in_reach(world, player, object, data)
    {
        return Err(InteractionError::TooFarAway);
    }
    if data.charges > 0 && data.uses >= data.charges as u32 {
        return Err(InteractionError::InvalidTarget);
    }
    if data.party_only && !shares_owner_group(world, player, object) {
        return Err(InteractionError::InvalidTarget);
    }
    Ok(())
}

fn shares_owner_group(world: &World, player: Entity, object: Entity) -> bool {
    let Some(summoned) = world.get::<SummonedObject>(object) else {
        return false;
    };
    if summoned.owner == player {
        return true;
    }
    let (Some(owner), Some(user)) = (
        world.get::<ClientId>(summoned.owner),
        world.get::<ClientId>(player),
    ) else {
        return false;
    };
    world
        .resource::<crate::group::GroupRegistry>()
        .is_in_same_group(owner.0, user.0)
}

fn in_reach(world: &World, player: Entity, object: Entity, data: &SpellcasterObject) -> bool {
    let (Some(user), Some(at), Some(bounds)) = (
        world.get::<Position>(player),
        world.get::<Position>(object),
        world.get::<GameObjectBounds>(object),
    ) else {
        return false;
    };
    let distance = if data.radius_override > 0 {
        data.radius_override as f32 / 100.0
    } else {
        GAME_OBJECT_INTERACT_DISTANCE
    };
    let coordinates = |pos: &Position| {
        let (x, y, z) = crate::terrain_height::bevy_to_wow(pos.x, pos.y, pos.z);
        Vec3::new(x, y, z)
    };
    bounds.reaches(coordinates(at), coordinates(user), distance)
}

pub(crate) fn use_object(
    world: &mut World,
    player: Entity,
    object: Entity,
) -> Result<(), InteractionError> {
    let data = world
        .get::<SpellcasterObject>(object)
        .ok_or(InteractionError::InvalidTarget)?
        .clone();
    may_use(world, player, object, &data)?;
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(data.spell)
        .ok_or(InteractionError::InvalidTarget)?
        .clone();
    crate::auras::remove_auras(
        world,
        player,
        crate::auras::RemoveReason::Cancelled,
        |aura| {
            aura.effects
                .iter()
                .any(|effect| effect.aura_type == MOUNTED_AURA)
        },
    );
    crate::spell_cast::execute_cast(world, player, &spell, Some(player))
        .map_err(|_| InteractionError::InvalidTarget)?;
    world
        .get_mut::<SpellcasterObject>(object)
        .expect("cast does not remove its portal")
        .uses += 1;
    Ok(())
}
