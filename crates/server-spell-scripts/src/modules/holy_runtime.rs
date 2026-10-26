//! Native Holy Words, Surge of Light and Prayer of Mending.
use super::combat;
use bevy::prelude::*;
use combat::auras::scripts::AuraScripts;
use combat::spell_cast::{script_registry::{SpellScriptHook, TargetPlanHook}, targets::Relation};
use combat::spell_cast::scripts::{AfterCastHook, AreaTargetFilter, reduce_cooldown, script_effect_value};
use combat::spell_info::{SpellInfo, SpellStore};

#[path = "holy_runtime/holy_priest.rs"]
mod holy_priest;
#[path = "holy_runtime/prayer_of_mending.rs"]
mod prayer_of_mending;
#[path = "holy_runtime/prayer_healing.rs"]
mod prayer_healing;

const FLASH_HEAL: u32 = 2061;
const PRAYER_OF_HEALING: u32 = 596;
const SERENITY: u32 = 2050;
const SANCTIFY: u32 = 34861;

pub const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::SupportAuraRegistration(register_auras),
    SpellScriptHook::NamedAuraFields(&[(109186, 0), (200183, 0), (390994, 1), (prayer_of_mending::WARD, 0)]),
    SpellScriptHook::NativeAfterCast(after_cast),
    SpellScriptHook::EffectHandler(prayer_of_mending::effect_handler),
    SpellScriptHook::AreaFilter(prayer_of_mending::area_filter),
    SpellScriptHook::TargetPlan(target_plan),
];

fn register_auras(scripts: &mut AuraScripts) {
    holy_priest::register(scripts);
    prayer_of_mending::register(scripts);
}

fn after_cast(spell: u32) -> Option<AfterCastHook> {
    match spell {
        holy_priest::APOTHEOSIS => Some(holy_priest::restore_holy_words),
        prayer_of_mending::PRAYER => Some(holy_priest::reduce_word_on_mending),
        FLASH_HEAL => Some(flash_heal),
        PRAYER_OF_HEALING => Some(prayer),
        _ => None,
    }
}

fn target_plan(spell: u32) -> Option<(Relation, TargetPlanHook)> {
    (spell == PRAYER_OF_HEALING).then_some((Relation::Ally, prayer_healing::prayer_plan))
}

/// 2050s2 is the number of seconds removed when Flash Heal is cast.
fn flash_heal(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    if world.resource::<SpellStore>().get(SERENITY).is_some() {
        let seconds = script_effect_value(world, caster, SERENITY, 1);
        let seconds = holy_priest::holy_word_reduction(world, caster, seconds);
        reduce_cooldown(world, caster, SERENITY, seconds);
    }
    holy_priest::spend_surge(world, caster);
}

/// 34861s3: Prayer of Healing reduces an existing recharge by six seconds.
fn prayer(world: &mut World, caster: Entity, _spell: &SpellInfo) {
    let (word, index) = if combat::auras::has_aura(world, caster, 1246517) {
        (SERENITY, 1)
    } else {
        (SANCTIFY, 2)
    };
    if world.resource::<SpellStore>().get(word).is_some() {
        let seconds = script_effect_value(world, caster, word, index);
        let seconds = holy_priest::holy_word_reduction(world, caster, seconds);
        reduce_cooldown(world, caster, word, seconds);
    }
}
