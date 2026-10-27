//! Source-qualified spell scripts downstream of the combat engine.
use server_combat as combat;

use bevy::prelude::*;
use combat::spell_cast::script_registry::SpellScriptRegistry;

pub mod bombardments;
pub mod modules;
pub use modules::demon_hunter_fragments as soul_fragments;

/// Install before the first cast, in both the server and transport-free simulation.
pub struct SpellScriptsPlugin;

impl Plugin for SpellScriptsPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(registry());
        app.add_systems(FixedUpdate, soul_fragments::update);
    }
}

pub const fn registry() -> SpellScriptRegistry {
    SpellScriptRegistry::new(modules::MODULES)
}

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

#[cfg(test)]
mod policy_order_tests;
#[cfg(test)]
mod registry_target_tests;
#[cfg(test)]
mod runtime_stage_tests;
#[cfg(test)]
mod tests;
