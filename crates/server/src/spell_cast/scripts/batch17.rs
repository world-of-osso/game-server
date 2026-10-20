//! Local Mainline12.1.0.69933 named resources, rank consumers and healing hooks.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::script_effect_value;
use crate::auras::{ActiveAuras, AuraApplication, AuraInstance, AuraSource, RemoveReason};
use crate::spell_cast;
use crate::spell_info::{SpellInfo, SpellStore};

pub(super) const SHADOW_BOLT: u32 = 686;
pub(super) const CONFLAGRATE: u32 = 17962;
pub(super) const SOUL_FIRE: u32 = 6353;
pub(super) const BLOOD_BOIL: u32 = 50842;
pub(super) const MARROWREND: u32 = 195182;
pub(super) const REGROWTH: u32 = 8936;
const DEMONOLOGY_SPEC: u32 = 266;
const SHADOW_BOLT_POWER: u32 = 194192;
const CONFLAGRATE_POWER: u32 = 245330;
const SOUL_FIRE_POWER: u32 = 281490;
const BACKDRAFT: u32 = 196406;
const BACKDRAFT_BUFF: u32 = 117828;
const IMMOLATE_DOT: u32 = 157736;
const WITHER_TALENT: u32 = 445465;
const WITHER_DOT: u32 = 445474;
const BLOOD_PLAGUE: u32 = 55078;
const BLOOD_BOIL_REPLACEMENT: u32 = 212744;
const BONE_SHIELD: u32 = 195181;
const IMPROVED_REGROWTH: u32 = 231032;
pub(crate) const LIFEBLOOM: u32 = 33763;
const LIFEBLOOM_HEAL: u32 = 33778;
const LIFEBLOOM_REFRESH_BLOOM_FRACTION: f64 = 0.3;
pub(crate) const CENARION_WARD: u32 = 102351;
const CENARION_HEAL: u32 = 102352;

///686's c2 condition is Demonology (ChrSpecialization266/OrderIndex1).
///194192 explicitly refers back to686 and supplies ten raw shard units.
pub(super) fn energize_shadow_bolt(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == DEMONOLOGY_SPEC)
    {
        spell_cast::cast_triggered(world, caster, SHADOW_BOLT_POWER, Some(caster), 1);
    }
}

///6353 explicitly names281490s1; one grant per completed root cast.
pub(super) fn energize_soul_fire(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    spell_cast::cast_triggered(world, caster, SOUL_FIRE_POWER, Some(caster), 1);
}

///17962 names245330s1 fragments and196406 gates117828. Apply one use;
///CumulativeAura supplies the native cap, and ordinary proc charges consume it.
pub(super) fn conflagrate(world: &mut World, caster: Entity, target: Entity, depth: u8) {
    if !super::batch8::first_damage_target(world, caster, CONFLAGRATE, target) {
        return;
    }
    spell_cast::cast_triggered(world, caster, CONFLAGRATE_POWER, Some(caster), depth + 1);
    if crate::auras::has_aura(world, caster, BACKDRAFT) {
        let buff = world
            .resource::<SpellStore>()
            .get(BACKDRAFT_BUFF)
            .expect("Backdraft spell");
        //267115s1 is a flat Charges modifier; s2 separately raises the stack cap.
        let uses = crate::auras::spell_mods::modify(
            world,
            caster,
            buff,
            crate::auras::SpellModOp::Charges,
            1.0,
        ) as u32;
        for _ in 0..uses {
            spell_cast::cast_triggered(world, caster, BACKDRAFT_BUFF, Some(caster), depth + 1);
        }
    }
}

///6353's445465 condition selects the named Immolate/Wither periodic child.
///Only base periodic effect0 is established here, not optional vulnerabilities.
pub(super) fn apply_soul_fire_dot(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    if !landed_damage(world, caster, target, SOUL_FIRE) {
        return;
    }
    let spell_id = if crate::auras::has_aura(world, caster, WITHER_TALENT) {
        WITHER_DOT
    } else {
        IMMOLATE_DOT
    };
    apply_named_aura(world, caster, target, spell_id, 1);
}

///50842 names55078 unless learned212744 selects the replacement branch.
///The referenced rank has no local store row: do not synthesize a passive aura.
pub(super) fn apply_blood_plague(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    if !landed_damage(world, caster, target, BLOOD_BOIL) {
        return;
    }
    let replaced = world
        .get::<crate::class_progression::KnownSpells>(caster)
        .is_some_and(|known| known.contains(BLOOD_BOIL_REPLACEMENT));
    if !replaced {
        apply_named_aura(world, caster, target, BLOOD_PLAGUE, 1);
    }
}

