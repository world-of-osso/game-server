//! Antoran Inquisitor's native Mind Sear begins with an immediate tick.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::summon::SummonedCreature;
use shared::components::Npc;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::OnCast(initial_tick)];

fn initial_tick(world: &mut World, actor: Entity, spell: u32, depth: u8) {
    if spell != 1280457 || world.get::<Npc>(actor).is_none_or(|npc| npc.template_id != 258242) {
        return;
    }
    let Some(owner) = world.get::<SummonedCreature>(actor).map(|summon| summon.owner) else { return; };
    let Some(target) = world.get::<combat::networking::CombatTarget>(owner).and_then(|target| target.0) else { return; };
    // SimC76f8118 pets.cpp1978–2002: the unhasted 10s channel ticks at zero.
    combat::spell_cast::cast_triggered(world, actor, 1280460, Some(target), depth + 1);
}
