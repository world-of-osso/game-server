//! SimC76f8118 sc_rogue.cpp2819–2853/5508–5520/8501–8508:
//! earned Shadow Blades copies actual Rogue damage into native Shadow damage.
use super::{cast_relationships::has_talent, combat};
#[path = "subtlety_darkness.rs"]
mod subtlety_darkness;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{
    effects::{self, EffectCtx},
    script_registry::SpellScriptHook,
    scripts::script_effect_value,
};
use combat::spell_info::SpellStore;
use shared::components::ActiveSpec;

const BUFF: u32 = 121471;
const PAYLOAD: u32 = 279043;
const ROGUE_FAMILY: u32 = 8;
const SHADOW: u32 = 32;
// Pinned residual sources explicitly opt out; they already carry source damage.
const EXCLUDED: &[u32] = &[279043, 386081, 457236, 1309309];
pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ResolvedDamage(after_spell),
    SpellScriptHook::LandedDamagePolicy(after_white),
    SpellScriptHook::LateAuraRegistration(subtlety_darkness::register),
];

fn after_spell(world: &mut World, ctx: &EffectCtx, target: Entity, _: bool, damage: f32) {
    let rogue = ctx
        .spell
        .class_options
        .as_ref()
        .is_some_and(|family| family.spell_class_set == ROGUE_FAMILY);
    if rogue && !EXCLUDED.contains(&ctx.spell.id) {
        emit_shadow_blades(
            world,
            ctx.caster,
            target,
            damage,
            ctx.spell.school_mask(),
            ctx.depth + 1,
        );
    }
}

fn after_white(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: Option<u32>,
    damage: f32,
    _: bool,
) {
    if spell.is_none() {
        emit_shadow_blades(world, caster, target, damage, 1, 1);
    }
}

fn emit_shadow_blades(
    world: &mut World,
    caster: Entity,
    target: Entity,
    damage: f32,
    school: u32,
    depth: u8,
) {
    if damage <= 0.0
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 261)
    {
        return;
    }
    let Some(fraction) = read_blades_fraction(world, caster) else {
        return;
    };
    let amount = damage * fraction * shadow_school_multiplier(world, caster, school);
    let spell = world
        .resource::<SpellStore>()
        .get(PAYLOAD)
        .expect("native Shadow Blades payload")
        .clone();
    let ctx = EffectCtx {
        caster,
        spell: &spell,
        depth,
        dest: None,
        chain_multiplier: 1.0,
    };
    // SimC passes mitigated result_amount, already containing the original crit.
    effects::deal_copied_damage(world, &ctx, target, amount);
}

fn read_blades_fraction(world: &World, caster: Entity) -> Option<f32> {
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == BUFF && aura.caster == caster)?;
    Some(
        aura.effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("native Shadow Blades fraction")
            .amount
            / 100.0,
    )
}

fn shadow_school_multiplier(world: &World, caster: Entity, school: u32) -> f32 {
    if school & SHADOW != 0 {
        return 1.0;
    }
    // sc_rogue.cpp2080–2089: only physical/non-Shadow source damage
    // receives these native Shadow modifiers on its copied residual.
    [(382517, 0), (382017, 0), (382504, 1), (1268936, 1)]
        .into_iter()
        .filter(|&(talent, _)| has_talent(world, caster, talent))
        .map(|(talent, index)| 1.0 + script_effect_value(world, caster, talent, index) / 100.0)
        .product()
}
