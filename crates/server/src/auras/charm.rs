//! TC a352b1fa Unit::SetCharmedBy/RemoveCharmedBy (Unit.cpp11837-12130),
//! HandleModPossess/HandleModCharm (SpellAuraEffects.cpp3238-3320).
//! Charm is not summon ownership: retain the unit, its stats and original faction.
use bevy::prelude::*;
use shared::components::{
    CreatureMotion, MovementControl, MovementSpeed, Player, PlayerMotion, Position, Rotation,
    UnitCharm, UnitFactionTemplate, UnitFlags, VerticalVelocity,
};
use shared::protocol::{REACT_DEFENSIVE, REACT_PASSIVE};

use super::{AuraApplication, AuraInstance, RemoveReason, handlers};
use crate::creature_ai::{FollowOwner, PetMotion};
use crate::pet_control::PetCharm;

const POSSESSED_FLAG: u32 = 0x0100_0000;
const REMOVE_CLIENT_CONTROL_FLAG: u32 = 0x0000_0004;
const CHARM_FLAGS: u32 = POSSESSED_FLAG | crate::faction::UNIT_FLAG_PLAYER_CONTROLLED;

#[derive(Component)]
pub(crate) struct CharmedUnit(pub Entity);

#[derive(Component)]
pub(crate) struct CharmState {
    pub owner: Entity,
    pub possessed: bool,
    aura: u32,
    pub spell: u32,
    faction: UnitFactionTemplate,
    flags: u32,
    owner_flags: u32,
    owner_controlled: bool,
    target_controlled: bool,
    pet_charm: Option<PetCharm>,
}

fn charm_effect(aura: &AuraInstance) -> Option<u32> {
    aura.effects
        .iter()
        .find(|effect| is_charm(effect.aura_type))
        .map(|effect| effect.aura_type)
}

pub(crate) fn is_charm(aura_type: u32) -> bool {
    matches!(aura_type, handlers::MOD_POSSESS | handlers::MOD_CHARM)
}

/// Refuse a second charmer before storing any effects of its aura. Same-owner refresh
/// keeps the existing control link and original faction snapshot.
pub(super) fn can_apply(world: &World, application: &AuraApplication, aura: &AuraInstance) -> bool {
    if charm_effect(aura).is_none() {
        return true;
    }
    let caster = application.caster;
    let target = application.target;
    caster != target
        && world.get::<UnitFactionTemplate>(caster).is_some()
        && world.get::<UnitFactionTemplate>(target).is_some()
        && world.get::<CharmState>(caster).is_none()
        && world
            .get::<CharmState>(target)
            .is_none_or(|state| state.owner == caster && state.spell == application.spell_id)
}

pub(super) fn on_applied(world: &mut World, target: Entity, aura: &AuraInstance) {
    let Some(kind) = charm_effect(aura) else {
        return;
    };
    let owner = aura.caster;
    release_owned(world, owner);
    release_owned(world, target);
    let possessed = kind == handlers::MOD_POSSESS && world.get::<Player>(owner).is_some();
    let state = capture_state(world, target, aura, possessed);
    crate::spell_cast::stop_all_casts(world, target);
    reset_combat(world, target);
    super::mount::cancel_mount_aura(world, owner);
    super::mount::cancel_mount_aura(world, target);
    let passives = install_control(world, target, &state);
    world.entity_mut(owner).insert(CharmedUnit(target));
    world.entity_mut(target).insert(state);
    super::charm_spells::cast_passives(world, target, &passives);
    crate::pet_control::send_pet_spells(world, owner, target);
}

fn capture_state(
    world: &World,
    target: Entity,
    aura: &AuraInstance,
    possessed: bool,
) -> CharmState {
    CharmState {
        owner: aura.caster,
        possessed,
        aura: aura.instance_id,
        spell: aura.spell_id,
        faction: *world
            .get::<UnitFactionTemplate>(target)
            .expect("validated charm faction"),
        flags: world.get::<UnitFlags>(target).map_or(0, |flags| flags.0),
        owner_flags: world
            .get::<UnitFlags>(aura.caster)
            .map_or(0, |flags| flags.0),
        owner_controlled: world
            .get::<MovementControl>(aura.caster)
            .is_some_and(|control| control.controlled),
        target_controlled: world
            .get::<MovementControl>(target)
            .is_some_and(|control| control.controlled),
        pet_charm: world.get::<PetCharm>(target).cloned(),
    }
}

