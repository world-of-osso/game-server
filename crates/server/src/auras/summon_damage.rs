//! Native429 modifies damage of the owner's summons, not the owner itself.
use super::AuraStatMods;
use crate::hunter_pet::HunterPet;
use crate::spell_cast::summon::SummonedCreature;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

type Ownership<'a> = (Option<&'a SummonedCreature>, Option<&'a HunterPet>);

fn owner((summon, pet): Ownership<'_>) -> Option<Entity> {
    summon
        .map(|unit| unit.owner)
        .or_else(|| pet.map(|unit| unit.owner))
}

pub(crate) fn multiplier(world: &World, unit: Entity) -> f32 {
    owner((
        world.get::<SummonedCreature>(unit),
        world.get::<HunterPet>(unit),
    ))
    .and_then(|owner| world.get::<AuraStatMods>(owner))
    .map_or(1.0, |mods| mods.summon_damage_mult)
}

/// Reads the same native ownership fields without aliasing melee's mutable health query.
#[derive(SystemParam)]
pub(crate) struct SummonDamageModifiers<'w, 's> {
    owners: Query<'w, 's, Ownership<'static>>,
    modifiers: Query<'w, 's, &'static AuraStatMods>,
}

impl SummonDamageModifiers<'_, '_> {
    pub(crate) fn multiplier(&self, unit: Entity) -> f32 {
        self.owners
            .get(unit)
            .ok()
            .and_then(owner)
            .and_then(|owner| self.modifiers.get(owner).ok())
            .map_or(1.0, |mods| mods.summon_damage_mult)
    }
}
