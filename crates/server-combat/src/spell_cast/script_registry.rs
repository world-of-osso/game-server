//! Downstream script dispatch. The ordered module slices are supplied by the host.
//!
//! Lookup order is part of the combat contract:
//! - `NativeAfterCast`: first matching module wins, before the native spell match.
//! - `AuraRollAfterCast`: all matching modules run after native, policies,
//!   spellslinger and batch9, and before resource-value hooks.
//! - `OnCast`: all modules run before havoc, combo tracking and elemental launch.
//! - `AfterPayment`: all modules run after successful payment and before gargoyles.
//! - `AfterResourceValues`: all matching modules run after resource-value hooks.
//! - `TargetPlan`: first matching module wins before combat's native target plans.
//! - `PeriodicPrimaryDamage`: all modules run at the Explosive primary-tick boundary.
//! - `TargetDamageMultiplier`: factors run at the Giantkiller damage boundary.
//! - `AfterSpenderStacks`: after foreground Crusade stacks and the corresponding
//!   background Storm callback; before Empyrean Legacy or echo policy dispatch.
//! - `AuraRedirect`: first redirect consumes application before native aura dispatch.
//! - `EffectHit`, `Hit`, `AreaFilter`, `CriticalChance`: first matching callback,
//!   before the retained native lookup. Target handling uses the same registry copy.
//! - `ContextHit` and `DamageAmount`: first match after generic policy lookup.
//! - `CheckTarget`: before native target checks; first error stops admission.
//! - `AutoRepeat`: after the actual Auto Shot launch.
//! - `EarlyAuraRegistration`: startup aura scripts, before native registration.
//! Within a stage, traverse module slices and entries from first to last. Never sort
//! or deduplicate them. Adding a policy to an existing module changes only that module.

mod policy;
pub use policy::{ChannelMomentum, Policy, Relationship};

use bevy::prelude::*;

use super::{CostPool, TargetPlan, effects::EffectCtx, scripts::AfterCastHook, targets::Relation};
use crate::spell_info::SpellInfo;

pub type TargetPlanHook = fn(&mut World, Entity, &SpellInfo, Option<Entity>) -> TargetPlan;
pub type TargetPlanLookup = fn(u32) -> Option<(Relation, TargetPlanHook)>;
pub type PeriodicPrimaryDamageHook = fn(&mut World, &EffectCtx<'_>, Entity, f32);
pub type TargetDamageMultiplierHook = fn(&World, Entity, Entity, &SpellInfo) -> f32;
pub type AuraRedirectHook = fn(&mut World, Entity, Entity, u32, u8) -> bool;

pub type AfterCastLookup = fn(u32) -> Option<AfterCastHook>;
pub type OnCastHook = fn(&mut World, Entity, u32, u8);
pub type AfterPaymentHook = fn(&mut World, Entity, &[(CostPool, f32)]);
pub type HitHook = fn(&mut World, Entity, Entity, u8);
pub type EffectHitLookup = fn(u32, u8) -> Option<HitHook>;
pub type HitLookup = fn(u32) -> Option<HitHook>;
pub type ContextHitHook = fn(&mut World, &EffectCtx<'_>, Entity);
pub type ContextHitLookup = fn(u32) -> Option<ContextHitHook>;
pub type AreaTargetFilter = fn(&World, &SpellInfo, Option<Entity>, &mut Vec<Entity>);
pub type AreaFilterLookup = fn(u32, u8) -> Option<AreaTargetFilter>;
pub type CheckTargetHook = fn(&World, Entity, u32, Entity) -> Result<(), super::CastFailure>;
pub type DamageAmountHook = fn(&World, Entity, Entity, f32) -> f32;
pub type DamageAmountLookup = fn(u32) -> Option<DamageAmountHook>;
pub type AutoRepeatHook = fn(&mut World, Entity, Entity);
pub type CriticalChanceHook = fn(&World, Entity, Entity, u32) -> Option<f32>;
pub type AuraRegistrationHook = fn(&mut crate::auras::scripts::AuraScripts);

#[derive(Clone, Copy)]
pub enum SpellScriptHook {
    Policies(&'static [Policy]),
    NativeAfterCast(AfterCastLookup),
    AuraRollAfterCast(AfterCastLookup),
    OnCast(OnCastHook),
    AfterPayment(AfterPaymentHook),
    AfterResourceValues(AfterCastLookup),
    TargetPlan(TargetPlanLookup),
    PeriodicPrimaryDamage(PeriodicPrimaryDamageHook),
    TargetDamageMultiplier(TargetDamageMultiplierHook),
    AfterSpenderStacks(AfterCastHook),
    AuraRedirect(AuraRedirectHook),
    EffectHit(EffectHitLookup),
    Hit(HitLookup),
    ContextHit(ContextHitLookup),
    AreaFilter(AreaFilterLookup),
    CheckTarget(CheckTargetHook),
    DamageAmount(DamageAmountLookup),
    AutoRepeat(AutoRepeatHook),
    CriticalChance(CriticalChanceHook),
    EarlyAuraRegistration(AuraRegistrationHook),
}

#[derive(Clone, Copy, Resource)]
pub struct SpellScriptRegistry {
    modules: &'static [&'static [SpellScriptHook]],
}

impl SpellScriptRegistry {
    pub const fn new(modules: &'static [&'static [SpellScriptHook]]) -> Self {
        Self { modules }
    }

    fn hooks(self) -> impl Iterator<Item = &'static SpellScriptHook> {
        self.modules.iter().flat_map(|module| module.iter())
    }

    pub fn effect_hit(self, spell: u32, index: u8) -> Option<HitHook> {
        self.hooks().find_map(|hook| match hook {
            SpellScriptHook::EffectHit(lookup) => lookup(spell, index),
            _ => None,
        })
    }

    pub fn register_early_auras(self, scripts: &mut crate::auras::scripts::AuraScripts) {
        for hook in self.hooks() {
            if let SpellScriptHook::EarlyAuraRegistration(register) = hook {
                register(scripts);
            }
        }
    }

    pub fn policies(self) -> impl Iterator<Item = &'static Policy> {
        self.hooks().flat_map(|hook| match hook {
            SpellScriptHook::Policies(policies) => *policies,
            _ => &[],
        })
    }
}

