//! The module order is stable; dispatch-stage semantics live in combat's
//! `spell_cast::script_registry`. New policies belong in their module's const slice.
use super::combat;
use combat::spell_cast::script_registry::SpellScriptHook;

#[path = "modules/frost.rs"]
mod frost;

#[path = "modules/after_cast_auras.rs"]
mod after_cast_auras;
#[path = "modules/aura_rolls.rs"]
mod aura_rolls;
#[path = "modules/beast_mastery.rs"]
mod beast_mastery;
#[path = "modules/cast_grants.rs"]
mod cast_grants;
#[path = "modules/cast_resets.rs"]
mod cast_resets;
#[path = "modules/devastation.rs"]
mod devastation;
#[path = "modules/essence_burst.rs"]
mod essence_burst;
#[path = "modules/evoker_tier.rs"]
mod evoker_tier;
#[path = "modules/giantkiller.rs"]
mod giantkiller;
#[path = "modules/havoc.rs"]
mod havoc;
#[path = "modules/marksmanship_explosive.rs"]
mod marksmanship_explosive;
#[path = "modules/paid_cooldowns.rs"]
mod paid_cooldowns;
#[path = "modules/ret_resonance.rs"]
mod ret_resonance;
#[path = "modules/retribution.rs"]
mod retribution;
#[path = "modules/rogue.rs"]
mod rogue;
#[path = "modules/warrior.rs"]
mod warrior;
#[path = "modules/windwalker_momentum.rs"]
mod windwalker_momentum;
#[path = "modules/windwalker_zenith.rs"]
mod windwalker_zenith;

pub const MODULES: &[&[SpellScriptHook]] = &[
    devastation::HOOKS,
    beast_mastery::HOOKS,
    windwalker_zenith::HOOKS,
    warrior::HOOKS,
    windwalker_momentum::HOOKS,
    retribution::HOOKS,
    rogue::HOOKS,
    frost::HOOKS,
    cast_resets::HOOKS,
    after_cast_auras::HOOKS,
    aura_rolls::HOOKS,
    cast_grants::HOOKS,
    paid_cooldowns::HOOKS,
    essence_burst::HOOKS,
    havoc::HOOKS,
    evoker_tier::HOOKS,
    giantkiller::HOOKS,
    marksmanship_explosive::HOOKS,
    ret_resonance::HOOKS,
];
