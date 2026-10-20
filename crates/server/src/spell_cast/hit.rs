//! Incoming aura defenses, TrinityCore a352b1fa Object::MagicSpellHitResult
//! (Object.cpp:1857-1935), Unit::MeleeSpellHitResult (Unit.cpp:2610-2675).
//! Aura 288 ignores hit direction; only 287 supplies deflection chance.

use crate::random as rand;
use bevy::prelude::*;
use shared::protocol::{CombatLogKind, MissKind};

use super::{TargetPlan, attributes, cast_event, log_event, targets};
use crate::auras::{ActiveAuras, handlers, mechanic_immunity};
use crate::spell_info::{SpellInfo, SpellStore};

const ATTR0_NO_IMMUNITIES: u32 = 0x2000_0000;
const ATTR0_NO_ACTIVE_DEFENSE: u32 = 0x0020_0000;
const ATTR3_COMPLETELY_BLOCKED: u32 = 0x8;
const ATTR8_NO_ATTACK_BLOCK: u32 = 0x1;
const ATTR3_NO_AVOIDANCE: u32 = 0x40;
const ATTR7_NO_ATTACK_MISS: u32 = 0x0200_0000;
const ATTR7_NO_ATTACK_DODGE: u32 = 0x0080_0000;
const MOD_EXPERTISE: u32 = 240;
const MOD_ENEMY_DODGE: u32 = 251;
const EXPERTISE_PER_DODGE_PERCENT: f32 = 4.0;
const MELEE_HIT_DODGE: i32 = 2;
const MELEE_HIT_BLOCK: i32 = 3;
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
        if kind == MissKind::Block && attributes(spell, 3) & ATTR3_COMPLETELY_BLOCKED == 0 {
            continue;
        }
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
            if misses.iter().any(|(missed, kind)| {
                missed == unit
                    && (*kind != MissKind::Block
                        || attributes(spell, 3) & ATTR3_COMPLETELY_BLOCKED != 0)
            }) {
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
        planned.blocked_units = planned
            .units
            .iter()
            .copied()
            .filter(|unit| {
                misses
                    .iter()
                    .any(|(missed, kind)| *missed == *unit && *kind == MissKind::Block)
            })
            .collect();
    }
    // TC UpdateSpellCastDataTargets: immune only when no effect still hits the unit.
    immune_units.retain(|unit| !plan.iter().any(|planned| planned.units.contains(unit)));
    immune_units.sort_unstable();
    immune_units.dedup();
    immune_units
}

/// Unit::SpellHitResult(canReflect=false, canHitPositive=true): a beneficial
/// shield's retaliation still rolls incoming hostile defenses.
pub(crate) fn damage_shield_hits(
    world: &mut World,
    owner: Entity,
    spell: &SpellInfo,
    attacker: Entity,
) -> bool {
    if let Some(kind) = roll_defense(world, owner, spell, attacker, u32::MAX) {
        log_event(
            world,
            cast_event(owner, Some(attacker), spell, CombatLogKind::Miss(kind)),
        );
        return false;
    }
    true
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
    let defense = spell
        .categories
        .as_ref()
        .map_or(0, |categories| categories.defense_type);
    if matches!(defense, DEFENSE_MELEE | DEFENSE_RANGED) {
        return roll_weapon_spell_defense(world, caster, spell, unit, defense);
    }
    let auras = world.get::<ActiveAuras>(unit);
    let can_miss = attributes(spell, 7) & ATTR7_NO_ATTACK_MISS == 0;
    let chance = incoming_miss_chance(world, caster, spell, unit, defense);
    if can_miss && roll_percent(chance) {
        return Some(MissKind::Miss);
    }
    let deflect = spell_deflection_chance(world, caster, unit, defense, auras);
    roll_percent(deflect).then_some(MissKind::Deflect)
}

