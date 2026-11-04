//! TC a352b1fa spell_evoker.cpp322–350: one native Azure talent roll per completed cast.
//! Dragonrage guarantee: native70448 375087s2; SimC76f8118 sc_evoker.cpp5625/7012.
//! Azure Sweep shares the base Azure execute producer: sc_evoker.cpp5603–5628/5671–5678.
//! Ruby and Preservation 369297 effect 0 roll per actual Living Flame hit (`evoker_flames`,
//! sc_evoker.cpp7004–7023); Preservation 369297 effect 1 rolls once per Reversion cast.
use super::combat;
use combat::random as rand;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::AfterCastHook;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(hook),
    SpellScriptHook::AfterForegroundSpender(cast_twin_flame),
    SpellScriptHook::AfterForegroundSpender(cast_set_living_flame),
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::AfterForegroundSpecial(roll_reversion_essence_burst),
    SpellScriptHook::ResolvedHeal(titanic_precision_heal),
    SpellScriptHook::AfterPolicyDirectDamage(titanic_precision_damage),
];

fn hook(spell: u32) -> Option<AfterCastHook> {
    match spell {
        AZURE_STRIKE | AZURE_SWEEP => Some(roll_azure_essence_burst),
        _ => None,
    }
}
use bevy::prelude::*;

use combat::auras::{ActiveAuras, has_aura};
use combat::spell_info::SpellInfo;

pub(super) const AZURE_STRIKE: u32 = 362969;
const AZURE_SWEEP: u32 = 1265872;
const AZURE_ESSENCE_BURST: u32 = 375721;
const RUBY_ESSENCE_BURST: u32 = 376872;
const ESSENCE_BURST: u32 = 359618;
const DRAGONRAGE: u32 = 375087;
const PRESERVATION_ESSENCE_BURST_TALENT: u32 = 369297;
const PRESERVATION_ESSENCE_BURST: u32 = 369299;
const TITANIC_PRECISION: u32 = 445625;
const ESSENCE_WELL: u32 = 1265993;

const PRESERVATION_SET_2PC: u32 = 1296641;
const PRESERVATION_SET_4PC: u32 = 1296642;
const VERDANT_EMBRACE: u32 = 360995;
const LIVING_FLAME: u32 = 361469;
const LIVING_FLAME_DAMAGE: u32 = 361500;
const LIVING_FLAME_HEAL: u32 = 361509;

fn consumes_essence_burst(spell: &SpellInfo) -> bool {
    spell
        .cast_charge_mods
        .iter()
        .flatten()
        .any(|modifier| modifier.aura_spell_id == 369299)
}

fn cast_twin_flame(world: &mut World, caster: Entity, spell: &SpellInfo, target: Option<Entity>) {
    if !has_aura(world, caster, 1265979) || !consumes_essence_burst(spell) {
        return;
    }
    let Some(target) = target else {
        warn!(?caster, "Twin Flame lacks a primary target");
        return;
    };
    let child = if combat::spell_cast::targets::relation_matches(
        world,
        caster,
        target,
        combat::spell_cast::targets::Relation::Ally,
    ) {
        1265991
    } else {
        1265980
    };
    if !combat::spell_cast::try_cast_triggered(world, caster, child, Some(target), 1) {
        warn!(?caster, ?target, child, "Twin Flame cast refused");
    }
}

/// Native 1296641 description: consuming Essence Burst sends forth a Living Flame at the target
/// (effect 0: 100% effectiveness), or at a nearby injured ally when the target is at full health.
fn cast_set_living_flame(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    target: Option<Entity>,
) {
    use combat::spell_cast::targets::{self, Relation};
    if !has_aura(world, caster, PRESERVATION_SET_2PC) || !consumes_essence_burst(spell) {
        return;
    }
    let Some(target) = target else {
        warn!(
            ?caster,
            "Preservation 2pc Living Flame lacks a primary target"
        );
        return;
    };
    let injured = |world: &World, unit: Entity| {
        world
            .get::<shared::components::Health>(unit)
            .is_some_and(|health| health.current < health.max)
    };
    let (child, recipient) = if !targets::relation_matches(world, caster, target, Relation::Ally) {
        (LIVING_FLAME_DAMAGE, target)
    } else if injured(world, target) {
        (LIVING_FLAME_HEAL, target)
    } else {
        let nearby = nearby_injured_ally(world, caster, target, injured);
        (LIVING_FLAME_HEAL, nearby.unwrap_or(target))
    };
    combat::spell_cast::cast_triggered(world, caster, child, Some(recipient), 1);
}

/// Nearest injured ally to the target within Living Flame's range.
fn nearby_injured_ally(
    world: &mut World,
    caster: Entity,
    target: Entity,
    injured: impl Fn(&World, Entity) -> bool,
) -> Option<Entity> {
    use combat::spell_cast::targets::{self, Relation};
    let center = targets::position(world, target)?;
    let spell = world
        .resource::<combat::spell_info::SpellStore>()
        .get(LIVING_FLAME)
        .expect("native Living Flame");
    let (_, range) = targets::spell_range(spell, true);
    targets::area_units(world, caster, center, range, Relation::Ally, 0)
        .into_iter()
        .find(|&unit| unit != target && injured(world, unit))
}

