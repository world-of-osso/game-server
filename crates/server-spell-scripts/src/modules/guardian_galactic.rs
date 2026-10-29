//! SimC76f8118 sc_druid.cpp7871–7947,12694–12749.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, AuraInstance, RemoveReason, procs::AuraEvent, scripts::AuraScripts};
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellStore;

const TALENT: u32 = 203964;
const BUFF: u32 = 213708;
const MOON_GUARDIAN: u32 = 429520;
const MOONFIRE: u32 = 8921;
const MOONFIRE_DAMAGE: u32 = 164812;

pub(super) const HOOKS: &[SpellScriptHook] = &[SpellScriptHook::EarlyAuraRegistration(register)];

fn register(scripts: &mut AuraScripts) {
    let script = scripts.0.entry(TALENT).or_default();
    script.check_proc = Some(roll_galactic);
    script.on_damage_proc = Some(trigger_galactic);
}

fn roll_galactic(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    if world
        .get::<combat::class_progression::TraitLoadout>(aura.caster)
        .is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == 1252871)
        })
    {
        return false;
    }
    if event.actor != aura.caster
        || event.damage <= 0.0
        || !event.can_proc
        || event.target.is_none()
        || matches!(event.spell_id, Some(MOONFIRE | MOONFIRE_DAMAGE))
    {
        return false;
    }
    let mask = world
        .resource::<SpellStore>()
        .get(TALENT)
        .and_then(|spell| spell.aura_options.as_ref())
        .expect("Galactic Guardian native proc mask")
        .proc_type_mask[0];
    if event.actor_mask[0] & mask == 0 {
        return false;
    }
    let chance = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Galactic Guardian native chance")
        .amount;
    combat::random::random::<f32>() < chance / 100.0
}

fn trigger_galactic(world: &mut World, caster: Entity, _: &AuraInstance, event: &AuraEvent) {
    let Some(target) = event.target else {
        return;
    };
    let empowered = auras::has_aura(world, caster, BUFF);
    let can_energize = empowered && auras::has_aura(world, caster, MOON_GUARDIAN);
    spell_cast::cast_triggered_with_overrides(
        world,
        caster,
        MOONFIRE,
        Some(target),
        1,
        spell_cast::TriggeredSpellOverrides {
            effect_mask: Some(if can_energize { 3 } else { 1 }),
            ..Default::default()
        },
    );
    if empowered {
        auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
            aura.spell_id == BUFF && aura.caster == caster
        });
    }
    spell_cast::cast_triggered(world, caster, BUFF, Some(caster), 1);
}
