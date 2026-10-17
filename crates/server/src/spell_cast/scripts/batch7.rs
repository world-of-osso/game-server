//! Bounded Mainline paths: TDB1210 registrations, pinned TC a352b1fa hooks,
//! and local 12.1.0.69933 descriptions/effects (see spell-dummy-scripts contract).
use bevy::prelude::*;
use shared::components::Health;

use super::script_effect_value;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::{self, PlannedEffect, TargetPlan, targets};
use crate::spell_info::{SpellInfo, SpellStore};

pub(super) const COLOSSUS_SMASH: u32 = 167105;
pub(super) const HEART_STRIKE: u32 = 206930;
pub(super) const RADIANCE: u32 = 194509;
pub(super) const RAGING_BLOW: u32 = 85288;
pub(super) const WHIRLWIND: u32 = 190411;
pub(super) const VERDANT_EMBRACE: u32 = 360995;
pub(super) const RAKE: u32 = 1822;
const COLOSSUS_WOUND: u32 = 208086;
const HEARTBREAKER: u32 = 221536;
const HEARTBREAKER_ENERGIZE: u32 = 210738;
const ATONEMENT: u32 = 194384;
const IMPROVED_RAGING_BLOW: u32 = 383854;
const ALWAYS_ANGRY: u32 = 280270;
const IMPROVED_WHIRLWIND: u32 = 12950;
const WHIRLWIND_ENERGIZE: u32 = 280715;
const WHIRLWIND_CLEAVE: u32 = 85739;
const PANACEA: u32 = 387761;
const PANACEA_HEAL: u32 = 387763;
const RAKE_RANK: u32 = 231052;
const POUNCING_STRIKES: u32 = 405834;
const RAKE_STUN: u32 = 163505;
const STEALTH_AURA: u32 = 16;
const RAW_RAGE_PER_POINT: f32 = 10.0;

/// TC spell_warr_colossus_smash;167105 explicitly names208086s1/d.
/// Apply only the base caster-owned vulnerability, not optional aura343 or
/// the child's unidentified self marker. Never amplify the initial strike.
pub(super) fn apply_colossus_wound(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    crate::auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target,
            spell_id: COLOSSUS_WOUND,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
}

/// TDB/TC spell_dk_heartbreaker and221536 explicitly name210738 per target.
pub(super) fn energize_heartbreaker(world: &mut World, caster: Entity, _target: Entity, depth: u8) {
    if crate::auras::has_aura(world, caster, HEARTBREAKER) {
        spell_cast::cast_triggered(
            world,
            caster,
            HEARTBREAKER_ENERGIZE,
            Some(caster),
            depth + 1,
        );
    }
}

///194509s3: primary plus four injured allies. TC radiance priority first
/// favors allies without THIS priest's Atonement. Keep native radius/order;
/// raid/pet priority and actual Atonement application are separate contracts.
pub(super) fn radiance_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let primary = explicit.expect("Radiance requires validated ally");
    let index = spell
        .effects
        .iter()
        .position(|e| e.index == 1)
        .expect("Radiance heal effect");
    let mut units = select_radiance_allies(world, caster, spell, primary, index);
    units.truncate(script_effect_value(world, caster, RADIANCE, 2) as usize);
    units.insert(0, primary);
    vec![PlannedEffect {
        index,
        units,
        dest: targets::effect_dest(
            world,
            caster,
            explicit,
            &spell.effects[index],
            spell.cast_destination,
        ),
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}

fn select_radiance_allies(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    primary: Entity,
    index: usize,
) -> Vec<Entity> {
    let mut units = targets::select_targets(
        world,
        caster,
        Some(primary),
        spell,
        &spell.effects[index],
        0,
        false,
    )
    .expect("Radiance catalog area selectors");
    units.retain(|&unit| {
        unit != primary
            && world
                .get::<Health>(unit)
                .is_some_and(|health| health.current < health.max)
    });
    // Stable sorting retains native nearest-first ordering within each priority.
    units.sort_by_key(|&unit| {
        world.get::<ActiveAuras>(unit).is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == ATONEMENT && aura.caster == caster)
        })
    });
    units
}

