//! SimC76f8118 sc_rogue.cpp3411–3419,10282,10615–10618:
//! landed weapon swings dispatch Hunt Them Down to their actual owned-mark victim.
use super::{cast_relationships::has_talent, combat};
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::MeleeResidual(after_white_hit)];
const HUNT_THEM_DOWN: u32 = 457054;
const MARK: u32 = 457129;
const DAMAGE: u32 = 457193;

fn after_white_hit(world: &mut World, caster: Entity, target: Entity, _damage: f32) {
    if !has_talent(world, caster, HUNT_THEM_DOWN) {
        return;
    }
    let now = spell_cast::now(world);
    let marked = world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.spell_id == MARK
                && aura.caster == caster
                && aura.expires_at.is_none_or(|expires| expires > now)
        })
    });
    if marked {
        spell_cast::cast_triggered(world, caster, DAMAGE, Some(target), 1);
    }
}
