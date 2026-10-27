//! Infected Claws follows real permanent and lesser ghoul attacks.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self, effects::EffectCtx, script_registry::SpellScriptHook, summon::SummonedCreature,
};
use combat::spell_info::SpellStore;
use shared::components::Npc;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::MeleeResidual(melee_hit),
    SpellScriptHook::SpellResidual(spell_hit),
];

fn melee_hit(world: &mut World, actor: Entity, target: Entity, damage: f32) {
    if damage > 0.0 {
        trigger(world, actor, target);
    }
}

fn spell_hit(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, damage: f32) {
    if damage > 0.0 && matches!(ctx.spell.id, 91776 | 91778 | 199373 | 1278150) {
        trigger(world, ctx.caster, target);
    }
}

fn trigger(world: &mut World, actor: Entity, target: Entity) {
    let Some(npc) = world.get::<Npc>(actor) else {
        return;
    };
    let owner = match npc.template_id {
        26125 => world
            .get::<combat::hunter_pet::HunterPet>(actor)
            .map(|pet| pet.owner),
        237409 => world
            .get::<SummonedCreature>(actor)
            .map(|summon| summon.owner),
        _ => None,
    };
    let Some(owner) = owner else {
        return;
    };
    if !combat::auras::has_aura(world, owner, 207272) {
        return;
    }
    // SimC76f8118 sc_death_knight.cpp3193–3215/3238/3257/3506:
    // ghoul attacks roll the actual Infected Claws driver's native proc chance.
    let chance = world
        .resource::<SpellStore>()
        .get(1241792)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("native Infected Claws proc options")
        .proc_chance;
    if combat::random::random::<f32>() * 100.0 >= chance as f32 {
        return;
    }
    if !spell_cast::try_cast_triggered(world, owner, 1241786, Some(target), 1) {
        warn!(?owner, ?target, "native Infected Claws application refused");
    }
}
