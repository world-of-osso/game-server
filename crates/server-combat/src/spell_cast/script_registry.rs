//! Downstream script dispatch. The ordered module slices are supplied by the host.
//!
//! Lookup order is part of the combat contract:
//! - `NativeAfterCast`: first matching module wins, before the native spell match.
//! - `AuraRollAfterCast`: all matching modules run after native, policies,
//!   spellslinger and batch9, and before resource-value hooks.
//! - `OnCast`: all modules run before havoc, combo tracking and elemental launch.
//! - `AfterPayment`: all modules run after successful payment and before gargoyles.
//! Within a stage, traverse module slices and entries from first to last. Never sort
//! or deduplicate them. Adding a policy to an existing module changes only that module.

mod policy;
pub use policy::{ChannelMomentum, Policy, Relationship};

use bevy::prelude::*;

use super::{CostPool, scripts::AfterCastHook};

pub type AfterCastLookup = fn(u32) -> Option<AfterCastHook>;
pub type OnCastHook = fn(&mut World, Entity, u32, u8);
pub type AfterPaymentHook = fn(&mut World, Entity, &[(CostPool, f32)]);

#[derive(Clone, Copy)]
pub enum SpellScriptHook {
    Policies(&'static [Policy]),
    NativeAfterCast(AfterCastLookup),
    AuraRollAfterCast(AfterCastLookup),
    OnCast(OnCastHook),
    AfterPayment(AfterPaymentHook),
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

#[cfg(test)]
use crate as combat;
#[cfg(test)]
#[path = "../../../server-spell-scripts/src/modules.rs"]
mod fixtures;
