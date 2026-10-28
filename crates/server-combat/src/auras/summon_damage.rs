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

/// SimC76f8118 sc_hunter.cpp1900/1928/1961/2003/2027: actual DireCritters.
pub(crate) fn is_dire_critter(summon: &SummonedCreature) -> bool {
    [219199, 442419, 459733, 459739, 471990].contains(&summon.spell_id)
}

// SimC76f8118 sc_hunter.cpp1877–1882: selected Dire Frenzy affects
// genuine DireCritters, including their melee and spell damage.
fn dire_frenzy_multiplier(summon: Option<&SummonedCreature>, mods: &AuraStatMods) -> f32 {
    if summon.is_some_and(is_dire_critter) {
        mods.dire_frenzy_damage_mult
    } else {
        1.0
    }
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

pub(crate) fn ownership(world: &World, unit: Entity) -> Option<(Entity, bool)> {
    owner_and_guardian((
        world.get::<SummonedCreature>(unit),
        world.get::<HunterPet>(unit),
    ))
}

pub fn multiplier(world: &World, unit: Entity) -> f32 {
    ownership(world, unit)
        .and_then(|(owner, guardian)| {
            world.get::<AuraStatMods>(owner).map(|mods| {
                owner_multiplier(mods, guardian)
                    * dire_frenzy_multiplier(world.get::<SummonedCreature>(unit), mods)
            })
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
    stats: Query<'w, 's, &'static crate::stats::CharacterStats>,
    npcs: Query<'w, 's, &'static shared::components::Npc>,
    demons: Query<'w, 's, (), With<crate::hunter_pet::SummonedDemon>>,
}

impl SummonDamageModifiers<'_, '_> {
    /// SimC76f8118 pets.cpp661–668: Felguard white swings use 1.44.
    pub(crate) fn white_swing_multiplier(&self, unit: Entity) -> f32 {
        if self
            .owners
            .get(unit)
            .ok()
            .is_some_and(|(_, pet)| pet.is_some())
            && self
                .npcs
                .get(unit)
                .is_ok_and(|npc| npc.template_id == 17252)
        {
            1.44
        } else {
            1.0
        }
    }

    pub(crate) fn multiplier(&self, unit: Entity) -> f32 {
        self.owners
            .get(unit)
            .ok()
            .and_then(|ownership| {
                let (owner, guardian) = owner_and_guardian(ownership)?;
                self.modifiers.get(owner).ok().map(|mods| {
                    owner_multiplier(mods, guardian) * dire_frenzy_multiplier(ownership.0, mods)
                })
            })
            .unwrap_or(1.0)
    }

    /// SimC76f8118 pet.hpp138–139: pet damage uses live owner versatility.
    pub(crate) fn owner_versatility_multiplier(&self, unit: Entity) -> Option<f32> {
        let (owner, _) = self.owners.get(unit).ok().and_then(owner_and_guardian)?;
        self.stats
            .get(owner)
            .ok()
            .map(|stats| 1.0 + stats.versatility_pct / 100.0)
    }

    pub(crate) fn pet_melee_time_multiplier(&self, unit: Entity) -> f32 {
        let Some((owner, guardian)) = self.owners.get(unit).ok().and_then(owner_and_guardian)
        else {
            return 1.0;
        };
        let pet_modifier = if guardian {
            1.0
        } else {
            self.modifiers
                .get(owner)
                .map_or(1.0, |mods| mods.pet_melee_time_mult)
        };
        // SimC76f8118 pet.cpp222–228: actual demons inherit owner attack haste.
        // Stat-bearing actors already use their own haste in the swing clock.
        let owner_haste = if self.demons.get(unit).is_ok() && self.stats.get(unit).is_err() {
            self.stats
                .get(owner)
                .map_or(1.0, |stats| 1.0 / (1.0 + stats.haste_pct / 100.0))
        } else {
            1.0
        };
        pet_modifier * owner_haste
    }

    pub(crate) fn taken_multiplier(&self, unit: Entity, auras: Option<&ActiveAuras>) -> f32 {
        let ownership = self.owners.get(unit).ok().and_then(owner_and_guardian);
        guardian_taken_multiplier(ownership, auras)
    }
}
