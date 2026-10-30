//! Nathreza's native chain keeps its primary and owned Corruption/Wither secondaries.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{
    PlannedEffect, TargetPlan,
    script_registry::{SpellScriptHook, TargetPlanHook},
    targets::{self, Relation},
};
use combat::spell_info::{SpellEffect, SpellInfo};

const SHADOW: u32 = 1262710;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetPlan(target_plan)];

fn target_plan(spell: u32) -> Option<(Relation, TargetPlanHook)> {
    (spell == SHADOW).then_some((Relation::Enemy, plan_chain))
}

fn has_owned_corruption(world: &World, caster: Entity, target: Entity) -> bool {
    let now = combat::auras::aura_now(world);
    world.get::<ActiveAuras>(target).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.caster == caster
                && matches!(aura.spell_id, 146739 | 445474)
                && aura.stacks > 0
                && aura.expires_at.is_none_or(|expiry| expiry > now)
        })
    })
}

// SimC76f8118 actions2532–2546: primary_target_or(corruption_or_wither_only()).
// Native selectors still own chain count/range; the source filter only removes
// ineligible secondary recipients, not the actual undotted Haunt primary.
fn select_chain(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
    primary: Entity,
) -> Vec<Entity> {
    let mut units = targets::select_targets(world, caster, Some(primary), spell, effect, 0, false)
        .expect("native Nathreza chain selectors are supported");
    units.retain(|&target| target == primary || has_owned_corruption(world, caster, target));
    units
}

fn plan_chain(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let primary = explicit.expect("Nathreza requires its actual Haunt recipient");
    spell
        .effects
        .iter()
        .enumerate()
        .map(|(index, effect)| {
            let units = select_chain(world, caster, spell, effect, primary);
            let chain_multipliers =
                std::iter::successors(Some(1.0), |factor| Some(factor * effect.chain_amplitude))
                    .zip(&units)
                    .map(|(factor, &target)| (target, factor))
                    .collect();
            PlannedEffect {
                index,
                units,
                dest: None,
                chain_multipliers,
                selected_units: Vec::new(),
                blocked_units: Vec::new(),
missed_units: Vec::new(),
                prevent_hit_script: false,
            }
        })
        .collect()
}