/// Unit.cpp2622–2743: one uniform integer roll, cumulative truncated buckets.
/// Parry and baseline hit statistics remain gaps.
fn roll_weapon_spell_defense(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    unit: Entity,
    defense: u32,
) -> Option<MissKind> {
    let auras = world.get::<ActiveAuras>(unit);
    let miss = weapon_spell_miss_chance(world, caster, spell, unit, defense);
    let roll = rand::Rng::gen_range(&mut rand::thread_rng(), 0..10_000);
    let mut threshold = spell_chance_units(miss);
    if roll < threshold {
        return Some(MissKind::Miss);
    }
    threshold += spell_chance_units(physical_mechanic_resistance(spell, auras));
    if roll < threshold {
        return Some(MissKind::Resist);
    }
    if attributes(spell, 0) & ATTR0_NO_ACTIVE_DEFENSE != 0 {
        return None;
    }
    threshold += spell_chance_units(spell_deflection_chance(world, caster, unit, defense, auras));
    if roll < threshold {
        return Some(MissKind::Deflect);
    }
    threshold += spell_chance_units(weapon_spell_dodge_chance(
        world, caster, spell, unit, defense,
    ));
    if roll < threshold {
        return Some(MissKind::Dodge);
    }
    threshold += spell_chance_units(spell_block_chance(world, caster, spell, unit, auras));
    (roll < threshold).then_some(MissKind::Block)
}

/// Unit::GetMechanicResistChance: maximum aura total over selected mechanics.
fn physical_mechanic_resistance(spell: &SpellInfo, auras: Option<&ActiveAuras>) -> f32 {
    let Some(auras) = auras else {
        return 0.0;
    };
    let category = spell.categories.as_ref().map_or(0, |row| row.mechanic);
    spell
        .effects
        .iter()
        .take_while(|effect| effect.effect != 0)
        .map(|effect| {
            let mechanic = if effect.mechanic != 0 {
                effect.mechanic
            } else {
                category
            };
            mechanic_resistance_total(auras, mechanic)
        })
        .fold(0.0, f32::max)
}

fn mechanic_resistance_total(auras: &ActiveAuras, mechanic: u32) -> f32 {
    const MOD_MECHANIC_RESISTANCE: u32 = 117;
    if mechanic == 0 {
        return 0.0;
    }
    auras
        .iter()
        .map(|aura| {
            aura.effects
                .iter()
                .filter(|effect| {
                    effect.aura_type == MOD_MECHANIC_RESISTANCE
                        && effect.misc_values[0] as u32 == mechanic
                })
                .map(|effect| effect.amount * aura.stack_multiplier())
                .sum::<f32>()
        })
        .sum()
}

fn weapon_spell_miss_chance(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    unit: Entity,
    defense: u32,
) -> f32 {
    if attributes(spell, 7) & ATTR7_NO_ATTACK_MISS != 0 {
        0.0
    } else {
        incoming_miss_chance(world, caster, spell, unit, defense)
    }
}

/// Unit.cpp2750–2785: player DodgePercentage or NPC baseline/49/level bonus,
/// then caster251 and non-player expertise. Player expertise/248 remain gaps.
fn weapon_spell_dodge_chance(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    unit: Entity,
    defense: u32,
) -> f32 {
    let is_player = world.get::<shared::components::Player>(unit).is_some();
    let is_unit = is_player || world.get::<shared::components::Npc>(unit).is_some();
    if defense != DEFENSE_MELEE || !is_unit {
        return 0.0;
    }
    let disabled = attributes(spell, 7) & ATTR7_NO_ATTACK_DODGE != 0
        || caster_ignores_combat_result(world, caster, spell, MELEE_HIT_DODGE);
    let auras = world.get::<ActiveAuras>(unit);
    let direction_allowed = !is_player || can_deflect_from(world, caster, unit, auras);
    let can_dodge = direction_allowed && can_avoid_weapon_spell(world, unit, auras);
    if disabled || !can_dodge {
        return 0.0;
    }
    let dodge = if is_player {
        world
            .get::<crate::stats::CharacterStats>(unit)
            .expect("player retains native dodge percentage")
            .defense
            .dodge
    } else {
        creature_spell_dodge_chance(world, caster, unit, auras)
    };
    apply_caster_dodge_modifiers(world, caster, dodge)
}

fn apply_caster_dodge_modifiers(world: &World, caster: Entity, dodge: f32) -> f32 {
    let caster_auras = world.get::<ActiveAuras>(caster);
    let enemy_dodge = caster_auras.map_or(0.0, |auras| auras.modifier(MOD_ENEMY_DODGE));
    let expertise = if world.get::<shared::components::Player>(caster).is_some() {
        0.0
    } else {
        caster_auras.map_or(0.0, |auras| {
            auras.modifier(MOD_EXPERTISE) / EXPERTISE_PER_DODGE_PERCENT
        })
    };
    (dodge + enemy_dodge - expertise).max(0.0)
}

