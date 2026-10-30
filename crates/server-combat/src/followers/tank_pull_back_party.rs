//! Keep the party together without dropping combat while the tank positions a pull.
use bevy::prelude::*;
use shared::casting::CastState;
use shared::group::GroupRole;

use super::{Follower, positioning};
use crate::creature_ai::{CreatureState, FollowOwner};
use crate::networking::CombatTarget;

#[derive(Component)]
struct SavedPetReaction(u32);

fn hold_pets(world: &mut World, owner: Entity) {
    let mut changed = false;
    for pet in crate::hunter_pet::active_owned_pets(world, owner) {
        if world.get::<SavedPetReaction>(pet).is_none()
            && let Some(charm) = world.get::<crate::pet_control::PetCharm>(pet)
        {
            let reaction = charm.react;
            world.entity_mut(pet).insert(SavedPetReaction(reaction));
            changed = true;
        }
        if let Some(mut charm) = world.get_mut::<crate::pet_control::PetCharm>(pet) {
            charm.react = shared::protocol::REACT_PASSIVE;
        }
    }
    if changed {
        crate::pet_control::order_companion_follow(world, owner);
    }
}

pub(super) fn restore_pets(world: &mut World, owner: Entity) {
    let pets: Vec<_> = world
        .query::<(Entity, &crate::hunter_pet::HunterPet)>()
        .iter(world)
        .filter(|(_, pet)| pet.owner == owner)
        .map(|(unit, _)| unit)
        .collect();
    for pet in pets {
        if let Some(saved) = world.entity_mut(pet).take::<SavedPetReaction>()
            && let Some(mut charm) = world.get_mut::<crate::pet_control::PetCharm>(pet)
        {
            charm.react = saved.0;
        }
    }
}

pub(super) fn find_tank(world: &World, party: &[Entity]) -> Option<Entity> {
    party.iter().copied().find(|&unit| {
        world
            .get::<Follower>(unit)
            .is_some_and(|follower| follower.profile.role == GroupRole::Tank)
            && crate::spell_cast::targets::is_alive(world, unit)
    })
}

pub(super) fn stop_damage(world: &mut World, unit: Entity) {
    if world.get::<CastState>(unit).is_some_and(|cast| {
        let hostile = Entity::try_from_bits(cast.target)
            .is_some_and(|target| crate::spell_cast::targets::is_hostile(world, unit, target));
        let damage_slot = world.get::<Follower>(unit).is_some_and(|follower| {
            world
                .resource::<super::FollowerCatalog>()
                .kits
                .iter()
                .any(|entry| {
                    entry.spec_id == follower.profile.spec_id
                        && entry.spell_id == cast.spell_id
                        && matches!(entry.slot, super::kit::Slot::Attack | super::kit::Slot::Aoe)
                })
        });
        hostile || damage_slot
    }) {
        crate::spell_cast::cancel_cast(world, unit);
    }
    crate::spell_cast::auto_repeat::cancel(world, unit);
    crate::combat::attack_stop(world, unit);
    hold_pets(world, unit);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pull_back_stops_damage_without_dropping_native_combat_reference() {
        let (mut h, mage, targets) = super::super::combat_refs_tests::native_dummies();
        let target = targets[0];
        crate::spell_cast::creature_cast(h.app.world_mut(), mage, 133, Some(target), false)
            .unwrap();
        for _ in 0..120 {
            h.advance(1);
            if crate::combat_refs::has_active_reference_to(h.app.world(), target, mage) {
                break;
            }
        }
        assert!(crate::combat_refs::has_active_reference_to(
            h.app.world(),
            target,
            mage
        ));
        h.advance(80);
        crate::spell_cast::creature_cast(h.app.world_mut(), mage, 133, Some(target), false)
            .unwrap();
        assert!(h.app.world().get::<CastState>(mage).is_some());
        stop_damage(h.app.world_mut(), mage);
        assert!(h.app.world().get::<CastState>(mage).is_none());
        assert!(crate::combat_refs::has_active_reference_to(
            h.app.world(),
            target,
            mage
        ));
        let casts = h
            .inbox
            .go
            .iter()
            .filter(|go| go.caster == mage.to_bits() && go.spell_id == 133)
            .count();
        h.advance(80);
        assert_eq!(
            h.inbox
                .go
                .iter()
                .filter(|go| go.caster == mage.to_bits() && go.spell_id == 133)
                .count(),
            casts
        );
        assert!(crate::combat_refs::has_active_reference_to(
            h.app.world(),
            target,
            mage
        ));
    }

    #[test]
    fn healer_keeps_healing_and_combat_reference_while_party_waits() {
        let (mut h, healer) = super::super::kit_tests::at_level(209072, 13, |_| {});
        let owner = h.caster;
        let enemy = h.npc;
        let before = {
            let mut health = h
                .app
                .world_mut()
                .get_mut::<shared::components::Health>(owner)
                .unwrap();
            health.current = health.max * 0.1;
            health.current
        };
        assert!(crate::combat_refs::has_active_reference_to(
            h.app.world(),
            enemy,
            owner
        ));
        h.app
            .world_mut()
            .entity_mut(owner)
            .insert(super::super::tank_pull_back::WaitingForPullBack);
        let follower = h.app.world().get::<Follower>(healer).unwrap().clone();
        for _ in 0..240 {
            super::super::ai::update_follower(h.app.world_mut(), healer, &follower);
            h.advance(1);
            if h.health(owner) > before {
                break;
            }
        }
        assert!(
            h.health(owner) > before,
            "healer must use a native heal while waiting"
        );
        assert!(
            crate::combat_refs::has_active_reference_to(h.app.world(), enemy, owner),
            "waiting must retain the live combat reference"
        );
    }
}

pub(super) fn follow_tank(world: &mut World, unit: Entity, tank: Entity) {
    if positioning::hold_unsafe_approach(world, unit, tank) {
        return;
    }
    match super::hazards::approach_is_unsafe(world, unit, tank) {
        Some(false) => {}
        Some(true) | None => return,
    }
    let following = world
        .get::<FollowOwner>(unit)
        .is_some_and(|follow| follow.owner == tank);
    world
        .entity_mut(unit)
        .insert((CreatureState::Idle, CombatTarget(None)));
    if !following {
        world.entity_mut(unit).insert(FollowOwner::new(tank));
    }
}
