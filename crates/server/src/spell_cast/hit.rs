//! Incoming aura defenses, TrinityCore a352b1fa Object::MagicSpellHitResult
//! (Object.cpp:1857-1935), Unit::MeleeSpellHitResult (Unit.cpp:2610-2675).
//! Aura 288 ignores hit direction; only 287 supplies deflection chance.

use bevy::prelude::*;
use shared::protocol::{CombatLogKind, MissKind};

use super::{TargetPlan, attributes, cast_event, log_event, targets};
use crate::auras::{ActiveAuras, handlers};
use crate::spell_info::{SpellInfo, SpellStore};

const ATTR0_NO_IMMUNITIES: u32 = 0x2000_0000;
const ATTR3_NO_AVOIDANCE: u32 = 0x40;
const ATTR7_NO_ATTACK_MISS: u32 = 0x0200_0000;
const DEFENSE_MAGIC: u32 = 1;
const DEFENSE_MELEE: u32 = 2;
const DEFENSE_RANGED: u32 = 3;

pub(super) fn remove_defended_targets(
    world: &mut World,
    caster: Entity,
    spell: &SpellInfo,
    plan: &mut TargetPlan,
) {
    let negative =
        crate::spell_info::positivity::negative_effects(world.resource::<SpellStore>(), spell);
    let misses: Vec<_> = super::hit_units(plan)
        .into_iter()
        .filter(|&unit| unit != caster)
        .filter_map(|unit| {
            roll_defense(world, caster, spell, unit, negative).map(|kind| (unit, kind))
        })
        .collect();
    let mut immune_units = Vec::new();
    for planned in plan.iter_mut() {
        let effect = &spell.effects[planned.index];
        if negative & (1 << effect.index) == 0 {
            continue;
        }
        planned.units.retain(|unit| {
            if misses.iter().any(|(missed, _)| missed == unit) {
                return false;
            }
            if attributes(spell, 0) & ATTR0_NO_IMMUNITIES == 0
                && immune_to_mechanic(world, *unit, effect.mechanic)
            {
                immune_units.push(*unit);
                return false;
            }
            true
        });
    }
    immune_units.sort_unstable();
    immune_units.dedup();
    for (unit, kind) in misses.into_iter().chain(
        immune_units
            .into_iter()
            .map(|unit| (unit, MissKind::Immune)),
    ) {
        log_event(
            world,
            cast_event(caster, Some(unit), spell, CombatLogKind::Miss(kind)),
        );
    }
}

fn immune_to_mechanic(world: &World, unit: Entity, mechanic: u32) -> bool {
    mechanic != 0
        && world.get::<ActiveAuras>(unit).is_some_and(|auras| {
            auras.iter().flat_map(|aura| &aura.effects).any(|effect| {
                effect.aura_type == handlers::MECHANIC_IMMUNITY
                    && effect.misc_values[0] as u32 == mechanic
            })
        })
}

fn roll_defense(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    unit: Entity,
    negative: u32,
) -> Option<MissKind> {
    if negative == 0 {
        return None;
    }
    let mechanic = spell
        .categories
        .as_ref()
        .map_or(0, |categories| categories.mechanic);
    if attributes(spell, 0) & ATTR0_NO_IMMUNITIES == 0 && immune_to_mechanic(world, unit, mechanic)
    {
        return Some(MissKind::Immune);
    }
    if attributes(spell, 3) & ATTR3_NO_AVOIDANCE != 0 {
        return None;
    }
    let auras = world.get::<ActiveAuras>(unit);
    let defense = spell
        .categories
        .as_ref()
        .map_or(0, |categories| categories.defense_type);
    if attributes(spell, 7) & ATTR7_NO_ATTACK_MISS == 0 {
        let baseline = if defense == DEFENSE_MELEE
            && world.get::<shared::components::Npc>(caster).is_some()
            && targets::is_hostile(world, caster, unit)
        {
            crate::combat::creature_miss_chance(world.get::<crate::auras::AuraStatMods>(caster))
        } else {
            0.0
        };
        let modifier = auras.map_or(0.0, |auras| {
            attacker_hit_modifier(auras, defense, spell.school_mask())
        });
        let miss = baseline - modifier;
        if roll_percent(miss) {
            return Some(MissKind::Miss);
        }
    }
    let auras = auras?;
    let deflectable = matches!(defense, DEFENSE_MAGIC | DEFENSE_RANGED);
    let controlled = auras.is_stunned() || auras.is_confused();
    if deflectable
        && !controlled
        && can_deflect_from(world, caster, unit, auras)
        && roll_percent(auras.modifier(handlers::DEFLECT_SPELLS))
    {
        return Some(MissKind::Deflect);
    }
    None
}

fn attacker_hit_modifier(auras: &ActiveAuras, defense: u32, school: u32) -> f32 {
    let aura_type = match defense {
        DEFENSE_MELEE => handlers::MOD_ATTACKER_MELEE_HIT_CHANCE,
        DEFENSE_RANGED => handlers::MOD_ATTACKER_RANGED_HIT_CHANCE,
        DEFENSE_MAGIC => handlers::MOD_ATTACKER_SPELL_HIT_CHANCE,
        _ => return 0.0,
    };
    if defense != DEFENSE_MAGIC {
        return auras.modifier(aura_type);
    }
    auras
        .iter()
        .map(|aura| {
            let stacks = f32::from(aura.stacks.max(1));
            aura.effects
                .iter()
                .filter(|effect| {
                    effect.aura_type == aura_type && effect.misc_values[0] as u32 & school != 0
                })
                .map(|effect| effect.amount * stacks)
                .sum::<f32>()
        })
        .sum()
}

fn can_deflect_from(world: &World, caster: Entity, unit: Entity, auras: &ActiveAuras) -> bool {
    if auras.has_type(handlers::IGNORE_HIT_DIRECTION) {
        return true;
    }
    let (Some(from), Some(to)) = (
        targets::position(world, unit),
        targets::position(world, caster),
    ) else {
        return false;
    };
    let angle = targets::orientation(world, unit);
    let forward = Vec3::new(angle.cos(), 0.0, -angle.sin());
    forward.dot(to - from) >= 0.0
}

fn roll_percent(chance: f32) -> bool {
    chance > 0.0 && rand::random::<f32>() * 100.0 < chance
}
