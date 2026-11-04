//! SimC76f8118 sc_druid.cpp8372–8459: MID2 Starfall adds an instant native area.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self,
    effects::EffectCtx,
    script_registry::{ContextHitHook, ScriptReach, SpellScriptHook},
    targets,
};
use combat::spell_info::{SpellInfo, SpellStore};

const STARFALL: u32 = 191034;
const TWO_PIECE: u32 = 1296603;
const INSTANT_DRIVER: u32 = 1301747;
const INSTANT_DAMAGE: u32 = 1301742;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SuccessfulCastRelationships(after_starfall),
    SpellScriptHook::ContextHit(instant_hit),
    SpellScriptHook::DeclaredReach(declared_reach),
];

fn after_starfall(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id == STARFALL && combat::auras::has_aura(world, caster, TWO_PIECE) {
        spell_cast::cast_triggered(world, caster, INSTANT_DRIVER, Some(caster), 1);
    }
}

fn instant_hit(spell: u32) -> Option<ContextHitHook> {
    (spell == INSTANT_DRIVER).then_some(damage_recipient)
}

fn damage_recipient(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    // Each recipient of native 1301747 gets its own unsplit 1301742 impact.
    spell_cast::cast_triggered(
        world,
        ctx.caster,
        INSTANT_DAMAGE,
        Some(target),
        ctx.depth + 1,
    );
}

fn declared_reach(world: &World, _: Entity, spell: &SpellInfo) -> ScriptReach {
    if spell.id != STARFALL {
        return ScriptReach::None;
    }
    let driver = world
        .resource::<SpellStore>()
        .get(INSTANT_DRIVER)
        .expect("native MID2 Starfall instant driver");
    ScriptReach::caster(targets::radius(&driver.effects[0], 1))
}