/// Specialized Totem/Lightwell summons remain unsupported, as in NPC block.
fn creature_spell_dodge_chance(
    world: &World,
    caster: Entity,
    unit: Entity,
    auras: Option<&ActiveAuras>,
) -> f32 {
    const CREATURE_DODGE_CHANCE: f32 = 3.0;
    const DODGE_CHANCE_PER_LEVEL: f32 = 1.5;
    if world.get::<shared::components::Npc>(unit).is_none() {
        return 0.0;
    }
    let attacker = crate::level_scaling::UnitScaling::of(world, caster);
    let victim = crate::level_scaling::UnitScaling::of(world, unit);
    let level_difference = victim
        .level_against(&attacker)
        .saturating_sub(attacker.level_against(&victim));
    let modifier = auras.map_or(0.0, |auras| auras.modifier(handlers::MOD_DODGE_PERCENT));
    CREATURE_DODGE_CHANCE + modifier + DODGE_CHANCE_PER_LEVEL * f32::from(level_difference)
}

fn can_avoid_weapon_spell(world: &World, unit: Entity, auras: Option<&ActiveAuras>) -> bool {
    let controlled =
        auras.is_some_and(|auras| auras.is_stunned() || auras.is_confused() || auras.is_feared());
    let casting = world.get::<shared::casting::CastState>(unit).is_some();
    !controlled && !casting
}

fn can_defend_from_front(
    world: &World,
    caster: Entity,
    unit: Entity,
    auras: Option<&ActiveAuras>,
) -> bool {
    can_avoid_weapon_spell(world, unit, auras) && can_deflect_from(world, caster, unit, auras)
}

fn spell_chance_units(chance: f32) -> u32 {
    (chance.max(0.0) * 100.0) as u32
}

fn spell_deflection_chance(
    world: &World,
    caster: Entity,
    unit: Entity,
    defense: u32,
    auras: Option<&ActiveAuras>,
) -> f32 {
    let Some(auras) = auras else {
        return 0.0;
    };
    let deflectable = matches!(defense, DEFENSE_MAGIC | DEFENSE_RANGED);
    let controlled = auras.is_stunned() || auras.is_confused();
    if deflectable && !controlled && can_deflect_from(world, caster, unit, Some(auras)) {
        auras.modifier(handlers::DEFLECT_SPELLS)
    } else {
        0.0
    }
}

/// Unit::MeleeSpellHitResult2610–2741: shared active-defense gates precede
/// player shield or native creature block chance. Full hit-table ordering is separate.
fn spell_block_chance(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    unit: Entity,
    auras: Option<&ActiveAuras>,
) -> f32 {
    let disabled = attributes(spell, 0) & ATTR0_NO_ACTIVE_DEFENSE != 0
        || attributes(spell, 8) & ATTR8_NO_ATTACK_BLOCK != 0;
    if disabled || caster_ignores_combat_result(world, caster, spell, MELEE_HIT_BLOCK) {
        return 0.0;
    }
    if !can_defend_from_front(world, caster, unit, auras) {
        return 0.0;
    }
    if world.get::<shared::components::Player>(unit).is_some() {
        return world
            .get::<crate::stats::CharacterStats>(unit)
            .map_or(0.0, |stats| {
                if stats.defense.shield_block > 0.0 {
                    stats.defense.block
                } else {
                    0.0
                }
            });
    }
    creature_spell_block_chance(world, caster, unit, auras)
}

/// Unit::GetUnitBlockChance2832–2861. Specialized Totem/Lightwell summons
/// remain unsupported by the summon runtime; template creature type is not IsTotem.
fn creature_spell_block_chance(
    world: &World,
    caster: Entity,
    unit: Entity,
    auras: Option<&ActiveAuras>,
) -> f32 {
    // TC a352b1fa CreatureData.h348; 0x40 is NO_XP, not NO_BLOCK.
    const CREATURE_FLAG_EXTRA_NO_BLOCK: u32 = 0x10;
    const CREATURE_BLOCK_CHANCE: f32 = 3.0;
    const BLOCK_CHANCE_PER_LEVEL: f32 = 1.5;
    let Some(npc) = world.get::<shared::components::Npc>(unit) else {
        return 0.0;
    };
    let template = world
        .resource::<crate::world_data::WorldData>()
        .templates
        .get(&npc.template_id)
        .unwrap_or_else(|| panic!("missing sourced block flags for NPC{}", npc.template_id));
    if template.flags_extra & CREATURE_FLAG_EXTRA_NO_BLOCK != 0 {
        return 0.0;
    }
    let attacker = crate::level_scaling::UnitScaling::of(world, caster);
    let victim = crate::level_scaling::UnitScaling::of(world, unit);
    let level_difference = victim
        .level_against(&attacker)
        .saturating_sub(attacker.level_against(&victim));
    let modifier = auras.map_or(0.0, |auras| auras.modifier(handlers::MOD_BLOCK_PERCENT));
    (CREATURE_BLOCK_CHANCE + modifier + BLOCK_CHANCE_PER_LEVEL * f32::from(level_difference))
        .max(0.0)
}

