//! Fiery Brand's landed impact applies its target DoT and defensive buff.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{effects::EffectCtx, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ResolvedDamage(impact)];

fn impact(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool, _: f32) {
    if ctx.spell.id != 204021 {
        return;
    }
    // SimC76f8118 sc_demon_hunter.cpp4280–4358 applies the DoT to the
    // impacted target and the separate damage-taken buff to the demon hunter.
    for (recipient, effect_mask) in [(target, 0b10110), (ctx.caster, 0b1)] {
        combat::auras::apply_aura(
            world,
            &combat::auras::AuraApplication {
                caster: ctx.caster,
                target: recipient,
                spell_id: 207771,
                effect_mask,
                source: combat::auras::AuraSource::Cast,
            },
        );
    }
}
