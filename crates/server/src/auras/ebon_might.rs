//! Active70448 Ebon Might only; not a general support540 implementation.
//! SimC76f8118 sc_evoker.cpp4731–5073/9000/10725; buff.cpp3553–3563.
use bevy::prelude::*;
use shared::components::Player;

use super::{ActiveAuras, AuraInstance};
use crate::class_progression::ActiveSpec;
use crate::spell_info::{ClassSpells, SpecInfo, SpellInfo};
use crate::stats::{CharacterStats, PrimaryStat, StatModifiers};

pub(crate) const EBON: u32 = 395_152;
pub(super) const SELF_BUFF: u32 = 395_296;
const SUPPORT_STAT: u32 = 540;

fn specialization(world: &World, unit: Entity) -> Option<&SpecInfo> {
    let player = world.get::<Player>(unit)?;
    let active = world.get::<ActiveSpec>(unit)?;
    world
        .get_resource::<ClassSpells>()?
        .classes
        .iter()
        .find(|class| class.class_id == player.class)?
        .specs
        .iter()
        .find(|spec| spec.spec_id == active.0)
}

fn is_damage_player(world: &World, unit: Entity) -> bool {
    // Native ChrSpecializationRole:0 tank,1 healer,2 damage. No guessed class role.
    specialization(world, unit).is_some_and(|spec| spec.role == 2)
}

/// Apply native friendliness/map/radius/player/exclude-caster rules first,
/// then filter the known damage roles before the authored target cap.
pub(crate) fn filter_allies(
    world: &World,
    spell: &SpellInfo,
    _explicit: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    units.retain(|&unit| is_damage_player(world, unit));
    let cap = spell
        .target_restrictions
        .as_ref()
        .expect("native Ebon target restriction")
        .max_targets;
    if cap > 0 {
        units.truncate(cap as usize);
    }
}

pub(crate) fn cast_self_buff(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    crate::spell_cast::cast_triggered(world, caster, SELF_BUFF, Some(caster), 1);
    update_owned_support(world, caster);
}

fn support_amount(aura: &AuraInstance) -> f32 {
    aura.effects
        .iter()
        .find(|effect| effect.index == 1 && effect.aura_type == SUPPORT_STAT)
        .map_or(0.0, |effect| effect.amount * aura.stack_multiplier())
}

pub(super) fn primary_bonus(auras: &ActiveAuras) -> f32 {
    auras
        .iter()
        .filter(|aura| aura.spell_id == EBON)
        .map(support_amount)
        .sum()
}

/// Use the existing concrete primary-flat representation, selected from native
/// spec metadata. Unknown specialization is an explicit unsupported boundary.
pub(super) fn add_primary_bonus(
    world: &World,
    unit: Entity,
    amount: f32,
    stats: &mut StatModifiers,
) {
    if amount == 0.0 {
        return;
    }
    let Some(spec) = specialization(world, unit) else {
        warn!(
            ?unit,
            "Ebon Might primary support requires native specialization metadata"
        );
        return;
    };
    match PrimaryStat::from_priority(spec.primary_stat_priority) {
        PrimaryStat::Strength => stats.primary_flat.strength += amount,
        PrimaryStat::Agility => stats.primary_flat.agility += amount,
        PrimaryStat::Intellect => stats.primary_flat.intellect += amount,
    }
}

fn foreign_intellect(world: &World, caster: Entity) -> f32 {
    let raw: f32 = world.get::<ActiveAuras>(caster).map_or(0.0, |auras| {
        auras
            .iter()
            .filter(|aura| aura.spell_id == EBON && aura.caster != caster)
            .map(support_amount)
            .sum()
    });
    let modifiers = world
        .get::<StatModifiers>(caster)
        .copied()
        .unwrap_or_default();
    // Preserve this server's existing flat-bonus → bonus-pct → total-pct order.
    let bonus = 1.0 + modifiers.primary_bonus_pct.intellect / 100.0;
    let total = 1.0 + modifiers.primary_pct.intellect / 100.0;
    raw * bonus * total
}

fn own_intellect(world: &World, caster: Entity) -> f32 {
    let intellect = world
        .get::<CharacterStats>(caster)
        .expect("Ebon caster computed stats")
        .primary
        .intellect;
    intellect - foreign_intellect(world, caster)
}

fn owned_ebons(world: &mut World, caster: Entity) -> Vec<(Entity, u32)> {
    world
        .query::<(Entity, &ActiveAuras)>()
        .iter(world)
        .flat_map(|(unit, auras)| {
            auras
                .iter()
                .filter(move |aura| aura.spell_id == EBON && aura.caster == caster)
                .map(move |aura| (unit, aura.instance_id))
        })
        .collect()
}

fn update_amount(world: &mut World, unit: Entity, instance: u32, amount: f32) {
    let mut auras = world
        .get_mut::<ActiveAuras>(unit)
        .expect("owned Ebon recipient");
    let aura = auras
        .get_mut(instance)
        .expect("collected owned Ebon instance");
    let Some(effect) = aura
        .effects
        .iter_mut()
        .find(|effect| effect.index == 1 && effect.aura_type == SUPPORT_STAT)
    else {
        return; // A partial effect mask carries no primary support to update.
    };
    if effect.amount != amount {
        effect.amount = amount;
        super::auras_changed(world, unit);
    }
}

fn update_owned_support(world: &mut World, caster: Entity) {
    let recipients = owned_ebons(world, caster);
    if recipients.is_empty() {
        return;
    }
    let percent = crate::spell_cast::scripts::script_effect_value(world, caster, EBON, 0);
    let threshold = crate::spell_cast::scripts::script_effect_value(world, caster, EBON, 2);
    let split = if recipients.len() as f32 > threshold {
        threshold / recipients.len() as f32
    } else {
        1.0
    };
    let amount = own_intellect(world, caster) * (percent / 100.0) * split;
    for (unit, instance) in recipients {
        update_amount(world, unit, instance, amount);
    }
}

pub(super) fn refresh_owned(world: &mut World, _unit: Entity, aura: &AuraInstance, index: u8) {
    if index == 6 {
        update_owned_support(world, aura.caster);
    }
}

pub(super) fn consumes_field(spell: u32, index: u8) -> bool {
    matches!((spell, index), (EBON, 0 | 1) | (SELF_BUFF, 6))
}
