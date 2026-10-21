//! Bounded scripts derived exclusively from local Mainline 12.1.0.69933 rows.
use crate::random as rand;
use bevy::prelude::*;
use shared::components::PowerType;

use super::script_effect_value;
use crate::auras::handlers::{PERIODIC_DAMAGE, PERIODIC_DAMAGE_PERCENT};
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::{self, targets};
use crate::spell_info::{SpellInfo, SpellStore};

pub(super) const DEMONS_BITE: u32 = 162243;
pub(super) const CHAOS_STRIKE: u32 = 162794;
pub(super) const DISINTEGRATE: u32 = 356995;
pub(super) const CHAOS_BOLT: u32 = 116858;
pub(super) const RAPTURE: u32 = 324536;
pub(super) const RAPTURE_DAMAGE: u32 = 324540;
pub(super) const SHURIKEN_STORM: u32 = 197835;
pub(super) const DEATHS_CARESS: u32 = 195292;
pub(super) const EPIDEMIC: u32 = 207317;
pub(super) const EPIDEMIC_DAMAGE: u32 = 212739;
pub(super) const LAVA_BURST_DAMAGE: u32 = 285452;
const INSATIABLE_HUNGER: u32 = 258876;
const CHAOS_REFUND_CHANCE: u32 = 197125;
const CHAOS_REFUND: u32 = 193840;
const DISINTEGRATE_SLOW: u32 = 370898;
const BONE_SHIELD: u32 = 195181;
const VIRULENT_PLAGUE: u32 = 191587;
const DREAD_PLAGUE: u32 = 1240996;
const OUTBREAK: u32 = 77575;
const FLAME_SHOCK: u32 = 188389;
const SHADOW_BLADES: u32 = 121471;
const PERIODIC_HEALTH_LEECH: u32 = 53;

/// 162243 names258876s3/s4 as additional Fury, gated by the applied rank.
pub(super) fn demons_bite(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    if !crate::auras::has_aura(world, caster, INSATIABLE_HUNGER) {
        return;
    }
    let low = script_effect_value(world, caster, INSATIABLE_HUNGER, 2) as i32;
    let high = script_effect_value(world, caster, INSATIABLE_HUNGER, 3) as i32;
    let extra = rand::Rng::gen_range(&mut rand::thread_rng(), low..=high);
    spell_cast::powers::change_power(world, caster, PowerType::Fury, extra as f32);
}

/// 162794 explicitly uses197125h (not its root's30 mirror) and193840s1.
/// One roll for the root's landed target, never one per weapon child.
pub(super) fn chaos_strike(world: &mut World, caster: Entity, _target: Entity, depth: u8) {
    let refund = world
        .resource::<SpellStore>()
        .get(CHAOS_REFUND_CHANCE)
        .expect("Chaos Strike requires197125");
    let base_chance = refund
        .aura_options
        .as_ref()
        .expect("197125 requires proc chance")
        .proc_chance as f32;
    let chance = crate::auras::spell_mods::modify(
        world,
        caster,
        refund,
        crate::auras::SpellModOp::ProcChance,
        base_chance,
    );
    if rand::random::<f32>() * 100.0 < chance {
        spell_cast::cast_triggered(world, caster, CHAOS_REFUND, Some(caster), depth + 1);
    }
}

/// 356995's own description names370898s1/d. Channel application, not every tick.
pub(super) fn disintegrate(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if targets::relation_matches(world, caster, target, targets::Relation::Enemy) {
        spell_cast::cast_triggered(world, caster, DISINTEGRATE_SLOW, Some(target), depth + 1);
    }
}

/// 116858 and51505 explicitly scale damage with the caster's crit chance.
pub(super) fn crit_scaled_damage(
    world: &World,
    caster: Entity,
    _target: Entity,
    amount: f32,
) -> f32 {
    let crit = world
        .get::<crate::stats::CharacterStats>(caster)
        .map_or(0.0, |stats| stats.crit_pct);
    amount * (1.0 + crit / 100.0)
}

fn damaging_periodic_count(world: &World, caster: Entity, target: Entity) -> usize {
    world.get::<ActiveAuras>(target).map_or(0, |auras| {
        auras
            .iter()
            .filter(|aura| aura.caster == caster)
            .flat_map(|aura| &aura.effects)
            .filter(|effect| {
                matches!(
                    effect.aura_type,
                    PERIODIC_DAMAGE | PERIODIC_HEALTH_LEECH | PERIODIC_DAMAGE_PERCENT
                )
            })
            .count()
    })
}

/// 324536: your damaging periodic effects erupt, with324540s1 per effect.
pub(super) fn rapture(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if damaging_periodic_count(world, caster, target) > 0 {
        spell_cast::cast_triggered(world, caster, RAPTURE_DAMAGE, Some(target), depth + 1);
    }
}

pub(super) fn rapture_damage(world: &World, caster: Entity, target: Entity, amount: f32) -> f32 {
    amount * damaging_periodic_count(world, caster, target) as f32
}

/// 197835s2 is explicitly combo points per target hit, not per cast.
pub(super) fn shuriken_storm(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    let mut points = script_effect_value(world, caster, SHURIKEN_STORM, 1);
    //121471 explicitly doubles combo-generating abilities. Its native masks
    //do not match197835's scripted effect1, unlike Backstab's native energize.
    if crate::auras::has_aura(world, caster, SHADOW_BLADES) {
        points *= 1.0 + script_effect_value(world, caster, SHADOW_BLADES, 3) / 100.0;
    }
    spell_cast::powers::change_power(world, caster, PowerType::ComboPoints, points);
}

/// 195292s3 generates Bone Shield charges. Normal aura refresh supplies its cap.
pub(super) fn deaths_caress(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    let charges = script_effect_value(world, caster, DEATHS_CARESS, 2) as u32;
    for _ in 0..charges {
        crate::auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: BONE_SHIELD,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
}

fn has_owned(world: &World, caster: Entity, target: Entity, id: u32) -> bool {
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.caster == caster && aura.spell_id == id)
    })
}

/// 207317 names212739 for each of your Virulent Plagues and77575s3 for extension.
pub(super) fn epidemic(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    let virulent = has_owned(world, caster, target, VIRULENT_PLAGUE);
    let milliseconds = script_effect_value(world, caster, OUTBREAK, 2) as u32;
    crate::auras::extend_owned_duration(
        world,
        caster,
        target,
        &[DREAD_PLAGUE, VIRULENT_PLAGUE],
        milliseconds,
    );
    if virulent {
        spell_cast::cast_triggered(world, caster, EPIDEMIC_DAMAGE, Some(target), depth + 1);
    }
}

/// The infected enemy receives212739s1; other nearby foes receive212739s2.
pub(super) fn epidemic_splash(
    _world: &World,
    _spell: &SpellInfo,
    explicit: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    units.retain(|&unit| Some(unit) != explicit);
}

pub(super) fn lava_burst_crit(world: &World, target: Entity) -> Option<f32> {
    crate::auras::has_aura(world, target, FLAME_SHOCK).then_some(100.0)
}
