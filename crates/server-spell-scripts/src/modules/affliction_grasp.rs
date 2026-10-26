//! Malefic Grasp copies base native DoT ticks into their own native payloads.
use super::combat;
use bevy::prelude::*;
use combat::auras::ActiveAuras;
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};

const GRASP: u32 = 1261153;
const WITHER_TALENT: u32 = 445465;
const AGONY: u32 = 980;
const AGONY_CHILD: u32 = 1261166;
const PERIODIC_DAMAGE: u32 = 3;
// SimC76f8118 sc_warlock_actions.cpp2036–2044: pinned bugs=1 compensates
// for the Agony child's missing direct-damage whitelists, not its coefficient.
const AGONY_CHILD_BUG_MULTIPLIER: f32 = 1.95;
const DOT_CHILDREN: &[(u32, u32, bool)] = &[
    (AGONY, AGONY_CHILD, true),
    (1259790, 1261176, true),
    (146739, 1261158, true),
    (445474, 1279686, false),
];

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::PeriodicPrimaryDamage(copy_owned_ticks),
    SpellScriptHook::EnemyContextHit(child_hit),
    SpellScriptHook::ConsumedEffectValues(&[(GRASP, 1)]),
];

// SimC76f8118 sc_warlock_actions.cpp136–158/2210–2214: inherit the
// unmodified base tick, with UA/Agony stacks but no Wither counter scaling.
// The real child owns its independent damage modifiers and critical roll.
fn copy_owned_ticks(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: f32, _: bool) {
    if ctx.spell.id != GRASP {
        return;
    }
    let fraction = spell_cast::scripts::script_effect_value(world, ctx.caster, GRASP, 1) / 100.0;
    let wither = combat::auras::has_aura(world, ctx.caster, WITHER_TALENT);
    for &(dot, child, use_stacks) in DOT_CHILDREN {
        if !selected_corruption(dot, wither) {
            continue;
        }
        let Some(base) = owned_base_tick(world, ctx.caster, target, dot, use_stacks) else {
            continue;
        };
        let amount = base * fraction;
        let amount = if child == AGONY_CHILD {
            amount * AGONY_CHILD_BUG_MULTIPLIER
        } else {
            amount
        };
        spell_cast::cast_triggered_with_effect0_amount(
            world,
            ctx.caster,
            child,
            Some(target),
            ctx.depth + 1,
            amount,
        );
    }
}

// Source actions2117–2131: the selected hero owns exactly one of these
// replacement children. The actual Corruption DoT is146739, not direct172.
fn selected_corruption(dot: u32, wither: bool) -> bool {
    match dot {
        146739 => !wither,
        445474 => wither,
        _ => true,
    }
}

fn owned_base_tick(
    world: &World,
    caster: Entity,
    target: Entity,
    dot: u32,
    use_stacks: bool,
) -> Option<f32> {
    let aura = world
        .get::<ActiveAuras>(target)?
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == dot && aura.stacks > 0)?;
    let effect = aura
        .effects
        .iter()
        .find(|effect| effect.aura_type == PERIODIC_DAMAGE)?;
    let stacks = if use_stacks {
        aura.stack_multiplier()
    } else {
        1.0
    };
    Some(effect.amount * stacks)
}

fn child_hit(spell: u32) -> Option<spell_cast::script_registry::EnemyContextHitHook> {
    (spell == AGONY_CHILD).then_some(advance_agony)
}

// SimC76f8118 sc_warlock_actions.cpp2052–2060: a landed Agony extra
// tick advances its actual owned DoT counter, but cannot fund shards.
fn advance_agony(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let instance = world.get::<ActiveAuras>(target).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.caster == ctx.caster && aura.spell_id == AGONY)
            .map(|aura| aura.instance_id)
    });
    if let Some(instance) = instance {
        combat::auras::advance_agony_stack(world, target, instance);
    }
}
