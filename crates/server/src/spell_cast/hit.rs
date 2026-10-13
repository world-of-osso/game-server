//! Incoming aura defenses, TrinityCore a352b1fa Object::MagicSpellHitResult
//! (Object.cpp:1857-1935), Unit::MeleeSpellHitResult (Unit.cpp:2610-2675).
//! Aura 288 ignores hit direction; only 287 supplies deflection chance.

use bevy::prelude::*;
use shared::protocol::{CombatLogKind, MissKind};

use super::{TargetPlan, attributes, cast_event, log_event, targets};
use crate::auras::{ActiveAuras, handlers, mechanic_immunity};
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
    let immune_units = remove_immune_effect_targets(world, spell, plan, negative, &misses);
    let immune_misses = immune_units
        .into_iter()
        .map(|unit| (unit, MissKind::Immune));
    for (unit, kind) in misses.into_iter().chain(immune_misses) {
        log_event(
            world,
            cast_event(caster, Some(unit), spell, CombatLogKind::Miss(kind)),
        );
    }
}

fn remove_immune_effect_targets(
    world: &World,
    spell: &SpellInfo,
    plan: &mut TargetPlan,
    negative: u32,
    misses: &[(Entity, MissKind)],
) -> Vec<Entity> {
    let mut immune_units = Vec::new();
    for planned in plan.iter_mut() {
        let effect = &spell.effects[planned.index];
        let harmful = negative & (1 << effect.index) != 0;
        planned.units.retain(|unit| {
            // A whole-spell miss removes every effect on that unit. Numeric
            // Dummy markers cannot turn an immune primary into a successful hit.
            if misses.iter().any(|(missed, _)| missed == unit) {
                return false;
            }
            if crate::auras::effect_immunity::blocks_effect(world, *unit, spell, effect)
                || harmful
                    && mechanic_immunity::blocks_mechanic(world, *unit, spell, effect.mechanic)
            {
                immune_units.push(*unit);
                return false;
            }
            true
        });
    }
    immune_units.sort_unstable();
    immune_units.dedup();
    immune_units
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
    // Object::SpellHitResult/Unit::IsImmunedToSpell checks school immunity
    // before spending resources, not only when damage is finally applied.
    let school_immune = attributes(spell, 0) & ATTR0_NO_IMMUNITIES == 0
        && crate::auras::immune_to_school(world.get::<ActiveAuras>(unit), spell.school_mask());
    if school_immune || mechanic_immunity::blocks_mechanic(world, unit, spell, 0) {
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
    let can_miss = attributes(spell, 7) & ATTR7_NO_ATTACK_MISS == 0;
    let chance = incoming_miss_chance(world, caster, spell, unit, defense);
    if can_miss && roll_percent(chance) {
        return Some(MissKind::Miss);
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

fn incoming_miss_chance(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    unit: Entity,
    defense: u32,
) -> f32 {
    let creature_melee = defense == DEFENSE_MELEE
        && world.get::<shared::components::Npc>(caster).is_some()
        && targets::is_hostile(world, caster, unit);
    let baseline = if creature_melee {
        crate::combat::creature_miss_chance(world.get::<crate::auras::AuraStatMods>(caster))
    } else {
        0.0
    };
    let modifier = world.get::<ActiveAuras>(unit).map_or(0.0, |auras| {
        attacker_hit_modifier(auras, defense, spell.school_mask())
    });
    // Creature melee baseline already consumes caster aura54. Other physical
    // spell paths must also subtract it (Unit::MeleeSpellMissChance, Unit.cpp12525).
    let caster_modifier = if !creature_melee && matches!(defense, DEFENSE_MELEE | DEFENSE_RANGED) {
        world
            .get::<ActiveAuras>(caster)
            .map_or(0.0, |auras| auras.modifier(handlers::MOD_HIT_CHANCE))
    } else {
        0.0
    };
    baseline - caster_modifier - modifier
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
