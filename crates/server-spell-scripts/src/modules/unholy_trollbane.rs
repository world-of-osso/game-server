//! Trollbane chains the target; the owner's Scourge Strike shatters them.
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, RemoveReason};
use combat::spell_cast::{
    self, effects::EffectCtx, script_registry::SpellScriptHook, scripts::AfterCastHook,
    summon::SummonedCreature,
};
use combat::spell_info::SpellInfo;
use combat::stats::CharacterStats;

#[derive(Component)]
struct ChainsReady(f64);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::GuardianActionEligible(eligible),
    SpellScriptHook::AuraRollAfterCast(after_cast),
    SpellScriptHook::AfterPolicyDirectDamage(shatter),
];

fn eligible(world: &World, actor: Entity, spell: u32, _: Entity) -> bool {
    if spell != 444826 {
        return true;
    }
    world
        .get::<ChainsReady>(actor)
        .is_none_or(|ready| combat::auras::aura_now(world) >= ready.0)
}

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    (spell == 444826).then_some(start_cooldown)
}

fn start_cooldown(world: &mut World, actor: Entity, _: &SpellInfo) {
    let Some(summon) = world
        .get::<SummonedCreature>(actor)
        .filter(|summon| summon.spell_id == 444254)
    else {
        return;
    };
    let haste = world
        .get::<CharacterStats>(summon.owner)
        .expect("Trollbane owner stats")
        .haste_pct;
    // SimC76f8118 sc_death_knight.cpp5252–5267: Chains has a four-second hasted cooldown.
    let ready = combat::auras::aura_now(world) + 4.0 / (1.0 + f64::from(haste) / 100.0);
    world.entity_mut(actor).insert(ChainsReady(ready));
}

fn shatter(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool) {
    if ctx.spell.id != 55090 || !combat::auras::has_aura(world, ctx.caster, 444097) {
        return;
    }
    let rider = world.get::<ActiveAuras>(target).and_then(|auras| {
        auras
            .iter()
            .find(|aura| {
                aura.spell_id == 444826
                    && world
                        .get::<SummonedCreature>(aura.caster)
                        .is_some_and(|summon| {
                            summon.owner == ctx.caster && summon.spell_id == 444254
                        })
            })
            .map(|aura| aura.caster)
    });
    let Some(rider) = rider else {
        return;
    };
    // SimC76f8118 sc_death_knight.cpp12569–12573: a real Scourge Strike
    // consumes Trollbane's slow and makes the owner cast native Icy Fury.
    combat::auras::remove_auras(world, target, RemoveReason::Consumed, |aura| {
        aura.spell_id == 444826 && aura.caster == rider
    });
    if !spell_cast::try_cast_triggered(world, ctx.caster, 444834, Some(target), 1) {
        warn!(caster = ?ctx.caster, ?target, "Trollbane Icy Fury cast refused");
    }
}