fn install_control(world: &mut World, target: Entity, state: &CharmState) -> Vec<u32> {
    let owner_faction = *world
        .get::<UnitFactionTemplate>(state.owner)
        .expect("validated charmer");
    let player_owner = world.get::<Player>(state.owner).is_some();
    let mut flags = state.flags & !CHARM_FLAGS;
    if player_owner {
        flags |= crate::faction::UNIT_FLAG_PLAYER_CONTROLLED;
    }
    if state.possessed {
        flags |= POSSESSED_FLAG;
    }
    let (bar, passives) = super::charm_spells::build_bar(world, target, state.possessed);
    let react = if state.possessed {
        REACT_PASSIVE
    } else {
        REACT_DEFENSIVE
    };
    world
        .entity_mut(target)
        .remove::<(FollowOwner, PetMotion)>()
        .insert((
            owner_faction,
            UnitFlags(flags),
            UnitCharm {
                charmer: state.owner.to_bits(),
                possessed: state.possessed,
            },
            PetCharm::new(bar, react),
            CreatureMotion::Still,
        ));
    if state.possessed {
        install_mover(world, target);
        change_movement_control(world, state.owner, true);
        let flags = state.owner_flags | REMOVE_CLIENT_CONTROL_FLAG;
        world.entity_mut(state.owner).insert(UnitFlags(flags));
    } else {
        world
            .entity_mut(target)
            .insert(FollowOwner::new(state.owner));
        if world.get::<Player>(target).is_some() {
            change_movement_control(world, target, true);
        }
    }
    crate::spell_cast::mark_dirty(world, target);
    crate::spell_cast::mark_dirty(world, state.owner);
    passives
}

fn install_mover(world: &mut World, target: Entity) {
    let y = world
        .get::<Position>(target)
        .expect("charmed unit position")
        .y;
    let speed = crate::creature_ai::BASE_RUN_SPEED
        * world
            .get::<super::AuraStatMods>(target)
            .map_or(1.0, |mods| mods.movement_speed);
    let mut entity = world.entity_mut(target);
    // Creature MovementSpeed is its last reported animation speed, initially0; a
    // client mover needs the actual run speed before it can send its first input.
    entity.insert(MovementSpeed(speed));
    if !entity.contains::<Rotation>() {
        entity.insert(Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        });
    }
    if !entity.contains::<PlayerMotion>() {
        entity.insert(PlayerMotion::default());
    }
    if !entity.contains::<VerticalVelocity>() {
        entity.insert(VerticalVelocity(0.0));
    }
    if !entity.contains::<crate::networking::FallTracker>() {
        entity.insert(crate::networking::FallTracker::grounded(y));
    }
    change_movement_control(world, target, false);
}

fn change_movement_control(world: &mut World, unit: Entity, controlled: bool) {
    let previous = world
        .get::<MovementControl>(unit)
        .copied()
        .unwrap_or_default();
    let owner_epoch = world
        .get::<UnitCharm>(unit)
        .filter(|charm| charm.possessed)
        .and_then(|charm| Entity::try_from_bits(charm.charmer))
        .and_then(|owner| world.get::<MovementControl>(owner))
        .map_or(0, |control| control.epoch);
    world.entity_mut(unit).insert(MovementControl {
        epoch: previous.epoch.max(owner_epoch).wrapping_add(1),
        controlled,
    });
    world.entity_mut(unit).insert(PlayerMotion::default());
    crate::spell_cast::mark_dirty(world, unit);
}

fn reset_combat(world: &mut World, target: Entity) {
    crate::combat::combat_stop(world, target);
    crate::creature_aggro::forget_combat(&mut world.entity_mut(target));
    world
        .entity_mut(target)
        .insert(crate::networking::Moving(false));
}

