//! Assassination Deathstalker relationships from pinned SimC76f8118.
use bevy::prelude::*;
use shared::components::{ActiveSpec, PowerType};

use crate::auras::{self, ActiveAuras, RemoveReason, SpellModOp};
use crate::spell_cast::{self, effects::EffectCtx};
use crate::spell_info::{ChargedSpellModifier, SpellInfo, SpellStore};

const MARK_TALENT: u32 = 457052;
const MARK: u32 = 457129;
const MARK_DAMAGE: u32 = 457157;
const DARKEST_TALENT: u32 = 457058;
const DARKEST: u32 = 457280;
const GARROTE: u32 = 703;
const ENVENOM: u32 = 32645;

pub(super) fn needs_context(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 259)
        && auras::has_aura(world, caster, MARK_TALENT)
        && (spell.id == GARROTE
            || spell
                .power_costs
                .iter()
                .any(|cost| cost.power_type == PowerType::ComboPoints as i32))
}

fn combo_points(spell: &SpellInfo) -> f32 {
    spell
        .cast_policy_context
        .as_ref()
        .and_then(|context| {
            context
                .resources
                .iter()
                .find(|(power, _)| *power == PowerType::ComboPoints)
        })
        .map_or(0.0, |(_, amount)| *amount)
}

fn native_amount(world: &World, caster: Entity, spell: u32, index: u8) -> f32 {
    world
        .get::<ActiveAuras>(caster)
        .expect("selected Deathstalker caster retains its auras")
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == caster)
        .expect("selected Deathstalker source aura")
        .effects
        .iter()
        .find(|effect| effect.index == index)
        .expect("selected Deathstalker aura retains its native effect")
        .amount
}

fn empowered(world: &World, caster: Entity, spell: &SpellInfo) -> bool {
    let Some((_, maximum)) = spell_cast::powers::power_value(world, caster, PowerType::ComboPoints)
    else {
        return false;
    };
    spell.id == ENVENOM && combo_points(spell) >= maximum && auras::has_aura(world, caster, DARKEST)
}

/// sc_rogue.cpp2435–2440/2569–2574: only the maximum-CP Envenom
/// snapshots Darkest Night's native damage bonus and guaranteed critical chance.
pub(in crate::spell_cast) fn prepare(world: &World, caster: Entity, spell: &mut SpellInfo) {
    if !needs_context(world, caster, spell) || !empowered(world, caster, spell) {
        return;
    }
    let damage = native_amount(world, caster, DARKEST, 1);
    spell.cast_damage_multiplier =
        Some(spell.cast_damage_multiplier.unwrap_or(1.0) * (1.0 + damage / 100.0));
    let critical_chance = 100.0 + native_amount(world, caster, DARKEST, 3);
    spell
        .cast_charge_mods
        .get_or_insert_default()
        .push(ChargedSpellModifier {
            aura_spell_id: DARKEST,
            op: SpellModOp::CritChance as i32,
            percent: false,
            amount: critical_chance,
        });
}

/// sc_rogue.cpp4599/8732–8817: real Garrote establishes the mark;
/// eligible finishers spend it, or consume Darkest Night to start a new cycle.
pub(super) fn on_hit(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if !needs_context(world, ctx.caster, ctx.spell) {
        return;
    }
    if ctx.spell.id == GARROTE {
        let stacks = native_amount(world, ctx.caster, MARK_TALENT, 0) as u8;
        apply_mark(world, ctx, target, stacks);
        return;
    }
    if auras::has_aura(world, ctx.caster, DARKEST) {
        if empowered(world, ctx.caster, ctx.spell) {
            let stacks = native_amount(world, ctx.caster, DARKEST, 2) as u8;
            apply_mark(world, ctx, target, stacks);
            auras::remove_auras(world, ctx.caster, RemoveReason::Consumed, |aura| {
                aura.spell_id == DARKEST && aura.caster == ctx.caster
            });
        }
        return;
    }
    let minimum = native_amount(world, ctx.caster, MARK_TALENT, 1);
    if combo_points(ctx.spell) < minimum {
        return;
    }
    spend_mark(world, ctx, target);
}

fn owned_mark(world: &World, caster: Entity, target: Entity) -> Option<u32> {
    world
        .get::<ActiveAuras>(target)?
        .iter()
        .find(|aura| aura.spell_id == MARK && aura.caster == caster)
        .map(|aura| aura.instance_id)
}

fn apply_mark(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, stacks: u8) {
    if owned_mark(world, ctx.caster, target).is_some() {
        return;
    }
    let previous: Vec<_> = world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .filter_map(|(unit, auras)| {
            auras
                .iter()
                .find(|aura| aura.spell_id == MARK && aura.caster == ctx.caster)
                .map(|aura| (unit, aura.instance_id))
        })
        .collect();
    for (unit, instance) in previous {
        auras::remove_instance(world, unit, instance, RemoveReason::Consumed);
    }
    let mut mark = world
        .resource::<SpellStore>()
        .get(MARK)
        .expect("selected Deathstalker requires native mark")
        .clone();
    mark.cast_aura_stacks = Some(stacks);
    spell_cast::launch_triggered_spell(world, ctx.caster, &mark, Some(target), ctx.depth + 1);
}

fn spend_mark(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let Some(instance) = owned_mark(world, ctx.caster, target) else {
        return;
    };
    auras::spend_stack(world, target, instance);
    spell_cast::cast_triggered(world, ctx.caster, MARK_DAMAGE, Some(target), ctx.depth + 1);
    if owned_mark(world, ctx.caster, target).is_none()
        && auras::has_aura(world, ctx.caster, DARKEST_TALENT)
    {
        // sc_rogue.cpp8772–8778: delay the native buff and its Energy effect
        // until after the consuming finisher's impacts have completed.
        spell_cast::queued_triggers::schedule_full_trigger(world, ctx, ctx.caster, DARKEST, 1);
    }
}
