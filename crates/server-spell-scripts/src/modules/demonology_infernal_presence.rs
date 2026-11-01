//! Selected Mark of F'harg gives each actual Charhound its native periodic aura.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::summon::SummonedCreature;
use shared::components::Npc;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SummonInitialized(on_summon),
    SpellScriptHook::AdditionalReachActions(autonomous_actions),
];

fn autonomous_actions(world: &World, actor: Entity) -> Vec<u32> {
    let charhound = world
        .get::<SummonedCreature>(actor)
        .is_some_and(|summon| summon.spell_id == 455476)
        && world
            .get::<Npc>(actor)
            .is_some_and(|npc| npc.template_id == 226269);
    if charhound && combat::auras::has_aura(world, actor, 428453) {
        vec![428455]
    } else {
        Vec::new()
    }
}

fn on_summon(world: &mut World, actor: Entity) {
    let Some(summon) = world.get::<SummonedCreature>(actor) else {
        return;
    };
    if summon.spell_id != 455476
        || world
            .get::<Npc>(actor)
            .is_none_or(|npc| npc.template_id != 226269)
    {
        return;
    }
    if combat::auras::has_aura(world, summon.owner, 455450)
        && !combat::spell_cast::try_cast_triggered(world, actor, 428453, Some(actor), 1)
    {
        warn!(?actor, "native Infernal Presence aura refused");
    }
}
