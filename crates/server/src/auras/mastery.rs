//! TC a352b1fa SpellInfo.cpp600 and StatSystem.cpp549: mastery is raw points;
//! Attr8 MasteryAffectsPoints adds BonusCoefficient*points, then effect modifiers.
//! Approved followers share class-derived stats; ordinary NPCs do not gain mastery.
use super::ActiveAuras;
use crate::followers::Follower;
use crate::spell_info::{SpellEffect, SpellInfo, SpellStore};
use crate::stats::CharacterStats;
use bevy::prelude::*;
use shared::components::Player;

const MASTERY_AFFECTS_POINTS: u32 = 0x2000_0000;

fn affects_points(spell: &SpellInfo) -> bool {
    spell
        .misc
        .as_ref()
        .is_some_and(|misc| misc.attributes[8] & MASTERY_AFFECTS_POINTS != 0)
}

pub(crate) fn effect_bonus(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: &SpellEffect,
) -> f32 {
    let has_class_stats =
        world.get::<Player>(caster).is_some() || world.get::<Follower>(caster).is_some();
    if !affects_points(spell) || !has_class_stats {
        return 0.0;
    }
    world
        .get::<CharacterStats>(caster)
        .map_or(0.0, |stats| stats.mastery_points * effect.bonus_coefficient)
}

struct AmountUpdate {
    instance: u32,
    index: u8,
    amount: f32,
}

fn compute_update(
    world: &World,
    unit: Entity,
    aura: &super::AuraInstance,
    state: &super::AuraEffectState,
    spell: &SpellInfo,
    now: f64,
) -> Option<AmountUpdate> {
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.index == state.index)?;
    if effect.bonus_coefficient == 0.0 {
        return None;
    }
    let amount = super::build_effect(world, unit, spell, effect, now).amount;
    (amount != state.amount).then_some(AmountUpdate {
        instance: aura.instance_id,
        index: state.index,
        amount,
    })
}

fn collect_updates(world: &World, unit: Entity, now: f64) -> Vec<AmountUpdate> {
    let Some(auras) = world.get::<ActiveAuras>(unit) else {
        return Vec::new();
    };
    let Some(store) = world.get_resource::<SpellStore>() else {
        return Vec::new();
    };
    auras
        .iter()
        .filter(|aura| aura.caster == unit)
        .flat_map(|aura| {
            let spell = store
                .get(aura.spell_id)
                .filter(|spell| affects_points(spell));
            aura.effects
                .iter()
                .filter_map(move |state| compute_update(world, unit, aura, state, spell?, now))
        })
        .collect()
}

fn apply_updates(world: &mut World, unit: Entity, updates: Vec<AmountUpdate>) {
    if updates.is_empty() {
        return;
    }
    let mut auras = world
        .get_mut::<ActiveAuras>(unit)
        .expect("mastery aura unit");
    for update in updates {
        let aura = auras
            .auras
            .iter_mut()
            .find(|aura| aura.instance_id == update.instance)
            .expect("collected mastery aura instance");
        aura.effects
            .iter_mut()
            .find(|effect| effect.index == update.index)
            .expect("collected mastery aura effect")
            .amount = update.amount;
    }
    super::auras_changed(world, unit);
    crate::spell_cast::mark_dirty(world, unit);
}

/// Recalculate amounts only; never restart native aura duration, stacks or tick phase.
pub(super) fn refresh_amounts(world: &mut World) {
    let now = super::aura_now(world);
    let units: Vec<_> = world
        .query_filtered::<Entity, (
            Or<(With<Player>, With<Follower>)>,
            Or<(Changed<CharacterStats>, Changed<ActiveAuras>)>,
        )>()
        .iter(world)
        .collect();
    for unit in units {
        let updates = collect_updates(world, unit, now);
        apply_updates(world, unit, updates);
    }
}
