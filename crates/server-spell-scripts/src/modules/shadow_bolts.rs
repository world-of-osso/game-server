//! Native Volley recipients and50ms child launch cadence.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::{HitHook, SpellScriptHook, TargetPlanHook};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{self, PlannedEffect, targets};
use combat::spell_info::{SpellInfo, SpellStore};
const VOID_VOLLEY: u32 = 1242173;
const VOLLEY_BOLT: u32 = 1242189;
const BOLT_INTERVAL_MS: u32 = 50;
const VOIDFORM: u32 = 228260;
pub const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EffectHit(effect_hit),
    SpellScriptHook::TargetPlan(target_plan),
];

fn effect_hit(spell: u32, index: u8) -> Option<HitHook> {
    match (spell, index) {
        (VOID_VOLLEY, 0) => Some(volley),
        (VOIDFORM, 0) => Some(initial_volley),
        _ => None,
    }
}

fn target_plan(spell: u32) -> Option<(targets::Relation, TargetPlanHook)> {
    (spell == VOID_VOLLEY).then_some((targets::Relation::Enemy, volley_plan))
}

///228260 explicitly names1242173. The initial volley is separate from the
///earned foreground uses granted by the downstream Shadow script.
fn initial_volley(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    spell_cast::cast_triggered(world, caster, VOID_VOLLEY, Some(target), depth + 1);
}

///1242173s1/s3 define ten bolts at the primary and one at each nearby enemy.
///Each uses1242189's native missile/hit path, with sourced50ms child scheduling.
fn volley_plan(
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
            selected_units: Vec::new(),
            blocked_units: Vec::new(),
            prevent_hit_script: false,
        },
        PlannedEffect {
            index: 1,
            units: vec![caster],
            dest: None,
            chain_multipliers: Vec::new(),
            selected_units: Vec::new(),
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
    let bolt = world
        .resource::<SpellStore>()
        .get(VOLLEY_BOLT)
        .expect("Volley requires1242189")
        .clone();
    for unit in read_recipients(world, caster, primary) {
        let index = if unit == primary { 0 } else { 2 };
        let count = script_effect_value(world, caster, VOID_VOLLEY, index) as usize;
        launch_bolts(world, caster, unit, &bolt, count, depth + 1);
    }
}

fn read_recipients(world: &mut World, caster: Entity, primary: Entity) -> Vec<Entity> {
    let root = world
        .resource::<SpellStore>()
        .get(VOID_VOLLEY)
        .expect("Volley root");
    let radius = targets::radius(&root.effects[0], 1);
    let center = targets::position(world, primary).expect("Volley hit target position");
    targets::area_units(world, caster, center, radius, targets::Relation::Enemy, 0)
}

fn launch_bolts(
    world: &mut World,
    caster: Entity,
    unit: Entity,
    bolt: &SpellInfo,
    count: usize,
    depth: u8,
) {
    let ctx = spell_cast::effects::EffectCtx {
        caster,
        spell: bolt,
        depth,
        dest: None,
        chain_multiplier: 1.0,
    };
    // SimC76f8118 sc_priest_shadow.cpp982–1014 explicitly repeats every50ms.
    for index in 1..=count {
        spell_cast::queued_triggers::schedule_full_trigger(
            world,
            &ctx,
            unit,
            bolt.id,
            index as u32 * BOLT_INTERVAL_MS,
        );
    }
}
