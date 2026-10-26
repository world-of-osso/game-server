//! TC a352b1fa spell_mage.cpp1037–1129, with native70448 driver195283 (Effect3).
//! SimC76f8118 sc_mage.cpp2427: only the cast admitted with Hot Streak spends it.
use crate::random as rand;
use bevy::prelude::*;

use crate::auras::{ActiveAuras, RemoveReason, remove_instance};
use crate::class_progression::KnownSpells;
use crate::spell_info::{ChargedSpellModifier, SpellInfo};

const DRIVER: u32 = 195283;
const HEATING_UP: u32 = 48107;
const PYROCLASM_TALENT: u32 = 269650;
const PYROCLASM: u32 = 269651;
pub(crate) const HOT_STREAK: u32 = 48108;
pub(super) const PYROBLAST: u32 = 11366;
pub(super) const FLAMESTRIKE: u32 = 2120;
pub(super) const NATIVE_FLAMESTRIKE: u32 = 1254851;

/// SimC76f8118 sc_mage.cpp6988–7010: only Hot Streak producers enter its flight count.
pub(crate) fn triggers_hot_streak(spell_id: u32) -> bool {
    matches!(spell_id, 133 | 108853 | 319836 | PYROBLAST | 2948)
}

pub(crate) fn record_fire_hit(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    crit: bool,
    guaranteed: bool,
) {
    let eligible = triggers_hot_streak(spell.id);
    let learned = world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(DRIVER));
    if !eligible || !learned {
        return;
    }
    apply_hot_streak_outcome(world, caster, crit, guaranteed);
}

fn apply_hot_streak_outcome(world: &mut World, caster: Entity, crit: bool, guaranteed: bool) {
    // SimC76f8118 sc_mage.cpp2138–2159/2186–2192: guaranteed outcomes
    // predict HU, and known-HU conversions predict HS; wasted crits keep HS.
    if crit && crate::auras::has_aura(world, caster, HOT_STREAK) {
        if guaranteed {
            #[cfg(any(test, feature = "test-support"))]
            crate::simc_apl_native::reaction::predict(world, caster, HOT_STREAK);
        }
        return;
    }
    #[cfg(any(test, feature = "test-support"))]
    let reacted =
        crate::simc_apl_native::reaction::has_observed_reaction(world, caster, HEATING_UP);
    let now = crate::auras::aura_now(world);
    let heating_up = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == HEATING_UP && aura.caster == caster)
        })
        .map(|aura| {
            let protected = aura.expires_at.is_some_and(|expiry| {
                let applied_at = expiry - f64::from(aura.duration_ms) / 1000.0;
                now - applied_at <= 0.2
            });
            (aura.instance_id, protected)
        });
    // SimC76f8118 sc_mage.cpp2196–2210: an ordinary noncritical hit
    // cannot remove Heating Up during its first200ms.
    if !crit && heating_up.is_some_and(|(_, protected)| protected) {
        return;
    }
    let heating_up = heating_up.map(|(instance, _)| instance);
    if let Some(instance) = heating_up {
        remove_instance(world, caster, instance, RemoveReason::Consumed);
    }
    if crit {
        let proc = if heating_up.is_some() {
            HOT_STREAK
        } else {
            HEATING_UP
        };
        crate::spell_cast::cast_triggered(world, caster, proc, Some(caster), 1);
        if guaranteed {
            #[cfg(any(test, feature = "test-support"))]
            if heating_up.is_none() || reacted {
                crate::simc_apl_native::reaction::predict(world, caster, proc);
            }
        }
    }
}

/// SimC76f8118 sc_mage.cpp3913–3920: one independent Fuel roll per cast
/// that hits any recipient, using the cast's crit snapshot before proc consumption.
pub(crate) fn flamestrike_hot_streak_chance(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    hit: Option<Entity>,
) -> Option<f32> {
    if !matches!(spell.id, FLAMESTRIKE | NATIVE_FLAMESTRIKE) {
        return None;
    }
    let hit = hit?;
    let known = world.get::<KnownSpells>(caster)?;
    if !known.contains(DRIVER) {
        return None;
    }
    Some(if known.contains(416094) {
        crate::spell_cast::effects::crit_chance(world, caster, hit, spell)
    } else {
        0.0
    })
}

pub(crate) fn roll_flamestrike_hot_streak(world: &mut World, caster: Entity, chance: f32) {
    let crit = rand::random::<f32>() * 100.0 < chance;
    apply_hot_streak_outcome(world, caster, crit, chance >= 100.0);
}

pub(super) fn admitted_hot_streak(world: &World, caster: Entity, spell: &SpellInfo) -> Option<u32> {
    world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras.iter().find(|aura| {
                aura.spell_id == HOT_STREAK && spell.cast_proc_mods.contains(&aura.instance_id)
            })
        })
        .map(|aura| aura.instance_id)
}

fn consume_admitted_hot_streak(world: &mut World, caster: Entity, spell: &SpellInfo) -> bool {
    let Some(instance) = admitted_hot_streak(world, caster, spell) else {
        return false;
    };
    remove_instance(world, caster, instance, RemoveReason::Consumed);
    true
}

/// MAGE_PYROCLASM_SIMC_HOT_STREAK_TRIGGER, SimC76f8118 sc_mage.cpp2326–2382:
/// positive admitted cast time uses one live buff's damage value, not stack count.
/// Retain it in the ordinary launch snapshot before that stack is consumed.
pub(crate) fn pyroclasm_damage_modifier(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> Option<ChargedSpellModifier> {
    if !spell.cast_was_hardcast || !matches!(spell.id, PYROBLAST | FLAMESTRIKE | NATIVE_FLAMESTRIKE)
    {
        return None;
    }
    let aura = world
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.spell_id == PYROCLASM)?;
    let amount = aura.effects.iter().find(|effect| effect.index == 0)?.amount;
    Some(ChargedSpellModifier {
        aura_spell_id: aura.spell_id,
        op: crate::auras::SpellModOp::HealingAndDamage as i32,
        percent: true,
        amount,
    })
}

/// SimC76f8118 sc_mage.cpp2422–2428: hardcasts spend first; an actual Hot
/// Streak expenditure then rolls the new proc. Native70448 supplies chance/value.
pub(crate) fn consume_procs_and_roll_pyroclasm(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
) {
    if spell.cast_was_hardcast {
        let instance = world
            .get::<ActiveAuras>(caster)
            .and_then(|auras| auras.iter().find(|aura| aura.spell_id == PYROCLASM))
            .map(|aura| aura.instance_id);
        if let Some(instance) = instance {
            crate::auras::spend_stack(world, caster, instance);
        }
    }
    let spent_hot_streak = consume_admitted_hot_streak(world, caster, spell);
    if spent_hot_streak {
        roll_pyroclasm(world, caster);
    }
    super::fire_procs::after_hot_streak_cast(world, caster, spent_hot_streak);
}

fn roll_pyroclasm(world: &mut World, caster: Entity) {
    // SimC76f8118 sc_mage.cpp6446: chance belongs to talent269650.
    // Native MID2's label5947 modifies buff269651, not talent label5949.
    let chance = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == PYROCLASM_TALENT))
        .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
        .map_or(0.0, |effect| effect.amount);
    if chance > 0.0 && rand::random::<f32>() * 100.0 < chance {
        crate::spell_cast::cast_triggered(world, caster, PYROCLASM, Some(caster), 1);
    }
}