fn register(scripts: &mut combat::auras::scripts::AuraScripts) {
    scripts
        .0
        .entry(PRESERVATION_SET_4PC)
        .or_insert(combat::auras::scripts::AuraScript::default())
        .check_proc = Some(is_own_verdant_embrace);
}

/// Native 1296642 description: only Verdant Embrace can grant Essence Burst.
/// DB2 1296642 effect 1 has empty SpellClassMask fields, so the spell is matched by ID;
/// the generic successful-cast proc would otherwise accept any cast.
fn is_own_verdant_embrace(
    _: &World,
    aura: &combat::auras::AuraInstance,
    event: &combat::auras::procs::AuraEvent,
) -> bool {
    event.actor == aura.caster && event.spell_id == Some(VERDANT_EMBRACE)
}

fn roll_azure_essence_burst(world: &mut World, caster: Entity, _: &SpellInfo) {
    roll_essence_burst(world, caster, AZURE_ESSENCE_BURST, 0, ESSENCE_BURST);
}

/// One actual Living Flame hit: Ruby for Devastation, 369297 effect 0 for Preservation.
pub(super) fn roll_living_flame_essence_burst(world: &mut World, caster: Entity) {
    roll_essence_burst(world, caster, RUBY_ESSENCE_BURST, 0, ESSENCE_BURST);
    roll_preservation_essence_burst(world, caster, 0);
}

fn roll_preservation_essence_burst(world: &mut World, caster: Entity, effect: u8) {
    let talent = PRESERVATION_ESSENCE_BURST_TALENT;
    roll_essence_burst(world, caster, talent, effect, PRESERVATION_ESSENCE_BURST);
}

/// Native 369297 effect 1: Reversion has a 15% chance per cast. INFERRED_WCL vhKYp3J2BcVdQ7fa
/// and four other current pulls: Merithra's Blessing casts gain Essence Burst 33/39 times (gap).
fn roll_reversion_essence_burst(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if super::evoker_golden_hour::is_reversion(spell.id) {
        roll_preservation_essence_burst(world, caster, 1);
    }
}

fn titanic_precision_heal(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx<'_>,
    _: Entity,
    _: f32,
    _: f32,
    critical: bool,
    periodic: bool,
) {
    if ctx.spell.id == LIVING_FLAME_HEAL && critical && !periodic {
        roll_titanic_precision(world, ctx.caster);
    }
}

fn titanic_precision_damage(
    world: &mut World,
    ctx: &combat::spell_cast::effects::EffectCtx<'_>,
    _: Entity,
    critical: bool,
) {
    if ctx.spell.id == LIVING_FLAME_DAMAGE && critical {
        roll_titanic_precision(world, ctx.caster);
    }
}

/// SimC76f8118 sc_evoker.cpp5237–5261: a critical Living Flame hit rolls Essence Burst again.
/// Only Preservation's roll is native; Devastation's Ruby and Azure crit rolls are not.
fn roll_titanic_precision(world: &mut World, caster: Entity) {
    if has_aura(world, caster, TITANIC_PRECISION) {
        roll_preservation_essence_burst(world, caster, 0);
    }
}

/// Native 1265993: 50% per Dream Breath or Fire Breath release (SimC76f8118
/// sc_evoker.cpp5354–5361). Only Preservation's grant is native. INFERRED_WCL: retail gains
/// land up to ~3 s after the release; the native roll lands at it.
pub(super) fn roll_essence_well(world: &mut World, caster: Entity) {
    if has_aura(world, caster, PRESERVATION_ESSENCE_BURST_TALENT) {
        roll_essence_burst(world, caster, ESSENCE_WELL, 0, PRESERVATION_ESSENCE_BURST);
    }
}

fn roll_essence_burst(world: &mut World, caster: Entity, talent: u32, effect: u8, buff: u32) {
    let Some(chance) = talent_chance(world, caster, talent, effect) else {
        return;
    };
    let chance = if has_aura(world, caster, DRAGONRAGE) {
        combat::spell_cast::scripts::script_effect_value(world, caster, DRAGONRAGE, 1)
    } else {
        chance
    };
    if rand::random::<f32>() * 100.0 < chance {
        combat::spell_cast::cast_triggered(world, caster, buff, Some(caster), 1);
    }
}

fn talent_chance(world: &World, caster: Entity, talent: u32, effect: u8) -> Option<f32> {
    world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == talent)
        .map(|aura| {
            aura.effects
                .iter()
                .find(|candidate| candidate.index == effect)
                .expect("native Essence Burst talent retains its chance effect")
                .amount
        })
}
