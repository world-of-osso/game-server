//! Chaotic Energies: native mastery values, actual rolls and pure expected values.
use super::combat;
use bevy::prelude::*;
use combat::random as rand;
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

const MASTERY: u32 = 77220;
const CHAOS_INCARNATE: u32 = 387275;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::ActualDamageMultiplier(roll_multiplier),
    SpellScriptHook::ExpectedDamageMultiplier(expected_multiplier),
];

// SimC76f8118 sc_warlock_actions.cpp1068/1191/1215/3858/3933/4042/
//4074/4288–4290/4443/4532–4533/4656/4728/4776–4778/4862/5108/
//5149/5275/5351 explicitly mark these actions. Infernal Bolt, pets, items
// and Malevolence damage do not acquire this multiplier by school or class.
const AFFECTED: &[(u32, bool)] = &[
    (348, false),
    (6353, false),
    (17877, true),
    (17962, false),
    (29722, false),
    (42223, true),
    (116858, true),
    (152108, false),
    (157736, false),
    (196448, false),
    (434635, false),
    (434636, false),
    (445468, false),
    (445474, false),
    (445736, false),
    (1244890, false),
    (1265884, false),
];

fn read_multiplier_range(world: &World, caster: Entity, spell: &SpellInfo) -> Option<(f32, f32)> {
    let destruction = world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 267);
    if !destruction || !combat::auras::has_aura(world, caster, MASTERY) {
        return None;
    }
    let &(_, incarnate_action) = AFFECTED.iter().find(|(id, _)| *id == spell.id)?;
    let incarnate = incarnate_action
        && world
            .get::<combat::class_progression::TraitLoadout>(caster)
            .is_some_and(|traits| {
                traits
                    .active_talent_spells()
                    .any(|talent| talent.spell_id == CHAOS_INCARNATE)
            });
    // actions.cpp480–489: the default lower fraction is0.5; only marked
    // actions use selected Chaos Incarnate's native effect0 lower fraction.
    let minimum_fraction = if incarnate {
        script_effect_value(world, caster, CHAOS_INCARNATE, 0) / 100.0
    } else {
        0.5
    };
    let mastery = world
        .get::<combat::stats::CharacterStats>(caster)
        .expect("native Chaotic Energies actor requires CharacterStats")
        .mastery_pct
        / 100.0;
    Some((1.0 + minimum_fraction * mastery, 1.0 + mastery))
}

fn roll_multiplier(world: &mut World, ctx: &EffectCtx<'_>, _target: Entity) -> f32 {
    let Some((minimum, maximum)) = read_multiplier_range(world, ctx.caster, ctx.spell) else {
        return 1.0;
    };
    if minimum == maximum {
        return minimum;
    }
    // Actual resolution only: a separate sample for each native direct hit or
    // periodic update, never during APL readiness or expected-damage queries.
    rand::Rng::gen_range(&mut rand::thread_rng(), minimum..=maximum)
}

fn expected_multiplier(world: &World, caster: Entity, _target: Entity, spell: &SpellInfo) -> f32 {
    read_multiplier_range(world, caster, spell)
        .map_or(1.0, |(minimum, maximum)| (minimum + maximum) / 2.0)
}
