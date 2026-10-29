//! Preservation Life-Giver's Flame, 70448 talent 371426.
use bevy::prelude::*;
use shared::components::Health;

use crate::spell_cast::effects::{self, EffectCtx};
use crate::spell_cast::targets::{self, Relation};

const TALENT: u32 = 371426;
const HEAL: u32 = 371441;
const DAMAGE: &[u32] = &[357209, 353759];
const AGGREGATION_MS: u32 = 100;

#[derive(Component)]
struct PendingDamage(f32);

pub(crate) fn record_damage(
    world: &mut World,
    ctx: &EffectCtx<'_>,
    target: Entity,
    critical: bool,
    amount: f32,
) {
    let caster = ctx.caster;
    if !DAMAGE.contains(&ctx.spell.id) || amount <= 0.0 || !super::has_aura(world, caster, TALENT) {
        return;
    }
    let noncritical = if critical {
        let multiplier =
            effects::critical_damage_amount(world, caster, caster, target, ctx.spell, 1.0, false);
        amount / multiplier
    } else {
        amount
    };
    if let Some(mut pending) = world.get_mut::<PendingDamage>(caster) {
        pending.0 += noncritical;
    } else {
        world.entity_mut(caster).insert(PendingDamage(noncritical));
        crate::spell_cast::queued_triggers::schedule_callback(
            world,
            ctx,
            caster,
            AGGREGATION_MS,
            deliver_healing,
        );
    }
}

fn deliver_healing(world: &mut World, caster: Entity, _target: Entity, depth: u8) {
    let Some(pending) = world.entity_mut(caster).take::<PendingDamage>() else {
        return;
    };
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(HEAL)
        .expect("Life-Giver's Flame requires spell 371441")
        .clone();
    let Some(center) = targets::position(world, caster) else {
        return;
    };
    let radius = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.range)
        .expect("Life-Giver's Flame heal range")
        .max_friendly;
    let mut allies = targets::area_units(world, caster, center, radius, Relation::Ally, 0);
    allies.retain(|&unit| {
        world
            .get::<Health>(unit)
            .is_some_and(|health| health.current > 0.0 && health.current < health.max)
    });
    let cap =
        crate::spell_cast::scripts::script_effect_value(world, caster, TALENT, 2).max(0.0) as usize;
    allies.truncate(cap);
    if allies.is_empty() {
        return;
    }
    let share = crate::spell_cast::scripts::script_effect_value(world, caster, TALENT, 0) / 100.0;
    let amount = pending.0 * share / allies.len() as f32;
    let ctx = EffectCtx {
        caster,
        spell: &spell,
        depth,
        dest: None,
        chain_multiplier: 1.0,
    };
    for ally in allies {
        effects::apply_fixed_heal(world, &ctx, ally, amount);
    }
}
