//! Native Orb's once-per-real-pulse Fingers producer.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraApplication, AuraSource};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::{ContextHitHook, SpellScriptHook},
};
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::ContextHit(hit_hook)];

fn hit_hook(spell: u32) -> Option<ContextHitHook> {
    (spell == 84721).then_some(roll_fingers)
}

fn roll_fingers(world: &mut World, ctx: &EffectCtx, target: Entity) {
    if world
        .get::<ActiveSpec>(ctx.caster)
        .is_none_or(|spec| spec.0 != 64)
        || !auras::has_aura(world, ctx.caster, 385167)
    {
        return;
    }
    // SimC76f8118 sc_mage.cpp4141–4147: hit_any_target admits one native
    // Everlasting Frost roll per Orb bolt execution, not one per area recipient.
    // The completed native resolution ledger identifies its first actual hit,
    // including when the selected primary missed or the pulse has no primary.
    let first_hit = spell_cast::resolving_damage(world, ctx)
        .and_then(|hits| hits.first())
        .map(|hit| hit.0);
    if first_hit != Some(target) {
        return;
    }
    let chance = script_effect_value(world, ctx.caster, 385167, 1);
    if combat::random::random::<f32>() * 100.0 >= chance {
        return;
    }
    auras::apply_aura(
        world,
        &AuraApplication {
            caster: ctx.caster,
            target: ctx.caster,
            spell_id: 44544,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}
