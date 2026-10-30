//! SimC76f8118 sc_priest.cpp1557–1595/3564–3571: actual Rift end splits Collapse.
use super::combat;
use bevy::prelude::*;
use combat::auras;
use combat::spell_cast::script_registry::{SpellScriptHook, TargetPlanHook};
use combat::spell_cast::{self, PlannedEffect, TargetPlan, targets};
use combat::spell_info::SpellInfo;

const TALENT: u32 = 448403;
const DAMAGE: u32 = 448405;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::TargetPlan(target_plan),
];

fn declared_reach(
    world: &World,
    _caster: Entity,
    spell: &SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    if !matches!(spell.id, TALENT | DAMAGE | 263165) {
        return Default::default();
    }
    let damage = world
        .resource::<combat::spell_info::SpellStore>()
        .get(DAMAGE)
        .expect("native Collapse damage");
    let radius = damage
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Collapse effect")
        .radius[1]
        .expect("native Collapse radius")
        .radius_max;
    combat::spell_cast::script_registry::ScriptReach::target(radius)
}

pub(super) fn growth_multiplier(world: &World, caster: Entity, growth: u8) -> f32 {
    if growth == 0 {
        return 1.0;
    }
    let percent = spell_cast::scripts::script_effect_value(world, caster, TALENT, 2);
    1.0 + f32::from(growth) * percent / 100.0
}

pub(super) fn burst(world: &mut World, caster: Entity, primary: Entity, growth: u8) {
    if !auras::has_aura(world, caster, TALENT) {
        return;
    }
    let Some(mut damage) = spell_cast::load_triggered_spell(world, DAMAGE, 1) else {
        return;
    };
    damage.cast_primary_target = Some(primary);
    let plan = match spell_cast::plan_triggered_spell(world, caster, &damage, Some(primary), 1) {
        Ok(plan) => plan,
        Err(reason) => {
            warn!(?reason, "Native Collapse source-primary plan failed");
            return;
        }
    };
    let count = spell_cast::selected_units(&plan).len();
    if count == 0 {
        return;
    }
    // split_aoe_damage divides the unchanged native payload among actual recipients.
    damage.cast_damage_multiplier = Some(growth_multiplier(world, caster, growth) / count as f32);
    spell_cast::launch_planned_spell(world, caster, &damage, plan, 1);
}

fn target_plan(spell: u32) -> Option<(targets::Relation, TargetPlanHook)> {
    (spell == DAMAGE).then_some((targets::Relation::Enemy, plan_burst))
}

fn plan_burst(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    primary: Option<Entity>,
) -> TargetPlan {
    let primary = primary.expect("Collapse retains the actual Rift primary");
    let Some(center) = targets::position(world, primary) else {
        return Vec::new();
    };
    let radius = spell
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native Collapse damage effect")
        .radius[1]
        .expect("native Collapse enemy-area radius")
        .radius_max;
    let units = targets::area_units(world, caster, center, radius, targets::Relation::Enemy, 0);
    vec![PlannedEffect {
        index: 0,
        units,
        dest: None,
        chain_multipliers: Vec::new(),
        selected_units: Vec::new(),
        blocked_units: Vec::new(),
        missed_units: Vec::new(),
        prevent_hit_script: false,
    }]
}