fn caster_ignores_combat_result(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    result: i32,
) -> bool {
    // SpellAuraDefines.h289; aura186 is incoming spell-hit chance instead.
    const IGNORE_COMBAT_RESULT: u32 = 202;
    world.get::<ActiveAuras>(caster).is_some_and(|auras| {
        auras.iter().any(|aura| {
            aura.effects.iter().any(|effect| {
                effect.aura_type == IGNORE_COMBAT_RESULT
                    && effect.misc_values[0] == result
                    && crate::auras::spell_mods::class_mask_affects(aura, effect, spell)
            })
        })
    })
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

fn can_deflect_from(
    world: &World,
    caster: Entity,
    unit: Entity,
    auras: Option<&ActiveAuras>,
) -> bool {
    if auras.is_some_and(|auras| auras.has_type(handlers::IGNORE_HIT_DIRECTION)) {
        return true;
    }
    let (Some(from), Some(to)) = (
        targets::position(world, unit),
        targets::position(world, caster),
    ) else {
        return false;
    };
    let angle = targets::orientation(world, unit);
    targets::in_front_arc(from, to, angle)
}

fn roll_percent(chance: f32) -> bool {
    chance > 0.0 && rand::random::<f32>() * 100.0 < chance
}

#[cfg(test)]
mod tests {
    use super::{DEFENSE_MELEE, spell_chance_units, weapon_spell_dodge_chance};

    #[test]
    fn native_npc_dodge_chance_follows_authored_tuning_and_exact_level_bonus() {
        use crate::auras::{RemoveReason, remove_auras};
        use crate::spell_cast::npc_dodge_tests::spawn_native_dodge_target;
        use shared::components::{Level, UnitLevel};

        // Native40 tuning73 clamps1..30; native626 tuning6 clamps5..30.
        for (entry, level, expected) in [(40, 10, 3.0), (40, 31, 3.0), (626, 1, 9.0), (626, 2, 7.5)]
        {
            let mut h = spawn_native_dodge_target(entry, 0.0);
            h.app
                .world_mut()
                .entity_mut(h.caster)
                .insert((Level(level), UnitLevel(level)));
            h.advance(2);
            let spell = crate::spell_cast::test_harness::world_db()
                .unwrap()
                .store
                .get(38598)
                .unwrap();
            let chance =
                weapon_spell_dodge_chance(h.app.world(), h.caster, spell, h.npc, DEFENSE_MELEE);
            assert_eq!(
                chance,
                expected + 100.0,
                "native49 plus level chance for NPC{entry}/player{level}"
            );
            remove_auras(h.app.world_mut(), h.npc, RemoveReason::Cancelled, |aura| {
                aura.spell_id == 70188
            });
            h.advance(2);
            let chance =
                weapon_spell_dodge_chance(h.app.world(), h.caster, spell, h.npc, DEFENSE_MELEE);
            assert_eq!(
                chance, expected,
                "native baseline/level chance for NPC{entry}/player{level}"
            );
            assert_eq!(spell_chance_units(chance), (expected * 100.0) as u32);
        }
    }

    #[test]
    fn weapon_spell_probability_truncates_to_native_hundredths() {
        assert_eq!(spell_chance_units(1.239), 123);
        assert_eq!(spell_chance_units(50.0), 5000);
        assert_eq!(spell_chance_units(58.0), 5800);
        assert_eq!(spell_chance_units(150.0), 15_000);
    }

    #[test]
    fn weapon_spell_negative_probability_has_no_hit_table_bucket() {
        assert_eq!(spell_chance_units(-50.0), 0);
        assert_eq!(spell_chance_units(0.0), 0);
    }
}
