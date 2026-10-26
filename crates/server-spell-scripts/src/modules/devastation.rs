//! Explicit missing-script relationships; native records retain payload ownership.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::{Policy, Relationship, SpellScriptHook};
use combat::spell_cast::scripts::{reduce_cooldown, script_effect_value};

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::Policies(POLICIES),
    SpellScriptHook::PeriodicPrimaryDamage(causality_tick),
];

fn causality_tick(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    _amount: f32,
    _crit: bool,
) {
    const CAUSALITY: u32 = 375777;
    let primary_tick = ctx.spell.id == 356995 && ctx.spell.cast_primary_target == Some(target);
    if !primary_tick {
        return;
    }
    let devastation = world
        .get::<ActiveSpec>(ctx.caster)
        .is_some_and(|spec| spec.0 == 1467);
    if !devastation || !combat::auras::has_aura(world, ctx.caster, CAUSALITY) {
        return;
    }
    // SIMC_DEVASTATION_CAUSALITY: sc_evoker.cpp6694–6699. Native375777
    // effect0 owns the negative millisecond delta; secondary recipients do not repeat it.
    let seconds = -f64::from(script_effect_value(world, ctx.caster, CAUSALITY, 0)) / 1000.0;
    for empower in [357208, 359073] {
        reduce_cooldown(world, ctx.caster, empower, seconds);
    }
}

const POLICIES: &[Policy] = &[
    // SimC76f8118 sc_evoker.cpp6584–6595: one successful Disintegrate
    // spends one earned Mass Disintegrate use, not one per secondary recipient.
    Policy {
        name: "SIMC_DEVASTATION_MASS_DISINTEGRATE_CONSUMER",
        spec: 1467,
        talent: 436335,
        relationship: Relationship::SpendSelfAuraStackAfterCast {
            parent: 356995,
            payload: 436336,
        },
    },
    // SimC76f8118 sc_evoker.cpp6241–6245: Deep Breath's main impact
    // executes native353759; DB2 supplies its periodic damage and recipients.
    Policy {
        name: "SIMC_DEVASTATION_DEEP_BREATH_PAYLOAD",
        spec: 1467,
        talent: 357210,
        relationship: Relationship::TriggerPrimaryAfterCast {
            parent: 357210,
            payload: 353759,
        },
    },
    Policy {
        name: "SIMC_DEVASTATION_MANEUVERABLE_DEEP_BREATH_PAYLOAD",
        spec: 1467,
        talent: 433871,
        relationship: Relationship::TriggerPrimaryAfterCast {
            parent: 433874,
            payload: 353759,
        },
    },
];
