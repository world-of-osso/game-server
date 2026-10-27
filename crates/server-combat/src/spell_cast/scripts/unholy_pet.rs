//! SimC76f8118 sc_death_knight.cpp12256–12301 permanent Unholy Raise Dead.
use crate::spell_cast::{self, effects::EffectCtx};
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;

pub(super) fn after_cast_hook(spell: u32) -> Option<super::AfterCastHook> {
    match spell {
        46584 => Some(summon_ghoul),
        1233448 => Some(track_dark_transformation),
        _ => None,
    }
}

fn track_dark_transformation(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    let Some(pet) = crate::hunter_pet::summoned_pet(world, caster) else {
        return;
    };
    if !world
        .get::<crate::auras::ActiveAuras>(pet)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == 1233448 && aura.caster == caster)
        })
    {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp7253–7255/8759: the actual
    // pet-tied transformation also owns the native player buff.
    if !spell_cast::try_cast_triggered(world, caster, 1235391, Some(caster), 1) {
        warn!(?caster, "Native Dark Transformation owner state refused");
    }
}

fn summon_ghoul(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if world
        .get::<shared::components::ActiveSpec>(caster)
        .map(|spec| spec.0)
        != Some(252)
    {
        return;
    }
    // Native70448 retains the creature26125 identity on the class summon46585;
    // specialization46584/1242866 is only a dummy driver. Reuse the controlled
    // pet path, without a one-minute guardian or any fake creature/template.
    let effect = world
        .resource::<SpellStore>()
        .get(46585)
        .expect("native class Raise Dead metadata")
        .effects
        .iter()
        .find(|effect| effect.effect == spell_cast::summon::EFFECT_SUMMON)
        .expect("native Raise Dead creature/property binding")
        .clone();
    let context = EffectCtx {
        caster,
        spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    crate::hunter_pet::effect_summon_pet(world, &context, &effect);
}
