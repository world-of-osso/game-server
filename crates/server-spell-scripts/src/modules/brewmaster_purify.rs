//! SimC76f8118 sc_monk.cpp3507–3534: Purifying Brew clears the real Stagger pool.
use super::combat;
use bevy::prelude::*;
use combat::class_progression::TraitLoadout;
use combat::spell_cast::{
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
    scripts::{AfterCastHook, script_effect_value},
};
use combat::spell_info::{SpellInfo, SpellStore};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::AfterResourceValues(cast_hook)];

fn cast_hook(spell: u32) -> Option<AfterCastHook> {
    (spell == 119582).then_some(purify)
}

fn selected(world: &World, caster: Entity, spell: u32) -> bool {
    world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == spell)
    })
}

fn purify(world: &mut World, caster: Entity, _: &SpellInfo) {
    if selected(world, caster, 455068) {
        combat::auras::apply_aura(
            world,
            &combat::auras::AuraApplication {
                caster,
                target: caster,
                spell_id: 455071,
                effect_mask: u32::MAX,
                source: combat::auras::AuraSource::Cast,
            },
        );
    }
    let cleared = combat::auras::stagger::purify(world, caster);
    if cleared <= 0.0 || !selected(world, caster, 383700) {
        return;
    }
    let heal = cleared * script_effect_value(world, caster, 383700, 0) / 100.0;
    // sc_monk.cpp3476–3485: Gai Plin uses the cleared amount without a second multiplier.
    let spell = world
        .resource::<SpellStore>()
        .get(383701)
        .expect("native Gai Plin heal")
        .clone();
    let ctx = EffectCtx {
        caster,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    effects::apply_fixed_heal(world, &ctx, caster, heal);
}