/// TC spell_warr_raging_blow_cooldown_reset; own tooltip gates its s1 chance
/// on383854. Restore ONE charge (not every charge or a fresh cooldown).
/// Wrath and Fury / Surge of Adrenaline variants remain unimplemented.
pub(super) fn roll_raging_blow_reset(
    world: &mut World,
    caster: Entity,
    _target: Entity,
    depth: u8,
) {
    if !crate::auras::has_aura(world, caster, IMPROVED_RAGING_BLOW) {
        return;
    }
    let chance = script_effect_value(world, caster, RAGING_BLOW, 0);
    if rand::random::<f32>() * 100.0 >= chance {
        return;
    }
    let category = world
        .resource::<SpellStore>()
        .get(RAGING_BLOW)
        .unwrap()
        .categories
        .as_ref()
        .unwrap()
        .charge_category;
    spell_cast::cast_triggered(world, caster, ALWAYS_ANGRY, Some(caster), depth + 1);
    spell_cast::restore_charges(world, caster, category, 1);
}

///190411s1 base Rage, s2 per target with12950, at most s3 targets.
/// Run before weapon children: even a killing sweep counts its selected units.
/// No synthetic Dummy0/no-op hook just to populate the census registry.
pub(super) fn energize_whirlwind(world: &mut World, caster: Entity, _target: Entity, depth: u8) {
    let root = world
        .resource::<SpellStore>()
        .get(WHIRLWIND)
        .unwrap()
        .clone();
    let count = targets::select_targets(world, caster, None, &root, &root.effects[0], 0, false)
        .expect("Whirlwind catalog count selectors")
        .len();
    let rank = crate::auras::has_aura(world, caster, IMPROVED_WHIRLWIND);
    let base = script_effect_value(world, caster, WHIRLWIND, 0);
    let extra = if rank {
        let cap = script_effect_value(world, caster, WHIRLWIND, 2) as usize;
        count.min(cap) as f32 * script_effect_value(world, caster, WHIRLWIND, 1)
    } else {
        0.0
    };
    cast_whirlwind_energize(world, caster, (base + extra) * RAW_RAGE_PER_POINT, depth);
    if rank && count > 0 {
        apply_whirlwind_cleave(world, caster);
    }
}

fn cast_whirlwind_energize(world: &mut World, caster: Entity, amount: f32, depth: u8) {
    let mut child = world
        .resource::<SpellStore>()
        .get(WHIRLWIND_ENERGIZE)
        .unwrap()
        .clone();
    child.effects[0].base_points = amount;
    let plan = vec![PlannedEffect {
        index: 0,
        units: vec![caster],
        dest: None,
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }];
    spell_cast::missiles::launch(world, caster, &child, plan, depth + 1);
}

/// TC ApplyWhirlwindCleaveAura supplies the modified catalog maximum stacks.
/// Application only; copied single-target attacks/stack consumption stay open.
fn apply_whirlwind_cleave(world: &mut World, caster: Entity) {
    let spell = world
        .resource::<SpellStore>()
        .get(WHIRLWIND_CLEAVE)
        .unwrap();
    let base = spell.aura_options.as_ref().unwrap().cumulative_aura as f32;
    let maximum = spell_cast::modify(
        world,
        caster,
        spell,
        spell_cast::SpellModOp::MaxAuraStacks,
        base,
    )
    .clamp(1.0, 255.0) as usize;
    for _ in 0..maximum {
        crate::auras::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: caster,
                spell_id: WHIRLWIND_CLEAVE,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
}

/// TDB/TC spell_evo_panacea;387761 explicitly names387763 on cast.
/// Verdant Embrace's flight/primary heal are NOT certified by this self heal.
pub(super) fn heal_panacea(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if crate::auras::has_aura(world, caster, PANACEA) {
        spell_cast::cast_triggered(world, caster, PANACEA_HEAL, Some(caster), 1);
    }
}

/// TC spell_dru_rake supplies stealth/stun mechanics. Current Mainline tooltip
/// additionally gates them on231052 or405834. This hook certifies initial
/// damage only, not child155722's full periodic snapshot contract.
fn has_rake_stealth_bonus(world: &World, caster: Entity) -> bool {
    let rank = crate::auras::has_aura(world, caster, RAKE_RANK)
        || crate::auras::has_aura(world, caster, POUNCING_STRIKES);
    let stealth = world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.effects
                .iter()
                .any(|effect| effect.aura_type == STEALTH_AURA)
        })
    });
    rank && stealth
}

pub(super) fn rake_initial_damage(
    world: &World,
    caster: Entity,
    _target: Entity,
    amount: f32,
) -> f32 {
    if has_rake_stealth_bonus(world, caster) {
        amount * (1.0 + script_effect_value(world, caster, RAKE, 3) / 100.0)
    } else {
        amount
    }
}

pub(super) fn stun_rake(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if has_rake_stealth_bonus(world, caster) {
        spell_cast::cast_triggered(world, caster, RAKE_STUN, Some(target), depth + 1);
    }
}
