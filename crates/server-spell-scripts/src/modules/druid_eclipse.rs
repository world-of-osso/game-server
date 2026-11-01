//! SimC76f8118 sc_druid.cpp:3210–3290, 7183–7262, 8555–8561, 9117–9125.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::auras::{self, AuraInstance, RemoveReason, procs::AuraEvent, scripts::AuraScripts};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

const ECLIPSE: u32 = 1239669;
const SOLAR_ACTION: u32 = 1233346;
const LUNAR_ACTION: u32 = 1233272;
const SOLAR: u32 = 48517;
const LUNAR: u32 = 48518;
const OVERRIDE: u32 = 1233353;
const LUNAR_CALLING: u32 = 429523;
const SOLSTICE: u32 = 343647;
const SOLSTICE_BUFF: u32 = 343648;
const TOTAL_ECLIPSE: u32 = 1240206;
const ASCENDANT_ECLIPSES: u32 = 1261564;
const ASCENDANT_FIRES: u32 = 1263363;
/// Celestial Alignment and both Incarnation: Chosen of Elune rows.
const ALIGNMENTS: [u32; 3] = [194223, 102560, 390414];

/// Successful casts of these apply a Solar or Lunar Eclipse (`after_cast`).
pub(super) fn applies_eclipse(spell: u32) -> bool {
    matches!(spell, SOLAR_ACTION | LUNAR_ACTION) || ALIGNMENTS.contains(&spell)
}

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::ActionAvailable(action_available),
    SpellScriptHook::SuccessfulCastRelationships(after_cast),
];

fn selected(world: &World, caster: Entity, spell: u32) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 102)
        && world
            .get::<KnownSpells>(caster)
            .is_some_and(|known| known.contains(spell))
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.entry(SOLSTICE).or_default().check_proc = Some(block_generated_proc);
    scripts.0.entry(ASCENDANT_ECLIPSES).or_default().check_proc = Some(block_generated_proc);
    scripts.0.entry(LUNAR_CALLING).or_default().on_apply = Some(start_lunar_calling);
}

fn block_generated_proc(_: &World, _: &AuraInstance, _: &AuraEvent) -> bool {
    false
}

fn start_lunar_calling(world: &mut World, caster: Entity, _: &AuraInstance) {
    if selected(world, caster, ECLIPSE) {
        spell_cast::cast_triggered(world, caster, OVERRIDE, Some(caster), 1);
    }
}

fn action_available(world: &World, caster: Entity, spell: u32) -> bool {
    if !matches!(spell, SOLAR_ACTION | LUNAR_ACTION) || !selected(world, caster, ECLIPSE) {
        return true;
    }
    let lunar = auras::has_aura(world, caster, OVERRIDE);
    !auras::has_aura(world, caster, if lunar { LUNAR } else { SOLAR })
}

fn after_cast(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if !selected(world, caster, ECLIPSE) {
        return;
    }
    match spell.id {
        194153 => {
            consume_ascendant_fires(world, caster);
            spell_cast::cast_triggered(world, caster, OVERRIDE, Some(caster), 1);
        }
        190984 => {
            consume_ascendant_fires(world, caster);
            if !selected(world, caster, LUNAR_CALLING) {
                auras::remove_auras(world, caster, RemoveReason::Cancelled, |aura| {
                    aura.spell_id == OVERRIDE && aura.caster == caster
                });
            }
        }
        SOLAR_ACTION | LUNAR_ACTION => enter_eclipse(world, caster),
        id if ALIGNMENTS.contains(&id) => {
            let duration = spell_cast::duration_ms(world, caster, spell);
            assert!(
                duration > 0,
                "Celestial Alignment must have a native duration"
            );
            apply_eclipse(world, caster, SOLAR, Some(duration as u32));
            apply_eclipse(world, caster, LUNAR, Some(duration as u32));
            trigger_eclipse_benefits(world, caster);
        }
        _ => {}
    }
}

fn enter_eclipse(world: &mut World, caster: Entity) {
    let lunar = auras::has_aura(world, caster, OVERRIDE);
    let (current, opposite) = if lunar {
        (LUNAR, SOLAR)
    } else {
        (SOLAR, LUNAR)
    };
    auras::remove_auras(world, caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == opposite && aura.caster == caster
    });
    apply_eclipse(world, caster, current, None);
    if selected(world, caster, TOTAL_ECLIPSE) {
        let chance = spell_cast::scripts::script_effect_value(world, caster, TOTAL_ECLIPSE, 0);
        if combat::random::random::<f32>() * 100.0 < chance {
            apply_eclipse(world, caster, opposite, None);
        }
    }
    trigger_eclipse_benefits(world, caster);
}

fn consume_ascendant_fires(world: &mut World, caster: Entity) {
    auras::remove_auras(world, caster, RemoveReason::Cancelled, |aura| {
        aura.spell_id == ASCENDANT_FIRES && aura.caster == caster
    });
}

fn apply_eclipse(world: &mut World, caster: Entity, spell: u32, duration: Option<u32>) {
    spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        spell,
        Some(caster),
        1,
        spell_cast::TriggeredSpellOverrides {
            fixed_aura_duration_ms: duration,
            ..Default::default()
        },
    );
    super::druid_balance_of_all_things::on_eclipse(world, caster, spell);
    super::druid_sundered_firmament::on_eclipse(world, caster);
}

fn trigger_eclipse_benefits(world: &mut World, caster: Entity) {
    super::druid_ascendant_stars::on_eclipse(world, caster);
    if selected(world, caster, ASCENDANT_ECLIPSES) {
        spell_cast::cast_triggered(world, caster, ASCENDANT_FIRES, Some(caster), 1);
    }
    if selected(world, caster, SOLSTICE) {
        let amount = spell_cast::scripts::script_effect_value(world, caster, SOLSTICE, 0);
        spell_cast::cast_triggered_with_effect0_amount(
            world,
            caster,
            SOLSTICE_BUFF,
            Some(caster),
            1,
            amount,
        );
    }
}
