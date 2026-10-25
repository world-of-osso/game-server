//! Native Assassination children whose dispatch is scripted in SimC76f8118.
use crate::auras::{
    self, AuraInstance, RemoveReason,
    scripts::{AuraScript, AuraScripts},
};
use crate::spell_cast::{self, effects::EffectCtx};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use shared::components::{ActiveSpec, PowerType};

const RUPTURE: u32 = 1943;
const INTERNAL_BLEEDING: u32 = 381627;
const INTERNAL_PAYLOAD: u32 = 381628;
const KINGSBANE: u32 = 385627;
const IMPLACABLE: u32 = 1265387;
const IMPLACABLE_DRIVER: u32 = 1265787;
const IMPLACABLE_HITS: &[u32] = &[1265794, 1265795];

#[derive(Component)]
struct ImplacableTarget(Entity);

pub(crate) fn register(scripts: &mut AuraScripts) {
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
    let positive = world
        .get_resource::<spell_cast::ResolvingHits>()
        .and_then(|hits| hits.0.last())
        .filter(|frame| {
            frame.caster == ctx.caster && frame.spell_id == ctx.spell.id && frame.depth == ctx.depth
        })
        .and_then(|frame| frame.damage.iter().find(|(unit, _)| *unit == target))
        .is_some_and(|(_, damage)| *damage > 0.0);
    if positive {
        auras::rogue::after_poison_hit(world, ctx, target);
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