pub(super) fn on_removed(world: &mut World, target: Entity, aura: &AuraInstance) {
    if !world
        .get::<CharmState>(target)
        .is_some_and(|state| state.aura == aura.instance_id)
    {
        return;
    }
    let state = world
        .entity_mut(target)
        .take::<CharmState>()
        .expect("matched charm state");
    crate::spell_cast::stop_all_casts(world, target);
    reset_combat(world, target);
    restore_unit(world, target, &state);
    if world.get_entity(state.owner).is_ok() {
        restore_owner(world, &state);
        // A canceled channel must not leave the charmer channeling an absent aura.
        if world
            .get::<shared::casting::CastState>(state.owner)
            .is_some_and(|cast| cast.spell_id == state.spell)
        {
            crate::spell_cast::stop_all_casts(world, state.owner);
        }
    }
}

fn restore_unit(world: &mut World, target: Entity, state: &CharmState) {
    let flags = world.get::<UnitFlags>(target).map_or(0, |flags| flags.0);
    world
        .entity_mut(target)
        .remove::<(UnitCharm, PetCharm, FollowOwner, PetMotion)>()
        .insert((
            state.faction,
            UnitFlags((flags & !CHARM_FLAGS) | (state.flags & CHARM_FLAGS)),
            CreatureMotion::Still,
        ));
    if let Some(charm) = &state.pet_charm {
        world.entity_mut(target).insert(charm.clone());
    }
    if world.get::<MovementControl>(target).is_some() {
        change_movement_control(world, target, state.target_controlled);
    }
    crate::spell_cast::mark_dirty(world, target);
}

fn restore_owner(world: &mut World, state: &CharmState) {
    world.entity_mut(state.owner).remove::<CharmedUnit>();
    if state.possessed {
        change_movement_control(world, state.owner, state.owner_controlled);
        let flags = world
            .get::<UnitFlags>(state.owner)
            .map_or(0, |flags| flags.0);
        world.entity_mut(state.owner).insert(UnitFlags(
            (flags & !REMOVE_CLIENT_CONTROL_FLAG)
                | (state.owner_flags & REMOVE_CLIENT_CONTROL_FLAG),
        ));
    }
    if let Some(pet) = crate::hunter_pet::summoned_pet(world, state.owner) {
        crate::pet_control::send_pet_spells(world, state.owner, pet);
    } else {
        crate::pet_control::clear_pet_spells(world, state.owner);
    }
    crate::spell_cast::mark_dirty(world, state.owner);
}

pub(crate) fn release_owned(world: &mut World, owner: Entity) {
    let Some(target) = world.get::<CharmedUnit>(owner).map(|link| link.0) else {
        return;
    };
    release(world, target);
}

pub(crate) fn release(world: &mut World, target: Entity) {
    let Some(aura) = world.get::<CharmState>(target).map(|state| state.aura) else {
        return;
    };
    super::remove_auras(world, target, RemoveReason::Cancelled, |instance| {
        instance.instance_id == aura
    });
}

pub(crate) fn controlled_unit(world: &World, owner: Entity) -> Option<Entity> {
    world.get::<CharmedUnit>(owner).map(|link| link.0)
}

pub(crate) fn body_inactive(world: &World, player: Entity) -> bool {
    world.get::<CharmState>(player).is_some()
        || controlled_unit(world, player).is_some_and(|target| {
            world
                .get::<CharmState>(target)
                .is_some_and(|state| state.possessed)
        })
}

/// Losing the charmer through death/map transfer releases even a finite, non-channel charm.
pub(super) fn update(world: &mut World) {
    let lost: Vec<Entity> = world
        .query::<(Entity, &CharmState)>()
        .iter(world)
        .filter(|(target, state)| {
            !crate::spell_cast::targets::is_alive(world, state.owner)
                || world.get::<crate::maps::InMap>(*target)
                    != world.get::<crate::maps::InMap>(state.owner)
        })
        .map(|(target, _)| target)
        .collect();
    for target in lost {
        release(world, target);
    }
}
