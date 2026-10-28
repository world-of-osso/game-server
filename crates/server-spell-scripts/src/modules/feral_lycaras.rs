//! Selected Lycara's Teachings follows Cat Form.
use bevy::prelude::*;

use super::combat;
use combat::auras::{ActiveAuras, AuraInstance, RemoveReason, scripts::AuraScripts};
use combat::spell_cast::{self, script_registry::SpellScriptHook};

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EarlyAuraRegistration(register)];
const CAT_FORM: u32 = 768;
const TEACHINGS: u32 = 378988;
const CAT_CRIT: u32 = 378990;
const CAT_PASSIVE: u32 = 106840;
const HUNT: u32 = 439868;

fn register(scripts: &mut AuraScripts) {
    let form = scripts.0.entry(CAT_FORM).or_default();
    form.on_apply = Some(apply_form);
    form.on_remove = Some(remove_form);
    let talent = scripts.0.entry(TEACHINGS).or_default();
    talent.on_apply = Some(apply_talent);
    talent.on_remove = Some(remove_crit);
    let hunt = scripts.0.entry(HUNT).or_default();
    hunt.on_apply = Some(refresh_hunt);
    hunt.on_remove = Some(remove_hunt);
}

fn apply_form(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster == unit {
        apply_crit(world, unit);
        apply_cat_passive(world, unit);
    }
}

fn apply_talent(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster == unit && combat::auras::has_aura(world, unit, CAT_FORM) {
        apply_crit(world, unit);
    }
}

// SimC76f8118 sc_druid.cpp3628–3636/14344–14346: Cat Form grants
// the selected talent's rank-scaled crit amount through its native child.
fn apply_crit(world: &mut World, unit: Entity) {
    let amount = world.get::<ActiveAuras>(unit).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == TEACHINGS && aura.caster == unit)
            .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
            .map(|effect| effect.amount)
    });
    if let Some(amount) = amount {
        spell_cast::cast_triggered_with_overrides(
            world,
            unit,
            CAT_CRIT,
            Some(unit),
            1,
            spell_cast::TriggeredSpellOverrides {
                effect0_amount: Some(amount),
                effect_mask: Some(1),
                ..Default::default()
            },
        );
    }
}

// SimC76f8118 sc_druid.cpp14175: Cat Form passive's native spell masks
// carry the selected Hunt Beneath the Open Skies point modifiers.
fn apply_cat_passive(world: &mut World, unit: Entity) {
    spell_cast::cast_triggered_with_overrides(
        world,
        unit,
        CAT_PASSIVE,
        Some(unit),
        1,
        spell_cast::TriggeredSpellOverrides {
            effect_mask: Some(12),
            ..Default::default()
        },
    );
}

fn refresh_hunt(world: &mut World, unit: Entity, aura: &AuraInstance) {
    if aura.caster == unit && combat::auras::has_aura(world, unit, CAT_FORM) {
        remove_cat_passive(world, unit);
        apply_cat_passive(world, unit);
    }
}

fn remove_hunt(world: &mut World, unit: Entity, aura: &AuraInstance, _: RemoveReason) {
    refresh_hunt(world, unit, aura);
}

fn remove_cat_passive(world: &mut World, unit: Entity) {
    combat::auras::remove_auras(world, unit, RemoveReason::Cancelled, |child| {
        child.spell_id == CAT_PASSIVE && child.caster == unit
    });
}

fn remove_crit(world: &mut World, unit: Entity, aura: &AuraInstance, _: RemoveReason) {
    if aura.caster == unit {
        combat::auras::remove_auras(world, unit, RemoveReason::Cancelled, |child| {
            child.spell_id == CAT_CRIT && child.caster == unit
        });
    }
}

fn remove_form(world: &mut World, unit: Entity, aura: &AuraInstance, reason: RemoveReason) {
    remove_crit(world, unit, aura, reason);
    if aura.caster == unit {
        remove_cat_passive(world, unit);
    }
}
