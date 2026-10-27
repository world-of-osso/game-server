//! SimC76f8118 sc_priest_shadow.cpp1187/1886–1926/2769–2846: owned target counter.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, AuraApplication, AuraSource, RemoveReason};
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;

const TALENT: u32 = 373280;
const COUNTER: u32 = 1243069;
const HORRIFIC_DAMAGE: u32 = 1243105;
const VISION_DAMAGE: u32 = 1243106;
const HORRIFIC_BUFF: u32 = 1243113;
const VISION_BUFF: u32 = 1243114;
const MADNESS: u32 = 335467;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterElementalLaunch(after_launch),
    SpellScriptHook::PeriodicPrimaryDamage(after_tick),
    SpellScriptHook::AsynchronousAuras(&[HORRIFIC_BUFF, VISION_BUFF]),
];

fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo, foreground: bool) {
    // SimC76f8118 priest.cpp198 and shadow.cpp576/775/1187.
    let stacks = match spell.id {
        MADNESS => 12,
        8092 => 6,
        589 if foreground => 3,
        34914 if foreground => 4,
        _ => return,
    };
    if let Some(target) = spell.cast_executed_target {
        add_counter(world, caster, target, stacks);
    }
}

fn after_tick(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: f32, _: bool) {
    let stacks = match ctx.spell.id {
        15407 | 391403 => 1,
        // SimC76f8118 sc_priest_shadow.cpp1436: each Torrent tick contributes two.
        263165 => 2,
        _ => return,
    };
    add_counter(world, ctx.caster, target, stacks);
}

fn apply_owned(world: &mut World, caster: Entity, target: Entity, spell_id: u32) {
    auras::apply_aura(
        world,
        &AuraApplication {
            caster,
            target,
            spell_id,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
}

fn add_counter(world: &mut World, caster: Entity, target: Entity, amount: u8) {
    if !auras::has_aura(world, caster, TALENT) {
        return;
    }
    let previous = world
        .get::<ActiveAuras>(target)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == COUNTER && aura.caster == caster)
        })
        .map_or(0, |aura| aura.stacks);
    let total = u16::from(previous) + u16::from(amount);
    let horrific = spell_cast::scripts::script_effect_value(world, caster, TALENT, 0) as u16;
    let vision = spell_cast::scripts::script_effect_value(world, caster, TALENT, 1) as u16;
    if previous > 0 && u16::from(previous) < horrific && total >= horrific {
        spell_cast::cast_triggered(world, caster, HORRIFIC_DAMAGE, Some(target), 1);
        apply_owned(world, caster, caster, HORRIFIC_BUFF);
    } else if previous > 0 && total >= vision {
        auras::remove_auras(world, target, RemoveReason::Consumed, |aura| {
            aura.spell_id == COUNTER && aura.caster == caster
        });
        spell_cast::cast_triggered(world, caster, VISION_DAMAGE, Some(target), 1);
        apply_owned(world, caster, caster, VISION_BUFF);
    }
    let remaining = if total >= vision {
        total - vision
    } else {
        total
    };
    if remaining > 0 {
        apply_owned(world, caster, target, COUNTER);
        auras::set_owned_stacks(world, caster, target, COUNTER, remaining as u8);
    }
}
