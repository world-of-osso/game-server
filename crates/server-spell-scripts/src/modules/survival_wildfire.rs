//! Native Wildfire payloads and recipients, replacing the primary-only bridge.
use super::cast_relationships::has_talent;
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::script_registry::{HitHook, SpellScriptHook, TargetPlanHook};
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{self, PlannedEffect, TargetPlan, effects::EffectCtx, targets};
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EffectHit(root_hit),
    SpellScriptHook::TargetPlan(payload_plan),
    SpellScriptHook::ResolvedDamage(apply_periodic_payload),
];

const WILDFIRE_BOMB: u32 = 259495;
const EXPLOSION: u32 = 265157;
const ORDINARY_DOT: u32 = 269747;
const SHRAPNEL_TALENT: u32 = 1253172;
const SHRAPNEL_BLEED: u32 = 1253171;
// SimC76f8118 sc_hunter.cpp7064: its sourced approximation is a5yd circle
// around the actual primary. Neither native damage record carries a radius.
const EXPLOSION_RADIUS: f32 = 5.0;

fn root_hit(spell: u32, effect: u8) -> Option<HitHook> {
    (spell == WILDFIRE_BOMB && effect == 2).then_some(explode)
}

fn explode(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    // sc_hunter.cpp7134: one actual child explosion; native target planning
    // supplies its recipient list and the parent supplies cap/primary percentage.
    spell_cast::cast_triggered(world, caster, EXPLOSION, Some(target), depth + 1);
}

fn payload_plan(spell: u32) -> Option<(targets::Relation, TargetPlanHook)> {
    (spell == EXPLOSION).then_some((targets::Relation::Enemy, explosion_plan))
}

fn explosion_plan(
    world: &mut World,
    caster: Entity,
    _spell: &SpellInfo,
    explicit: Option<Entity>,
) -> TargetPlan {
    let primary = explicit.expect("native Wildfire explosion requires its actual primary");
    let recipients = select_recipients(world, caster, primary);
    let chain_multipliers = read_effectiveness(world, caster, primary, &recipients);
    vec![PlannedEffect {
        index: 0,
        units: recipients,
        chain_multipliers,
        dest: None,
        selected_units: Vec::new(),
        blocked_units: Vec::new(),
        prevent_hit_script: false,
    }]
}

fn select_recipients(world: &mut World, caster: Entity, primary: Entity) -> Vec<Entity> {
    let center = targets::position(world, primary).expect("native Wildfire primary position");
    let neighbors = targets::area_units(
        world,
        caster,
        center,
        EXPLOSION_RADIUS,
        targets::Relation::Enemy,
        0,
    );
    std::iter::once(primary)
        .chain(neighbors.into_iter().filter(|target| *target != primary))
        .collect()
}

fn read_effectiveness(
    world: &World,
    caster: Entity,
    primary: Entity,
    recipients: &[Entity],
) -> Vec<(Entity, f32)> {
    // SimC76f8118 sc_hunter.cpp7062–7068/7118–7120: the root's native
    // effect1 supplies reduced-AoE cap; effect2 supplies the primary-only bonus.
    let cap = script_effect_value(world, caster, WILDFIRE_BOMB, 1);
    assert!(cap > 0.0, "native Wildfire requires its authored AoE cap");
    let reduction = (cap / recipients.len() as f32).min(1.0).sqrt();
    let primary_bonus = 1.0 + script_effect_value(world, caster, WILDFIRE_BOMB, 2) / 100.0;
    recipients
        .iter()
        .map(|&target| {
            let effectiveness = if target == primary {
                primary_bonus
            } else {
                1.0
            };
            (target, reduction * effectiveness)
        })
        .collect()
}

fn apply_periodic_payload(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    _critical: bool,
    _damage: f32,
) {
    if ctx.spell.id != EXPLOSION {
        return;
    }
    // SimC76f8118 sc_hunter.cpp7030/7039–7043/7083–7089: selected
    // Shrapnel replaces the ordinary fire DoT for each actual explosion impact.
    let payload = if has_talent(world, ctx.caster, SHRAPNEL_TALENT) {
        SHRAPNEL_BLEED
    } else {
        ORDINARY_DOT
    };
    let Some(mut spell) = spell_cast::load_triggered_spell(world, payload, ctx.depth + 1) else {
        return;
    };
    spell.cast_damage_multiplier =
        Some(ctx.chain_multiplier * ctx.spell.cast_damage_multiplier.unwrap_or(1.0));
    if !spell_cast::launch_triggered_spell(world, ctx.caster, &spell, Some(target), ctx.depth + 1) {
        warn!(
            ?target,
            payload, "native Wildfire periodic payload refused its real impact recipient"
        );
    }
}
