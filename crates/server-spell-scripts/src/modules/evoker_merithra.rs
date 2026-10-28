//! Consume the native one-cast Reversion replacement.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    AuraInstance,
    procs::AuraEvent,
    scripts::{AuraScript, AuraScripts},
};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellStore;
use shared::components::PowerType;

const BLESSING: u32 = 1256581;
const OVERRIDE: u32 = 1256579;
const TALENT: u32 = 1256577;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterNativeOnCast(consume_override),
    SpellScriptHook::EarlyAuraRegistration(register),
];

fn register(scripts: &mut AuraScripts) {
    scripts
        .0
        .entry(TALENT)
        .or_insert(AuraScript::default())
        .check_proc = Some(is_essence_ability);
}

fn is_essence_ability(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    if event.actor != aura.caster {
        return false;
    }
    // Native talent1256577 description: Essence abilities can grant the upgrade.
    event
        .spell_id
        .and_then(|id| world.resource::<SpellStore>().get(id))
        .is_some_and(|spell| {
            spell.power_costs.iter().any(|cost| {
                cost.power_type == PowerType::Essence as i32
                    && (cost.mana_cost > 0 || cost.power_cost_pct > 0.0)
            })
        })
}

fn consume_override(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if spell != BLESSING || depth != 0 {
        return;
    }
    // Native talent1256577 description: only the next Reversion is upgraded.
    // Aura1256579 has no proc charges; its Attr9 override flag is client-only.
    combat::auras::remove_auras(
        world,
        caster,
        combat::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == OVERRIDE && aura.caster == caster,
    );
}
