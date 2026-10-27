//! Diabolist demons: Chaos Salvo's channel launches its native missile each tick,
//! Demonology scales the demons' damage and Wicked Cleave applies Cloven Soul.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::script_registry::{ContextHitHook, EnemyContextHitHook, SpellScriptHook};
use combat::spell_cast::summon::SummonedCreature;
use combat::spell_cast::{self, effects::EffectCtx};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

const DIABOLIC_RITUAL: u32 = 428514;
const CHAOS_SALVO: u32 = 432569;
const CHAOS_SALVO_MISSILE: u32 = 432592;
const WICKED_CLEAVE: u32 = 432120;
const CHAOS_SALVO_DAMAGE: u32 = 432596;
const FELSEEKER_DAMAGE: u32 = 434404;
const CLOVEN_SOULS: u32 = 428517;
const CLOVEN_SOUL: u32 = 434424;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ChannelPeriodic(periodic_trigger),
    SpellScriptHook::TargetDamageMultiplier(demonology_multiplier),
    SpellScriptHook::EnemyContextHit(cleave_hit),
];

fn periodic_trigger(spell: u32, index: u8) -> Option<ContextHitHook> {
    (spell == CHAOS_SALVO && index == 0).then_some(launch_salvo)
}

// SimC76f8118 pets.cpp2717–2758 and init.cpp545–547: each channel tick launches
// native432592, whose own trigger missile delivers the432596 damage.
fn launch_salvo(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    spell_cast::cast_triggered(
        world,
        ctx.caster,
        CHAOS_SALVO_MISSILE,
        Some(target),
        ctx.depth + 1,
    );
}

fn demonology_owner(world: &World, demon: Entity) -> Option<Entity> {
    let owner = world.get::<SummonedCreature>(demon)?.owner;
    (world
        .get::<ActiveSpec>(owner)
        .is_some_and(|spec| spec.0 == 266)
        && auras::has_aura(world, owner, DIABOLIC_RITUAL))
    .then_some(owner)
}

// pets.cpp2639–2652/2730–2743/2825–2838: Demonology multiplies each demon's
// damage by 1 + native Diabolic Ritual effect3.
fn demonology_multiplier(world: &World, caster: Entity, _: Entity, spell: &SpellInfo) -> f32 {
    if !matches!(spell.id, WICKED_CLEAVE | CHAOS_SALVO_DAMAGE | FELSEEKER_DAMAGE) {
        return 1.0;
    }
    demonology_owner(world, caster).map_or(1.0, |owner| {
        1.0 + spell_cast::scripts::script_effect_value(world, owner, DIABOLIC_RITUAL, 2) / 100.0
    })
}

fn cleave_hit(spell: u32) -> Option<EnemyContextHitHook> {
    (spell == WICKED_CLEAVE).then_some(apply_cloven_soul)
}

// pets.cpp2670–2676: each Wicked Cleave impact triggers the owner's Cloven Soul.
fn apply_cloven_soul(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let Some(owner) = world
        .get::<SummonedCreature>(ctx.caster)
        .map(|summon| summon.owner)
    else {
        return;
    };
    if auras::has_aura(world, owner, CLOVEN_SOULS)
        && !spell_cast::try_cast_triggered(world, owner, CLOVEN_SOUL, Some(target), ctx.depth + 1)
    {
        warn!(?owner, ?target, "Native Cloven Soul refused");
    }
}
