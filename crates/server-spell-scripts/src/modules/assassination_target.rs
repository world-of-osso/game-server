//! SimC76f8118 sc_rogue.cpp1718–1732,2347–2357,2489–2499: live target modifiers.
use bevy::prelude::*;
use shared::components::Health;

use super::combat;
use combat::auras::{ActiveAuras, AuraInstance, spell_mods};
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::TargetDamageMultiplier(multiplier)];
const MASTERY: u32 = 76803;
const LETHAL_DOSE: u32 = 381640;
const ZOLDYCK: u32 = 381798;
const ENVENOM: u32 = 32645;
const INTERNAL_BLEEDING: u32 = 381628;
const MUTILATED_FLESH: u32 = 394021;

fn owned(auras: &ActiveAuras, caster: Entity, spell: u32) -> Option<&AuraInstance> {
    auras
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == spell)
}

fn native_amount(aura: &AuraInstance, index: u8) -> f32 {
    aura.effects
        .iter()
        .find(|effect| effect.index == index)
        .expect("selected native Assassination talent retains its sourced effect")
        .amount
}

fn matches_mastery_mask(mastery: &AuraInstance, spell: &SpellInfo) -> bool {
    mastery
        .effects
        .iter()
        .filter(|effect| effect.index < 2)
        .any(|effect| spell_mods::class_mask_affects(mastery, effect, spell))
}

fn matches_mastery_label(mastery: &AuraInstance, spell: &SpellInfo) -> bool {
    mastery
        .effects
        .iter()
        .filter(|effect| matches!(effect.index, 2 | 3))
        .any(|effect| {
            u32::try_from(effect.misc_values[1])
                .is_ok_and(|label| spell.labels.binary_search(&label).is_ok())
        })
}

// sc_rogue.cpp9085–9090: one count per owned ticking bleed/Deadly Poison or
// active poison debuff, never per stack. Kingsbane explicitly ceased counting.
fn count_owned_effects(target: &ActiveAuras, caster: Entity) -> usize {
    const DOTS: &[u32] = &[360194, 703, 381628, 1943, 394021, 2818];
    const DEBUFFS: &[u32] = &[392388, 3409, 5760, 8680, 383414];
    let dots = DOTS
        .iter()
        .filter(|&&spell| {
            owned(target, caster, spell)
                .is_some_and(|aura| aura.effects.iter().any(|effect| effect.period_ms > 0))
        })
        .count();
    let debuffs = DEBUFFS
        .iter()
        .filter(|&&spell| owned(target, caster, spell).is_some())
        .count();
    dots + debuffs
}

fn lethal_dose(auras: &ActiveAuras, caster: Entity, target: Option<&ActiveAuras>) -> f32 {
    let Some(talent) = owned(auras, caster, LETHAL_DOSE) else {
        return 1.0;
    };
    let count = target.map_or(0, |target| count_owned_effects(target, caster));
    1.0 + native_amount(talent, 0) * count as f32 / 100.0
}

fn zoldyck(auras: &ActiveAuras, caster: Entity, health: Option<&Health>) -> f32 {
    let Some(talent) = owned(auras, caster, ZOLDYCK) else {
        return 1.0;
    };
    let Some(health) = health else { return 1.0 };
    let below_threshold = health.current * 100.0 < health.max * native_amount(talent, 1);
    if below_threshold {
        1.0 + native_amount(talent, 0) / 100.0
    } else {
        1.0
    }
}

fn multiplier(world: &World, caster: Entity, target: Entity, spell: &SpellInfo) -> f32 {
    let Some(auras) = world.get::<ActiveAuras>(caster) else {
        return 1.0;
    };
    let Some(mastery) = owned(auras, caster, MASTERY) else {
        return 1.0;
    };
    let masked = matches_mastery_mask(mastery, spell);
    // sc_rogue.cpp4263,6188,6281–6287: explicit Lethal Dose exceptions.
    let dose_eligible =
        spell.id != ENVENOM && (masked || matches!(spell.id, INTERNAL_BLEEDING | MUTILATED_FLESH));
    let dose = if dose_eligible {
        lethal_dose(auras, caster, world.get::<ActiveAuras>(target))
    } else {
        1.0
    };
    let execute = if masked || matches_mastery_label(mastery, spell) {
        zoldyck(auras, caster, world.get::<Health>(target))
    } else {
        1.0
    };
    dose * execute
}