///195182s3 grants charges to explicitly named195181; normal stacking caps them.
pub(super) fn grant_bone_shield(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    if !landed_damage(world, caster, target, MARROWREND) {
        return;
    }
    let charges = script_effect_value(world, caster, MARROWREND, 2) as u32;
    for _ in 0..charges {
        apply_named_aura(world, caster, caster, BONE_SHIELD, u32::MAX);
    }
}

///The production hit ledger includes fully absorbed hits but excludes immunity.
fn landed_damage(world: &World, caster: Entity, target: Entity, spell_id: u32) -> bool {
    world
        .resource::<spell_cast::ResolvingHits>()
        .0
        .last()
        .is_some_and(|frame| {
            frame.caster == caster
                && frame.spell_id == spell_id
                && frame.damage.iter().any(|(unit, _)| *unit == target)
        })
}

fn apply_named_aura(world: &mut World, caster: Entity, target: Entity, spell_id: u32, mask: u32) {
    crate::auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask: mask,
            source: AuraSource::Cast,
        },
    );
}

///231032 adds its native probability to the initial Regrowth heal only when
///the target already has Regrowth. The tooltip does not restrict HoT ownership.
pub(super) fn regrowth_crit(world: &World, caster: Entity, target: Entity) -> Option<f32> {
    if !crate::auras::has_aura(world, caster, IMPROVED_REGROWTH)
        || !crate::auras::has_aura(world, target, REGROWTH)
    {
        return None;
    }
    let spell = world
        .resource::<SpellStore>()
        .get(REGROWTH)
        .expect("Regrowth row");
    let base = world
        .get::<crate::stats::CharacterStats>(caster)
        .map_or(0.0, |stats| stats.crit_pct);
    let modified = crate::auras::spell_mods::modify(
        world,
        caster,
        spell,
        crate::auras::SpellModOp::CritChance,
        base,
    );
    Some(modified + script_effect_value(world, caster, IMPROVED_REGROWTH, 0))
}

/// LIFEBLOOM_REFRESH_BLOOM_THRESHOLD_SIMC: SimC76f8118 sc_druid.cpp6229–6240.
/// Read the old owned deadline before native application replaces it.
pub(super) fn bloom_lifebloom_before_refresh(
    world: &mut World,
    caster: Entity,
    target: Entity,
    spell: u32,
    depth: u8,
) {
    if spell != LIFEBLOOM {
        return;
    }
    let now = crate::auras::aura_now(world);
    let near_expiration = world
        .get::<ActiveAuras>(target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == LIFEBLOOM && aura.caster == caster)
        })
        .is_some_and(|aura| {
            let threshold_seconds =
                f64::from(aura.duration_ms) / 1000.0 * LIFEBLOOM_REFRESH_BLOOM_FRACTION;
            aura.expires_at
                .is_some_and(|expiry| expiry - now <= threshold_seconds)
        });
    if near_expiration {
        spell_cast::cast_triggered(world, caster, LIFEBLOOM_HEAL, Some(target), depth + 1);
    }
}

///33763 names33778 for expiry/dispel; refresh is handled before application.
pub(crate) fn bloom_lifebloom(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    reason: RemoveReason,
) {
    if matches!(reason, RemoveReason::Expired | RemoveReason::Dispelled) {
        spell_cast::cast_triggered(world, aura.caster, LIFEBLOOM_HEAL, Some(unit), 1);
    }
}

///33763 permits one target per caster; foreign owners remain independent.
pub(crate) fn replace_lifebloom(world: &mut World, unit: Entity, aura: &AuraInstance) {
    let mut query = world.query::<(Entity, &ActiveAuras)>();
    let previous: Vec<Entity> = query
        .iter(world)
        .filter(|(target, auras)| {
            *target != unit
                && auras
                    .iter()
                    .any(|old| old.spell_id == LIFEBLOOM && old.caster == aura.caster)
        })
        .map(|(target, _)| target)
        .collect();
    for target in previous {
        crate::auras::remove_auras(world, target, RemoveReason::Replaced, |old| {
            old.spell_id == LIFEBLOOM && old.caster == aura.caster
        });
    }
}

///102351 names102352 and consumes its single native proc charge. Cast from
///the actual aura owner onto the protected unit, never onto the attacking unit.
pub(crate) fn heal_cenarion_ward(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    _other: Option<Entity>,
) {
    spell_cast::cast_triggered(world, aura.caster, CENARION_HEAL, Some(unit), 1);
}
