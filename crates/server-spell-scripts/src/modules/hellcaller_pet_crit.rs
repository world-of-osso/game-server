//! SimC76f8118 sc_warlock_pets.cpp265–301: Xalan pet spell crit multipliers.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::{SpellInfo, SpellStore};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ActualCriticalChanceBonus(
    xalan_pet_critical_bonus,
)];

fn xalan_pet_critical_bonus(
    world: &World,
    caster: Entity,
    _target: Entity,
    spell: &SpellInfo,
    chance: f32,
) -> f32 {
    if combat::combat::read_pet_owner_critical_chance(world, caster).is_none() {
        return 0.0;
    }
    let owner = world
        .get::<combat::hunter_pet::HunterPet>(caster)
        .map(|pet| pet.owner)
        .or_else(|| {
            world
                .get::<combat::spell_cast::summon::SummonedCreature>(caster)
                .map(|summon| summon.owner)
        });
    let Some(owner) = owner else { return 0.0 };
    if !world
        .get::<ActiveSpec>(owner)
        .is_some_and(|spec| matches!(spec.0, 265..=267))
    {
        return 0.0;
    }
    let Some(auras) = world.get::<ActiveAuras>(owner) else {
        return 0.0;
    };
    let store = world.resource::<SpellStore>();
    let mut multiplier = 1.0;
    for (talent, indices) in [(440040, &[5, 8][..]), (440044, &[5, 6][..])] {
        if !auras
            .iter()
            .any(|aura| aura.caster == owner && aura.spell_id == talent)
        {
            continue;
        }
        let info = store.get(talent).expect("selected Xalan talent exists");
        for index in indices {
            let effect = info
                .effects
                .iter()
                .find(|effect| effect.index == *index)
                .expect("Xalan pet crit effect exists");
            if u32::try_from(effect.misc_values[1])
                .is_ok_and(|label| spell.labels.binary_search(&label).is_ok())
            {
                multiplier *= 1.0 + effect.base_points / 100.0;
            }
        }
    }
    chance * (multiplier - 1.0)
}
