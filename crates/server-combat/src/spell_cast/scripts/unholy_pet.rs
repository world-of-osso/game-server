//! SimC76f8118 sc_death_knight.cpp12256–12301 permanent Unholy Raise Dead.
use crate::spell_cast::{self, effects::EffectCtx};
use crate::spell_info::{SpellInfo, SpellStore};
use bevy::prelude::*;

pub(super) fn after_cast_hook(spell: u32) -> Option<super::AfterCastHook> {
    (spell == 46584).then_some(summon_ghoul)
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

pub(super) fn on_cast(world: &mut World, caster: Entity) {
    // pet.cpp145–150/sc_death_knight.cpp3409–3438: native pet follows the
    // owner's selected enemy and executes its actual autocast bar.
    crate::hunter_pet::death_knight::order_on_owner_cast(world, caster);
}
