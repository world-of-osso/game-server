//! Native Frost launch procs, execution supply and delayed callbacks.
use super::combat;
use combat::spell_cast::script_registry::{self, SpellScriptHook};
use combat::spell_cast::scripts::{AfterCastHook, script_effect_value};
use script_registry::conjure_count;

#[path = "frost_runtime/frost_mage_events.rs"]
mod frost_mage_events;
#[path = "frost_runtime/frost_mage_procs.rs"]
mod frost_mage_procs;
#[path = "frost_runtime/frost_mage_supply.rs"]
mod frost_mage_supply;

pub const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EffectHandler(frost_mage_procs::effect_handler),
    SpellScriptHook::OwnsPolicyProc(frost_mage_procs::owns_proc),
    SpellScriptHook::OwnsPolicyProc(frost_mage_supply::owns_proc),
    SpellScriptHook::SnapshotLaunchAuras(frost_mage_procs::snapshot_launch_auras),
    SpellScriptHook::TriggeredChildDamageMultiplier(frost_mage_procs::flurry_damage_multiplier),
    SpellScriptHook::ProcAfterCast(frost_mage_procs::after_cast_hook),
    SpellScriptHook::SupplyAfterCast(frost_mage_supply::after_cast_hook),
    SpellScriptHook::ResolvePendingScriptEvents(frost_mage_events::resolve),
    SpellScriptHook::AfterLaunchProcSupply(frost_mage_supply::after_launch),
];
