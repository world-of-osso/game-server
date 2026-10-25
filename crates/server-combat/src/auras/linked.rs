//! TC a352b1fa HandleAuraLinked: preserve authored BP0 at zero; override at nonzero.
use bevy::prelude::*;

use super::{AuraEffectState, AuraInstance, handlers, lookup_spell};
use crate::spell_cast::{self, targets};

pub(super) fn is_linked(aura_type: u32) -> bool {
    matches!(aura_type, handlers::LINKED | handlers::LINKED_2)
}

fn linked_effects(aura: &AuraInstance) -> impl Iterator<Item = &AuraEffectState> {
    aura.effects
        .iter()
        .filter(|effect| is_linked(effect.aura_type))
}

fn find_child_caster(
    world: &World,
    target: Entity,
    aura: &AuraInstance,
    child_id: u32,
) -> Option<Entity> {
    let Some(child) = lookup_spell(world, child_id) else {
        warn!("Linked aura {}: missing child {child_id}", aura.spell_id);
        return None;
    };
    let explicit_unit = targets::explicit_requirement(child.effects.iter()).is_some();
    let channel_unit = lookup_spell(world, aura.spell_id).is_some_and(|parent| {
        spell_cast::is_channeled(&parent)
            && child
                .effects
                .iter()
                .any(targets::provides_non_caster_unit_target)
    });
    Some(if explicit_unit || channel_unit {
        aura.caster
    } else {
        target
    })
}

pub(super) fn on_applied(world: &mut World, target: Entity, aura: &AuraInstance) {
    for effect in linked_effects(aura) {
        let Some(caster) = find_child_caster(world, target, aura, effect.trigger_spell) else {
            continue;
        };
        let depth = spell_cast::next_trigger_depth(world);
        spell_cast::cast_triggered_with_overrides(
            world,
            caster,
            effect.trigger_spell,
            Some(target),
            depth,
            spell_cast::TriggeredSpellOverrides {
                effect0_amount: (effect.amount != 0.0).then_some(effect.amount),
                ..Default::default()
            },
        );
    }
}

pub(super) fn on_reapplied(world: &mut World, target: Entity, aura: &AuraInstance) {
    for effect in linked_effects(aura) {
        let Some(caster) = find_child_caster(world, target, aura, effect.trigger_spell) else {
            continue;
        };
        let changed =
            synchronize_child_stacks(world, target, caster, effect.trigger_spell, aura.stacks);
        if changed {
            super::auras_changed(world, target);
        }
    }
}

fn synchronize_child_stacks(
    world: &mut World,
    target: Entity,
    caster: Entity,
    child_id: u32,
    parent_stacks: u8,
) -> bool {
    let Some(mut active) = world.get_mut::<super::ActiveAuras>(target) else {
        return false;
    };
    let Some(child) = active
        .auras
        .iter_mut()
        .find(|child| child.spell_id == child_id && child.caster == caster)
    else {
        return false;
    };
    let stacks = parent_stacks.min(child.max_stacks.max(1));
    if child.stacks == stacks {
        return false;
    }
    child.stacks = stacks;
    true
}

pub(super) fn on_removed(world: &mut World, target: Entity, aura: &AuraInstance) {
    for effect in linked_effects(aura) {
        let Some(caster) = find_child_caster(world, target, aura, effect.trigger_spell) else {
            continue;
        };
        super::remove_auras(world, target, super::RemoveReason::Cancelled, |child| {
            child.spell_id == effect.trigger_spell && child.caster == caster
        });
    }
}
