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
    // Magus triggers the selected Unholy Aura haste stack on its owner.
    if !spell_cast::try_cast_triggered(world, owner, 1268917, Some(owner), 1) {
        warn!(?owner, "native Unholy Aura haste buff refused");
    }
}
