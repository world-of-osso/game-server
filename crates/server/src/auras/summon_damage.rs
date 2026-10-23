//! SimC76f8118 parse_effects: pet429, guardian531 and caster's guardian380.
use super::{ActiveAuras, AuraStatMods, handlers};
use crate::hunter_pet::HunterPet;
use crate::spell_cast::summon::SummonedCreature;
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

type Ownership<'a> = (Option<&'a SummonedCreature>, Option<&'a HunterPet>);

/// Controlled pets win even if their summon properties also name a guardian.
fn owner_and_guardian((summon, pet): Ownership<'_>) -> Option<(Entity, bool)> {
    if let Some(pet) = pet {
        return Some((pet.owner, false));
    }
    let summon = summon?;
    if summon.properties.is_pet() {
        return Some((summon.owner, false));
    }
    summon
        .properties
        .is_guardian()
        .then_some((summon.owner, true))
}

fn owner_multiplier(mods: &AuraStatMods, guardian: bool) -> f32 {
    if guardian {
        mods.guardian_damage_mult
    } else {
        mods.pet_damage_mult
    }
}

fn guardian_taken_multiplier(
    ownership: Option<(Entity, bool)>,
    auras: Option<&ActiveAuras>,
) -> f32 {
    let Some((owner, true)) = ownership else {
        return 1.0;
    };
    auras.map_or(1.0, |auras| {
        auras
            .iter()
            .filter(|aura| aura.caster == owner)
            .flat_map(|aura| {
                let stacks = aura.stack_multiplier();
                aura.effects
                    .iter()
                    .filter(|effect| effect.aura_type == handlers::MOD_DAMAGE_FROM_CASTER_GUARDIAN)
                    .map(move |effect| 1.0 + effect.amount * stacks / 100.0)
            })
            .product()
    })
}

fn ownership(world: &World, unit: Entity) -> Option<(Entity, bool)> {
    owner_and_guardian((
        world.get::<SummonedCreature>(unit),
        world.get::<HunterPet>(unit),
    ))
}

pub(crate) fn multiplier(world: &World, unit: Entity) -> f32 {
    ownership(world, unit)
        .and_then(|(owner, guardian)| {
            world
                .get::<AuraStatMods>(owner)
                .map(|mods| owner_multiplier(mods, guardian))
        })
        .unwrap_or(1.0)
}

pub(crate) fn taken_multiplier(world: &World, unit: Entity, auras: Option<&ActiveAuras>) -> f32 {
    guardian_taken_multiplier(ownership(world, unit), auras)
}

/// Reads native ownership without aliasing melee's mutable health/aura query.
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
            .and_then(owner_and_guardian)
            .and_then(|(owner, guardian)| {
                self.modifiers
                    .get(owner)
                    .ok()
                    .map(|mods| owner_multiplier(mods, guardian))
            })
            .unwrap_or(1.0)
    }

    pub(crate) fn pet_melee_time_multiplier(&self, unit: Entity) -> f32 {
        let Some((owner, false)) = self.owners.get(unit).ok().and_then(owner_and_guardian) else {
            return 1.0;
        };
        self.modifiers
            .get(owner)
            .map_or(1.0, |mods| mods.pet_melee_time_mult)
    }

    pub(crate) fn taken_multiplier(&self, unit: Entity, auras: Option<&ActiveAuras>) -> f32 {
        let ownership = self.owners.get(unit).ok().and_then(owner_and_guardian);
        guardian_taken_multiplier(ownership, auras)
    }
}
