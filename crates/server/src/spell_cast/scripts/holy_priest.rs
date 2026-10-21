//! Native70448 Holy scripts. Numeric values remain in the local spell catalog.
use bevy::prelude::*;

use crate::auras::procs::AuraEvent;
use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::auras::{ActiveAuras, AuraInstance, has_aura};
use crate::spell_info::{SpellInfo, SpellStore};

pub(crate) const APOTHEOSIS: u32 = 200183;
const SURGE_TALENT: u32 = 109186;
const SURGE_BUFF: u32 = 114255;
const SMITE: u32 = 585;
const MIRACLE_WORKER: u32 = 235587;
const HOLY_WORDS: &[u32] = &[2050, 34861, 88625];

pub(crate) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        SURGE_TALENT,
        AuraScript {
            check_proc: Some(roll_surge_from_native_spell),
            on_proc: Some(grant_surge),
            ..Default::default()
        },
    );
}

fn roll_surge_from_native_spell(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    if event.actor != aura.caster {
        return false;
    }
    let eligible = event.spell_id.is_some_and(|id| {
        id == SMITE
            || world.resource::<SpellStore>().get(id).is_some_and(|spell| {
                spell
                    .effects
                    .iter()
                    .any(|effect| matches!(effect.effect, 10 | 67 | 136))
            })
    });
    if !eligible {
        return false;
    }
    let chance = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("native109186 retains its chance effect")
        .amount;
    rand::random::<f32>() * 100.0 < chance
}

fn grant_surge(world: &mut World, priest: Entity, _aura: &AuraInstance, _other: Option<Entity>) {
    #[cfg(test)]
    let before = read_surge_stacks(world, priest);
    crate::spell_cast::cast_triggered(world, priest, SURGE_BUFF, Some(priest), 1);
    #[cfg(test)]
    {
        let after = read_surge_stacks(world, priest);
        crate::spell_cast::holy_metrics::record_surge_gain(world, priest, before, after);
    }
}

#[cfg(test)]
fn read_surge_stacks(world: &World, priest: Entity) -> u8 {
    world
        .get::<ActiveAuras>(priest)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == SURGE_BUFF && aura.caster == priest)
        })
        .map_or(0, |aura| aura.stacks)
}

/// 200183: with235587 gain one charge; otherwise reset the Holy Words.
/// Unknown Words have never spent a charge, so ordinary restore is a no-op.
pub(super) fn restore_holy_words(world: &mut World, priest: Entity, _spell: &SpellInfo) {
    let count = if has_aura(world, priest, MIRACLE_WORKER) {
        1
    } else {
        u8::MAX
    };
    let categories: Vec<_> = HOLY_WORDS
        .iter()
        .filter_map(|id| {
            world
                .resource::<SpellStore>()
                .get(*id)?
                .categories
                .as_ref()
                .map(|category| category.charge_category)
        })
        .collect();
    for category in categories {
        crate::spell_cast::restore_charges(world, priest, category, count);
    }
}

/// Native200183s1 amplifies reductions, not the passage of recharge time.
pub(super) fn holy_word_reduction(world: &World, priest: Entity, seconds: f32) -> f64 {
    let bonus = world
        .get::<ActiveAuras>(priest)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == APOTHEOSIS && aura.caster == priest)
        })
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount);
    f64::from(seconds * (1.0 + bonus / 100.0))
}

/// A successful Flash consumes one existing native use before queued heal events
/// can grant another proc. Failed/cancelled casts never reach AfterCast.
pub(super) fn spend_surge(world: &mut World, priest: Entity) {
    let instance = world
        .get::<ActiveAuras>(priest)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == SURGE_BUFF && aura.caster == priest)
        })
        .map(|aura| aura.instance_id);
    if let Some(instance) = instance {
        crate::auras::spend_stack(world, priest, instance);
        #[cfg(test)]
        crate::spell_cast::holy_metrics::record_surge_use(world, priest);
    }
}

/// Native390994s2 names Prayer of Mending casts, not its damage-triggered bounces.
pub(super) fn reduce_word_on_mending(world: &mut World, priest: Entity, _spell: &SpellInfo) {
    const VOICE_OF_HARMONY: u32 = 390994;
    const SERENITY: u32 = 2050;
    if !has_aura(world, priest, VOICE_OF_HARMONY) {
        return;
    }
    let seconds = super::script_effect_value(world, priest, VOICE_OF_HARMONY, 1);
    let seconds = holy_word_reduction(world, priest, seconds);
    super::reduce_cooldown(world, priest, SERENITY, seconds);
}
