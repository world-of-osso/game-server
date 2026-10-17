//! Local Mainline12.1.0.69933 descriptions and numeric fields, not guessed actors.
use bevy::prelude::*;
use shared::components::{Level, PowerType};

use super::{batch3, batch9, script_effect_value};
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use crate::spell_cast::{self, PlannedEffect, targets};
use crate::spell_info::{SpellInfo, SpellStore};

pub(super) const SINISTER_STRIKE: u32 = 193315;
pub(super) const VOID_VOLLEY: u32 = 1242173;
pub(super) const VOIDFORM: u32 = 228260;
pub(super) const HAMMER_RIGHTEOUS: u32 = 53595;
pub(super) const BLADE_JUSTICE: u32 = 184575;
pub(super) const THRASH: u32 = 106830;
const VOLLEY_BOLT: u32 = 1242189;
const HAMMER_WAVE: u32 = 88263;
const BLADE_TALENT: u32 = 403826;
const THRASH_BLEED: u32 = 405233;
const SHRED_STEALTH_RANK: u32 = 343232;
const POUNCING_STRIKES: u32 = 405834;
const STEALTH_AURA: u32 = 16;

/// SpellXDescriptionVariables193315 ->338: exactly the authored level curve.
pub(super) fn sinister_damage(world: &World, caster: Entity, _target: Entity, amount: f32) -> f32 {
    let level = world
        .get::<Level>(caster)
        .expect("Sinister Strike caster Level")
        .0;
    amount * (((f32::from(level) - 10.0).clamp(0.0, 10.0) * 8.0 + 195.0) / 275.0)
}

/// Native E64 already names404358, but the description gates it on403826.
pub(in crate::spell_cast) fn effect_enabled(
    world: &World,
    caster: Entity,
    spell: u32,
    index: u8,
) -> bool {
    spell != BLADE_JUSTICE || index != 2 || crate::auras::has_aura(world, caster, BLADE_TALENT)
}

///228260 explicitly names1242173. The initial volley is separate from the
///unimplemented replacement/charge contract during Voidform.
pub(super) fn initial_volley(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    spell_cast::cast_triggered(world, caster, VOID_VOLLEY, Some(target), depth + 1);
}

///1242173s1/s3 define ten bolts at the primary and one at each nearby enemy.
///Each uses1242189's native missile/hit path. No un-authored inter-bolt delay.
pub(super) fn volley_plan(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> spell_cast::TargetPlan {
    let primary = explicit.expect("Volley validated primary target");
    let dest = targets::effect_dest(
        world,
        caster,
        explicit,
        &spell.effects[0],
        spell.cast_destination,
    );
    vec![
        PlannedEffect {
            index: 0,
            units: vec![primary],
            dest,
            chain_multipliers: Vec::new(),
            blocked_units: Vec::new(),
            prevent_hit_script: false,
        },
        PlannedEffect {
            index: 1,
            units: vec![caster],
            dest: None,
            chain_multipliers: Vec::new(),
            blocked_units: Vec::new(),
            prevent_hit_script: false,
        },
    ]
}

pub(super) fn volley(world: &mut World, caster: Entity, primary: Entity, depth: u8) {
    if depth >= spell_cast::effects::MAX_TRIGGER_DEPTH {
        warn!("Void Volley trigger depth exhausted");
        return;
    }
    let root = world
        .resource::<SpellStore>()
        .get(VOID_VOLLEY)
        .expect("Volley root")
        .clone();
    let center = targets::position(world, primary).expect("Volley hit target position");
    let bolt = world
        .resource::<SpellStore>()
        .get(VOLLEY_BOLT)
        .expect("Volley requires1242189")
        .clone();
    let units = targets::area_units(
        world,
        caster,
        center,
        targets::radius(&root.effects[0], 1),
        targets::Relation::Enemy,
        0,
    );
    for unit in units {
        let count = script_effect_value(
            world,
            caster,
            VOID_VOLLEY,
            if unit == primary { 0 } else { 2 },
        ) as usize;
        launch_bolts(world, caster, unit, &bolt, count, depth + 1);
    }
}

fn launch_bolts(
    world: &mut World,
    caster: Entity,
    unit: Entity,
    bolt: &SpellInfo,
    count: usize,
    depth: u8,
) {
    for _ in 0..count {
        let mut plan = vec![PlannedEffect {
            index: 0,
            units: vec![unit],
            dest: None,
            chain_multipliers: Vec::new(),
            blocked_units: Vec::new(),
            prevent_hit_script: false,
        }];
        spell_cast::hit::remove_defended_targets(world, caster, bolt, &mut plan);
        spell_cast::missiles::launch(world, caster, bolt, plan, depth);
    }
}

///53595 names88263 for all OTHER targets, only while standing in own26573.
pub(super) fn hammer_wave(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if depth >= spell_cast::effects::MAX_TRIGGER_DEPTH
        || !batch9::inside_owned_consecration(world, caster)
    {
        return;
    }
    let wave = world
        .resource::<SpellStore>()
        .get(HAMMER_WAVE)
        .expect("Hammer requires88263")
        .clone();
    let center = targets::position(world, target).expect("Hammer hit target position");
    let mut units = targets::area_units(
        world,
        caster,
        center,
        targets::radius(&wave.effects[0], 1),
        targets::Relation::Enemy,
        0,
    );
    units.retain(|&unit| unit != target);
    let mut plan = vec![PlannedEffect {
        index: 0,
        units,
        dest: Some(targets::Dest {
            position: center,
            orientation: 0.0,
        }),
        chain_multipliers: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }];
    spell_cast::hit::remove_defended_targets(world, caster, &wave, &mut plan);
    spell_cast::missiles::launch(world, caster, &wave, plan, depth + 1);
}

pub(super) fn shred_stealth_bonus(world: &World, caster: Entity) -> bool {
    let rank = crate::auras::has_aura(world, caster, SHRED_STEALTH_RANK)
        || crate::auras::has_aura(world, caster, POUNCING_STRIKES);
    let stealth = world
        .get::<ActiveAuras>(caster)
        .is_some_and(|auras| auras.has_type(STEALTH_AURA));
    rank && stealth
}

///5221s3 is the stealth damage bonus; keep the existing bleeding-rank bonus.
pub(super) fn shred_damage(world: &World, caster: Entity, target: Entity, amount: f32) -> f32 {
    let amount = batch3::shred_damage(world, caster, target, amount);
    if shred_stealth_bonus(world, caster) {
        amount * (1.0 + script_effect_value(world, caster, batch3::SHRED, 2) / 100.0)
    } else {
        amount
    }
}

///5221's conditional tooltip explicitly names343232s1 for the extra point.
pub(super) fn shred_points(world: &mut World, caster: Entity, _target: Entity, _depth: u8) {
    if shred_stealth_bonus(world, caster) {
        let points = script_effect_value(world, caster, SHRED_STEALTH_RANK, 0);
        spell_cast::powers::change_power(world, caster, PowerType::ComboPoints, points);
    }
}

///106830 explicitly names405233o1/d: only the named bleed, not other variants.
pub(super) fn thrash_bleed(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    crate::auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target,
            spell_id: THRASH_BLEED,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
}
