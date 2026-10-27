//! Selected Rune of the Apocalypse follows the owner's real ghoul impacts.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook, summon::SummonedCreature};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::SpellResidual(on_hit)];

fn on_hit(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, amount: f32) {
    if amount <= 0.0 { return; }
    let owner = match ctx.spell.id {
        390220 => ctx.caster,
        91778 | 199373 | 1278150 | 1247379 | 1300330 => {
            if let Some(pet) = world.get::<combat::hunter_pet::HunterPet>(ctx.caster) { pet.owner }
            else if let Some(pet) = world.get::<SummonedCreature>(ctx.caster) { pet.owner }
            else { return; }
        }
        _ => return,
    };
    if !combat::auras::has_aura(world, owner, 327087) { return; }
    // SimC76f8118 sc_death_knight.cpp3260/3508/3584/3617/12159/13972–13996:
    // select one of four authentic Rune of the Apocalypse effects per impact.
    let effects = [327095, 327092, 327093, 327096];
    let index = (combat::random::random::<f32>() * effects.len() as f32) as usize;
    let spell = effects[index];
    if !spell_cast::try_cast_triggered(world, owner, spell, Some(target), ctx.depth + 1) {
        warn!(?owner, ?target, spell, "native Rune of the Apocalypse effect refused");
    }
}
