//! Native Assassination Deathstalker, Internal Bleeding and Implacable children.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    effects::EffectCtx,
    script_registry::{BeforeEnemyContextHitHook, SpellScriptHook},
};
use combat::spell_info::SpellInfo;

#[path = "assassination_runtime/rogue_children.rs"]
mod rogue_children;
#[path = "assassination_runtime/rogue_hero.rs"]
mod rogue_hero;

pub const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::PolicyContextRequired(needs_context),
    SpellScriptHook::PreparePolicySpell(rogue_hero::prepare),
    SpellScriptHook::BeforePolicyPrimaryHit(on_primary_hit),
    SpellScriptHook::RogueAuraRegistration(rogue_children::register),
    SpellScriptHook::BeforeEnemyContextHit(poison_hit),
];

fn needs_context(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    rogue_hero::needs_context(world, caster, spell)
        || rogue_children::needs_context(world, caster, spell.id)
}

fn on_primary_hit(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    rogue_hero::on_hit(world, ctx, target);
    rogue_children::on_hit(world, ctx, target);
}

fn poison_hit(spell: u32) -> Option<BeforeEnemyContextHitHook> {
    combat::auras::rogue::LETHAL_POISON_HITS
        .contains(&spell)
        .then_some(rogue_children::poison_hit)
}
