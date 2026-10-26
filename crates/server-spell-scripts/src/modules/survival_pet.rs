//! Native Survival strikes use the already owned main pet and its shared combat path.
use super::cast_relationships::has_talent;
use super::combat;
use bevy::prelude::*;
use combat::auras::{ActiveAuras, procs};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::BeforeProcLaunch(strike_before_swipe),
    SpellScriptHook::ResolvedDamage(bleed_after_strike_crit),
    SpellScriptHook::OwnsPolicyProc(owns_strike_proc),
];

const STRIKE_AS_ONE: u32 = 1251717;
const STRIKE_DAMAGE: u32 = 1251779;
const SIC_EM: u32 = 1253137;
const SIC_EM_BLEED: u32 = 1253138;

const TIP: u32 = 260286;
const SWIPE: u32 = 1262293;
const SWIPE_RANK_THREE: u32 = 1259019;

fn owns_strike_proc(spell: u32) -> bool {
    // Only actual Tip consumption owns this ICD, not raw native melee hints.
    spell == STRIKE_AS_ONE
}

pub(super) fn after_tip_impact(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    // SimC76f8118 sc_hunter.cpp1367–1372/7944: normal Tip impact uses
    // the selected native aura's ICD, only after a real main pet executes.
    if !has_talent(world, ctx.caster, STRIKE_AS_ONE) {
        return;
    }
    let instance = world.get::<ActiveAuras>(ctx.caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == STRIKE_AS_ONE && aura.caster == ctx.caster)
            .map(|aura| aura.instance_id)
    });
    let Some(instance) = instance else {
        return;
    };
    if !procs::policy_proc_ready(world, ctx.caster, instance) {
        return;
    }
    if !launch_strike(world, ctx.caster, target, 1.0, ctx.depth + 1) {
        return;
    }
    if procs::start_policy_proc(world, ctx.caster, instance).is_none() {
        warn!(caster = ?ctx.caster, "Native Strike as One ICD could not start after execution");
    }
}

fn strike_before_swipe(world: &mut World, caster: Entity, spell: &SpellInfo) {
    // SimC76f8118 sc_hunter.cpp6181–6186/3384–3385: the tipped rank3
    // extra strike runs before the base execution and does not spend the normal ICD.
    if spell.id != SWIPE || !has_talent(world, caster, SWIPE_RANK_THREE) {
        return;
    }
    let tipped = world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == TIP && aura.caster == caster)
    });
    if !tipped || !has_talent(world, caster, STRIKE_AS_ONE) {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        return;
    };
    let effectiveness = script_effect_value(world, caster, SWIPE_RANK_THREE, 1) / 100.0;
    launch_strike(world, caster, target, effectiveness, 1);
}

fn bleed_after_strike_crit(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    critical: bool,
    _damage: f32,
) {
    // SimC76f8118 sc_hunter.cpp2893–2898: an actual main-pet strike
    // crit executes the selected native bleed from that same pet.
    if ctx.spell.id != STRIKE_DAMAGE || !critical {
        return;
    }
    let Some(owner) = world
        .get::<combat::hunter_pet::HunterPet>(ctx.caster)
        .map(|pet| pet.owner)
    else {
        return;
    };
    if read_main_pet(world, owner) != Some(ctx.caster) || !has_talent(world, owner, SIC_EM) {
        return;
    }
    spell_cast::cast_triggered(world, ctx.caster, SIC_EM_BLEED, Some(target), ctx.depth + 1);
}

fn read_main_pet(world: &World, owner: Entity) -> Option<Entity> {
    let pet = combat::hunter_pet::summoned_pet(world, owner)?;
    let owned = world
        .get::<combat::hunter_pet::HunterPet>(pet)
        .is_some_and(|pet| pet.owner == owner);
    if owned && spell_cast::targets::is_alive(world, pet) {
        Some(pet)
    } else {
        None
    }
}

fn launch_strike(
    world: &mut World,
    owner: Entity,
    target: Entity,
    effectiveness: f32,
    depth: u8,
) -> bool {
    let Some(pet) = read_main_pet(world, owner) else {
        return false;
    };
    let Some(mut spell) = spell_cast::load_triggered_spell(world, STRIKE_DAMAGE, depth) else {
        return false;
    };
    // Native1251779 supplies payload, modifiers and shared owner crit inheritance.
    spell.cast_damage_multiplier = Some(effectiveness);
    spell_cast::launch_triggered_spell(world, pet, &spell, Some(target), depth)
}
