//! SimC76f8118 sc_monk.cpp5039–5076,6817–6829: alternating class abilities.
use super::combat;
use bevy::prelude::*;
use combat::auras::{
    self, AuraApplication, AuraInstance, AuraSource, RemoveReason, procs::AuraEvent,
    scripts::AuraScripts,
};
use combat::class_progression::TraitLoadout;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_info::SpellInfo;

const TALENT: u32 = 450889;
const MAGIC: u32 = 451508;
const PHYSICAL: u32 = 451514;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(|spell| spell == TALENT),
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::CaptureConsumedDamageModifier(snapshot_chi_burst),
    SpellScriptHook::AfterWeaponLaunch(after_launch),
];

fn register(scripts: &mut AuraScripts) {
    for spell in [MAGIC, PHYSICAL] {
        scripts.0.entry(spell).or_default().check_proc = Some(cast_driven);
    }
}

fn cast_driven(_: &World, _: &AuraInstance, _: &AuraEvent) -> bool {
    false
}

fn snapshot_chi_burst(
    world: &World,
    caster: Entity,
    spell: &mut SpellInfo,
    _: bool,
) -> Option<u32> {
    if spell.id != 148135 {
        return None;
    }
    if let Some(bonus) = world
        .get::<combat::auras::ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == MAGIC && aura.caster == caster)
                .and_then(|aura| {
                    aura.effects
                        .iter()
                        .find(|effect| effect.index == 0)
                        .map(|effect| effect.amount * f32::from(aura.stacks))
                })
        })
    {
        spell.cast_damage_multiplier =
            Some(spell.cast_damage_multiplier.unwrap_or(1.0) * (1.0 + bonus / 100.0));
    }
    None
}

fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if !world.get::<TraitLoadout>(caster).is_some_and(|traits| {
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == TALENT)
    }) {
        return;
    }
    // SimC sc_monk.cpp2992–3020: Chi Burst's damage child consumes magic stacks;
    // the foreground parent grants the opposite buff after its children execute.
    if spell.id == 148135 {
        consume(world, caster, MAGIC);
        return;
    }
    if depth != 0 && spell.id != 331433 {
        return;
    }
    let (consumed, grant) = match spell.id {
        205523 | 121253 | 116847 | 101546 | 100780 | 331433 => (PHYSICAL, MAGIC),
        115181 | 117952 | 115072 | 325153 | 115175 | 116670 => (MAGIC, PHYSICAL),
        123986 => (0, PHYSICAL),
        _ => return,
    };
    consume(world, caster, consumed);
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target: caster,
            spell_id: grant,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn consume(world: &mut World, caster: Entity, spell: u32) {
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.spell_id == spell && aura.caster == caster
    });
}
