//! Grove Guardians summons a treant after Swiftmend or Wild Growth.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{
    self,
    script_registry::{AreaTargetFilter, SpellScriptHook},
    scripts::AfterCastHook,
};
use combat::spell_info::SpellInfo;
use shared::components::Health;

const TALENT: u32 = 1_226_140;
const SUMMON: u32 = 102_693;
const NOURISH: u32 = 422_090;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::AreaFilter(area_filter),
    SpellScriptHook::NamedHealingMultiplier(restoration_harmony),
];

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    matches!(spell, 18_562 | 48_438).then_some(summon_treant)
}

fn summon_treant(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !combat::auras::has_aura(world, caster, TALENT) {
        return;
    }
    let Some(target) = spell.cast_primary_target else {
        warn!(
            ?caster,
            spell = spell.id,
            "Grove Guardians cast lacks its primary target"
        );
        return;
    };
    if !spell_cast::try_cast_triggered(world, caster, SUMMON, Some(target), 1) {
        warn!(?caster, ?target, "Grove Guardians summon failed");
    }
}

fn restoration_harmony(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    // Percent-health and other modifier-immune heals must not gain mastery.
    if spell.misc.as_ref().is_some_and(|misc| misc.attributes[6] & 0x0800_0000 != 0) {
        return 1.0;
    }
    let guardian = if spell.id == NOURISH {
        world.get::<combat::spell_cast::summon::SummonedCreature>(caster)
            .filter(|summon| summon.spell_id == SUMMON)
            .map(|summon| summon.owner)
    } else {
        None
    };
    let owner = guardian.unwrap_or(caster);
    if !combat::auras::has_aura(world, owner, 77495)
        || spell.class_options.as_ref().is_none_or(|class| class.spell_class_set != 7)
    {
        return 1.0;
    }
    let Some(stats) = world.get::<combat::stats::CharacterStats>(owner) else {
        return 1.0;
    };
    let Some(auras) = world.get::<combat::auras::ActiveAuras>(target) else {
        return 1.0;
    };
    // SimC76f8118 sc_druid.cpp:13896–13913 counts these owned HoTs for Harmony.
    let hots = [200389, 155777, 8936, 774, 48438]
        .into_iter()
        .filter(|spell| {
            auras
                .iter()
                .any(|aura| aura.caster == owner && aura.spell_id == *spell)
        })
        .count() as f32;
    let lifebloom = auras
        .iter()
        .any(|aura| aura.caster == owner && aura.spell_id == 33763);
    let bonus_blooms = if combat::auras::has_aura(world, owner, 392256) {
        combat::spell_cast::scripts::script_effect_value(world, owner, 392256, 0)
    } else {
        0.0
    };
    let count = hots + if lifebloom { 1.0 + bonus_blooms } else { 0.0 };
    // Spell422090 effect1 grants the guardian 100% of its owner's Harmony.
    let effectiveness = if guardian.is_some() {
        combat::spell_cast::scripts::script_effect_value(world, owner, NOURISH, 1) / 100.0
    } else {
        1.0
    };
    1.0 + count * stats.mastery_pct / 100.0 * effectiveness
}

fn area_filter(spell: u32, effect: u8) -> Option<AreaTargetFilter> {
    (spell == NOURISH && effect == 0).then_some(select_nourish_target)
}

fn select_nourish_target(
    world: &World,
    spell: &SpellInfo,
    _: Option<Entity>,
    units: &mut Vec<Entity>,
) {
    // Native 70448 Spell1226140: heal the triggering target or a nearby ally.
    if let Some(primary) = spell.cast_primary_target
        && units.contains(&primary)
    {
        units.retain(|unit| *unit == primary);
        return;
    }
    units.sort_by(|a, b| {
        let fraction = |unit| {
            world
                .get::<Health>(unit)
                .map_or(1.0, |health| health.current / health.max.max(1.0))
        };
        fraction(*a).total_cmp(&fraction(*b))
    });
    units.truncate(1);
}
