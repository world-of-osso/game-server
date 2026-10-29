//! Bloodied Blade's parry-earned Strength and strike.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, RemoveReason, procs::AuraEvent, scripts::AuraScripts};
use combat::spell_cast::{self, TriggeredSpellOverrides, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;

const TALENT: u32 = 458753;
const STACKS: u32 = 460499;
const FINAL: u32 = 460500;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::AvoidanceProcHitMask(avoidance_mask),
];

fn register(scripts: &mut AuraScripts) {
    let blade = scripts.0.entry(TALENT).or_default();
    blade.check_proc = Some(check_parry);
    blade.on_proc = Some(on_parry);
}

fn avoidance_mask(spell: u32) -> Option<u32> {
    (spell == TALENT).then_some(combat::auras::procs::PROC_HIT_PARRY)
}

fn check_parry(_: &World, _: &AuraInstance, event: &AuraEvent) -> bool {
    event.hit_mask & combat::auras::procs::PROC_HIT_PARRY != 0
}

// SimC76f8118 sc_death_knight.cpp:15990–16000,16685–16696: each
// eligible parry gains one tenth of the native stack amount; the parry
// after maximum stacks replaces them with the final buff and a strike.
fn on_parry(world: &mut World, caster: Entity, _: &AuraInstance, _: Option<Entity>) {
    let stacks = world.get::<auras::ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == STACKS && aura.caster == caster)
            .map(|aura| (aura.stacks, aura.max_stacks))
    });
    if stacks.is_some_and(|(count, max)| max > 0 && count >= max) {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == STACKS && aura.caster == caster
        });
        spell_cast::cast_triggered(world, caster, FINAL, Some(caster), 1);
        if let Some(target) = world
            .get::<combat::networking::CombatTarget>(caster)
            .and_then(|target| target.0)
        {
            strike(world, caster, target);
        } else {
            warn!(?caster, "Bloodied Blade has no selected target");
        }
    } else if !auras::has_aura(world, caster, FINAL) {
        let amount = world
            .resource::<SpellStore>()
            .get(STACKS)
            .expect("Bloodied Blade stacks")
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("Bloodied Blade Strength")
            .base_points
            * 0.1;
        spell_cast::cast_triggered_with_overrides(
            world,
            caster,
            STACKS,
            Some(caster),
            1,
            TriggeredSpellOverrides {
                effect0_amount: Some(amount),
                ..Default::default()
            },
        );
    }
}

// SimC76f8118 sc_death_knight.cpp:11426–11447: replace the free Heart
// Strike with a rune-costing Vampiric Strike when the replacement is active.
fn strike(world: &mut World, caster: Entity, target: Entity) {
    if auras::has_aura(world, caster, 433899) {
        if spell_cast::powers::power_value(world, caster, shared::components::PowerType::Runes)
            .is_none_or(|(runes, _)| runes < 1.0)
        {
            return;
        }
        let cast = spell_cast::cast_costed_trigger_with_context(
            world,
            caster,
            433895,
            Some(target),
            1,
            None,
            None,
        );
        if cast && !auras::has_aura(world, caster, 434153) {
            auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
                aura.spell_id == 433899 && aura.caster == caster
            });
        }
    } else {
        spell_cast::cast_triggered(world, caster, 460501, Some(target), 1);
    }
}
