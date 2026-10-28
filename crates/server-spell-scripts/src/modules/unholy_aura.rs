//! Real Magi grant the selected native Unholy Aura haste buff.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook, summon::SummonedCreature};
use shared::components::Npc;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::SummonInitialized(on_summon)];

fn on_summon(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<SummonedCreature>(actor) else {
        return;
    };
    if !matches!(summon.spell_id, 317776 | 1242294)
        || world
            .get::<Npc>(actor)
            .is_none_or(|npc| npc.template_id != 163366)
        || !combat::auras::has_aura(world, summon.owner, 377440)
    {
        return;
    }
    let owner = summon.owner;
    // SimC76f8118 sc_death_knight.cpp4541–4547/16273–16277: each real
    // Magus grants a haste stack valued by the selected talent's first effect.
    let haste = spell_cast::scripts::script_effect_value(world, owner, 377440, 0);
    if !spell_cast::cast_triggered_with_overrides(
        world,
        owner,
        1268917,
        Some(owner),
        1,
        spell_cast::TriggeredSpellOverrides {
            effect0_amount: Some(haste),
            ..Default::default()
        },
    ) {
        warn!(?owner, "native Unholy Aura haste buff refused");
    }
}