impl Default for SpellScriptRegistry {
    fn default() -> Self {
        // Unit tests compile the exact downstream module sources against this test
        // crate's types. Production and external fixtures install the host registry.
        #[cfg(test)]
        return Self::new(fixtures::MODULES);
        #[cfg(not(test))]
        Self::new(&[])
    }
}

pub fn policies(world: &World) -> impl Iterator<Item = &'static Policy> + use<> {
    world.resource::<SpellScriptRegistry>().policies()
}

pub fn native_after_cast(world: &World, spell: u32) -> Option<AfterCastHook> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::NativeAfterCast(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn aura_roll_after_cast(
    world: &World,
    spell: u32,
) -> impl Iterator<Item = AfterCastHook> + use<> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(move |hook| match hook {
            SpellScriptHook::AuraRollAfterCast(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn on_cast(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::OnCast(run) = hook {
            run(world, caster, spell, depth);
        }
    }
}

pub fn after_payment(world: &mut World, caster: Entity, costs: &[(CostPool, f32)]) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterPayment(run) = hook {
            run(world, caster, costs);
        }
    }
}

pub fn after_resource_values(
    world: &World,
    spell: u32,
) -> impl Iterator<Item = AfterCastHook> + use<> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(move |hook| match hook {
            SpellScriptHook::AfterResourceValues(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn target_plan(world: &World, spell: u32) -> Option<(Relation, TargetPlanHook)> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::TargetPlan(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn periodic_primary_damage(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    amount: f32,
) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::PeriodicPrimaryDamage(run) = hook {
            run(world, ctx, target, amount);
        }
    }
}

pub fn target_damage_multiplier(
    world: &World,
    caster: Entity,
    target: Entity,
    spell: &SpellInfo,
) -> f32 {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .filter_map(|hook| match hook {
            SpellScriptHook::TargetDamageMultiplier(run) => Some(run(world, caster, target, spell)),
            _ => None,
        })
        .product()
}

pub fn after_spender_stacks(world: &mut World, caster: Entity, spell: &SpellInfo) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AfterSpenderStacks(run) = hook {
            run(world, caster, spell);
        }
    }
}

pub fn aura_redirect(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: u32,
    depth: u8,
) -> bool {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .any(|hook| match hook {
            SpellScriptHook::AuraRedirect(run) => run(world, caster, target, spell, depth),
            _ => false,
        })
}

pub fn hit(world: &World, spell: u32) -> Option<HitHook> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::Hit(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn context_hit(world: &World, spell: u32) -> Option<ContextHitHook> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::ContextHit(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn area_filter(world: &World, spell: u32, index: u8) -> Option<AreaTargetFilter> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::AreaFilter(lookup) => lookup(spell, index),
            _ => None,
        })
}

pub fn check_target(
    world: &World,
    caster: Entity,
    spell: u32,
    target: Entity,
) -> Result<(), super::CastFailure> {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::CheckTarget(run) = hook {
            run(world, caster, spell, target)?;
        }
    }
    Ok(())
}

pub fn damage_amount(world: &World, spell: u32) -> Option<DamageAmountHook> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::DamageAmount(lookup) => lookup(spell),
            _ => None,
        })
}

pub fn auto_repeat(world: &mut World, caster: Entity, target: Entity) {
    for hook in world.resource::<SpellScriptRegistry>().hooks() {
        if let SpellScriptHook::AutoRepeat(run) = hook {
            run(world, caster, target);
        }
    }
}

pub fn critical_chance(world: &World, caster: Entity, target: Entity, spell: u32) -> Option<f32> {
    world
        .resource::<SpellScriptRegistry>()
        .hooks()
        .find_map(|hook| match hook {
            SpellScriptHook::CriticalChance(run) => run(world, caster, target, spell),
            _ => None,
        })
}

#[cfg(test)]
use crate as combat;
#[cfg(test)]
#[path = "../../../server-spell-scripts/src/modules.rs"]
mod fixtures;
