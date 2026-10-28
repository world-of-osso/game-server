//! Native Assassination children whose dispatch is scripted in SimC76f8118.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    self, AuraInstance, RemoveReason,
    scripts::{AuraScript, AuraScripts},
};
use combat::spell_cast::{self, effects::EffectCtx};
use combat::spell_info::SpellStore;
use shared::components::{ActiveSpec, PowerType};

const RUPTURE: u32 = 1943;
const INTERNAL_BLEEDING: u32 = 381627;
const INTERNAL_PAYLOAD: u32 = 381628;
const KINGSBANE: u32 = 385627;
const KINGSBANE_RAMP: u32 = 394095;
const REGICIDE_TALENT: u32 = 1250325;
const REGICIDE_BUFF: u32 = 1250331;
const IMPLACABLE: u32 = 1265387;
const IMPLACABLE_DRIVER: u32 = 1265787;
const IMPLACABLE_HITS: &[u32] = &[1265794, 1265795];

#[derive(Component)]
struct ImplacableTarget(Entity);

pub(crate) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        KINGSBANE,
        AuraScript {
            on_remove: Some(finish_kingsbane_on_death),
            ..Default::default()
        },
    );
    scripts.0.insert(
        KINGSBANE_RAMP,
        AuraScript {
            on_natural_end: Some(grant_regicide),
            ..Default::default()
        },
    );
    scripts.0.insert(
        REGICIDE_BUFF,
        AuraScript {
            on_periodic: Some(decay_regicide),
            ..Default::default()
        },
    );
    scripts.0.insert(
        IMPLACABLE_DRIVER,
        AuraScript {
            on_periodic: Some(tick_implacable),
            on_remove: Some(clear_implacable),
            ..Default::default()
        },
    );
}

pub(super) fn needs_context(world: &World, caster: Entity, spell: u32) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 259)
        && match spell {
            RUPTURE => auras::has_aura(world, caster, INTERNAL_BLEEDING),
            KINGSBANE => auras::has_aura(world, caster, IMPLACABLE),
            _ => false,
        }
}

/// sc_rogue.cpp5257–5264/6200–6203: retain the actual Rupture's
/// effective CP for the native periodic child, without spending them again.
pub(super) fn on_hit(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    if !needs_context(world, ctx.caster, ctx.spell.id) {
        return;
    }
    if ctx.spell.id == KINGSBANE {
        // sc_rogue.cpp4891–4905/4954–4958: one native one-second driver
        // owns both children on every200ms tick at the actual impacted target.
        world
            .entity_mut(ctx.caster)
            .insert(ImplacableTarget(target));
        spell_cast::cast_triggered(
            world,
            ctx.caster,
            IMPLACABLE_DRIVER,
            Some(target),
            ctx.depth + 1,
        );
        return;
    }
    let combo = ctx
        .spell
        .cast_policy_context
        .as_ref()
        .unwrap()
        .resources
        .iter()
        .find(|(power, _)| *power == PowerType::ComboPoints)
        .expect("native Rupture snapshot retains paid/effective combo points")
        .1;
    let mut child = world
        .resource::<SpellStore>()
        .get(INTERNAL_PAYLOAD)
        .expect("selected Internal Bleeding requires its native child")
        .clone();
    child.cast_damage_multiplier = Some(combo.max(1.0));
    spell_cast::launch_triggered_spell(world, ctx.caster, &child, Some(target), ctx.depth + 1);
}

// Use the actual positive TargetInfo before expiry, not a deferred proc event.
// Each sourced lethal poison has one direct damage effect; absorbs grant no ramp.
pub(super) fn poison_hit(world: &mut World, ctx: &EffectCtx<'_>, target: Entity) {
    let positive = spell_cast::resolving_damage(world, ctx)
        .and_then(|damage| damage.iter().find(|(unit, _)| *unit == target))
        .is_some_and(|(_, damage)| *damage > 0.0);
    if positive {
        auras::rogue::after_poison_hit(world, ctx, target);
    }
}

// SimC76f8118 sc_rogue.cpp4967–4975/11158–11162: each five earned
// Kingsbane stacks grant one native haste stack, lost every native two seconds.
fn grant_regicide(world: &mut World, caster: Entity, ramp: &AuraInstance) {
    let Some(divisor) = world.get::<auras::ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|a| a.spell_id == REGICIDE_TALENT && a.caster == caster)
            .and_then(|a| a.effects.iter().find(|effect| effect.index == 0))
            .map(|effect| effect.amount as u8)
    }) else {
        return;
    };
    let stacks = ramp.stacks / divisor;
    if stacks == 0 {
        return;
    }
    let mut buff = world
        .resource::<SpellStore>()
        .get(REGICIDE_BUFF)
        .expect("selected Regicide requires its native buff")
        .clone();
    buff.cast_aura_stacks = Some(stacks);
    spell_cast::launch_triggered_spell(world, caster, &buff, Some(caster), 1);
}

fn finish_kingsbane_on_death(
    world: &mut World,
    _target: Entity,
    dot: &AuraInstance,
    reason: RemoveReason,
) {
    if reason != RemoveReason::Death {
        return;
    }
    let ramp = world
        .get::<auras::ActiveAuras>(dot.caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|a| a.spell_id == KINGSBANE_RAMP && a.caster == dot.caster)
                .cloned()
        });
    if let Some(ramp) = ramp {
        grant_regicide(world, dot.caster, &ramp);
        auras::remove_instance(world, dot.caster, ramp.instance_id, RemoveReason::Consumed);
    }
}

fn decay_regicide(world: &mut World, caster: Entity, buff: &AuraInstance, index: u8) {
    if index == 1 {
        auras::spend_stack(world, caster, buff.instance_id);
    }
}

fn tick_implacable(world: &mut World, caster: Entity, _aura: &AuraInstance, index: u8) {
    if index != 1 {
        return;
    }
    let Some(target) = world
        .get::<ImplacableTarget>(caster)
        .map(|snapshot| snapshot.0)
    else {
        return;
    };
    if !spell_cast::targets::is_alive(world, target) {
        return;
    }
    for &child in IMPLACABLE_HITS {
        spell_cast::cast_triggered(world, caster, child, Some(target), 1);
    }
}

fn clear_implacable(
    world: &mut World,
    caster: Entity,
    _aura: &AuraInstance,
    _reason: RemoveReason,
) {
    world.entity_mut(caster).remove::<ImplacableTarget>();
}
