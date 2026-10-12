//! TC a352b1fa SpellInfo.cpp600 and StatSystem.cpp549: mastery is raw points;
//! Attr8 MasteryAffectsPoints adds BonusCoefficient*points, then effect modifiers.
use super::ActiveAuras;
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
    if !affects_points(spell) || world.get::<Player>(caster).is_none() {
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

fn collect_updates(world: &World, unit: Entity) -> Vec<AmountUpdate> {
    let Some(auras) = world.get::<ActiveAuras>(unit) else {
        return Vec::new();
    };
    let Some(store) = world.get_resource::<SpellStore>() else {
        return Vec::new();
    };
    let now = super::aura_now(world);
    auras
        .iter()
        .filter(|aura| aura.caster == unit)
        .flat_map(|aura| {
            let spell = store
                .get(aura.spell_id)
                .filter(|spell| affects_points(spell));
            aura.effects.iter().filter_map(move |state| {
                let spell = spell?;
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
            })
        })
        .collect()
}

/// Recalculate amounts only; never restart native aura duration, stacks or tick phase.
pub(super) fn refresh_amounts(world: &mut World) {
    let units: Vec<_> = world
        .query_filtered::<Entity, (
            With<Player>,
            Or<(Changed<CharacterStats>, Changed<ActiveAuras>)>,
        )>()
        .iter(world)
        .collect();
    for unit in units {
        let updates = collect_updates(world, unit);
        if updates.is_empty() {
            continue;
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
        super::stat_mods::recompute(world, unit);
        crate::spell_cast::mark_dirty(world, unit);
    }
}
