//! Flametouched empowers actual summoned Dreadstalkers.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::summon::SummonedCreature;
use shared::components::Npc;

const FLAMETOUCHED: u32 = 453699;
const FEROCITY: u32 = 453704;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::SummonInitialized(on_summon)];

// SimC76f8118 pets.cpp1223–1226: each Dreadstalker gets its native Ferocity aura.
fn on_summon(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<SummonedCreature>(actor) else {
        return;
    };
    if !matches!(summon.spell_id, 193331 | 193332)
        || world.get::<Npc>(actor).is_none_or(|npc| npc.template_id != 98035)
    {
        return;
    }
    let owner = summon.owner;
    if combat::auras::has_aura(world, owner, FLAMETOUCHED)
        && !combat::spell_cast::try_cast_triggered(world, owner, FEROCITY, Some(actor), 1)
    {
        warn!(?owner, ?actor, "native Flametouched aura refused");
    }
}
