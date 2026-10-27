//! Frost Echoing Fury grants an Exterminate use from an actual Reaper's Mark.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use super::combat;
use combat::auras::{self, ActiveAuras};
use combat::spell_cast::effects::EffectCtx;
use combat::spell_cast::queued_triggers;
use combat::spell_cast::script_registry::SpellScriptHook;
use combat::spell_cast::scripts::script_effect_value;
use combat::spell_cast::{self, TriggeredSpellOverrides};
use combat::spell_info::SpellInfo;

const REAPERS_MARK: u32 = 439843;
const ECHOING_FURY: u32 = 1265855;
const EXTERMINATE: u32 = 441416;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::OwnsPolicyProc(|spell| spell == EXTERMINATE),
    SpellScriptHook::AfterElementalLaunch(after_launch),
    SpellScriptHook::AfterResolvedHits(after_hits),
    SpellScriptHook::TargetFilter(filter_damage_effect),
    SpellScriptHook::LateAuraRegistration(register_mark),
    SpellScriptHook::ResolvedDamage(stack_mark),
];

fn register_mark(scripts: &mut auras::scripts::AuraScripts) {
    scripts.0.entry(434765).or_default().on_remove = Some(explode_mark);
}

fn stack_mark(world: &mut World, ctx: &EffectCtx, target: Entity, critical: bool, damage: f32) {
    if damage <= 0.0
        || matches!(ctx.spell.id, 439843 | 436304 | 66198 | 458169 | 439539)
        || !world
            .get::<ActiveSpec>(ctx.caster)
            .is_some_and(|spec| spec.0 == 251)
    {
        return;
    }
    let school = ctx.spell.school_mask();
    let shadowfrost = school == 48
        || (auras::has_aura(world, ctx.caster, 443586)
            && (school & 48 != 0 || ctx.spell.id == 49998));
    let amount = if shadowfrost {
        if auras::has_aura(world, ctx.caster, 440031) {
            if critical { 4 } else { 2 }
        } else {
            1
        }
    } else if school == 16 || school == 32 {
        1
    } else {
        return;
    };
    let mark = world.get::<ActiveAuras>(target).and_then(|active| {
        active
            .iter()
            .find(|aura| aura.spell_id == 434765 && aura.caster == ctx.caster)
            .map(|aura| (aura.stacks, aura.max_stacks))
    });
    let Some((stacks, maximum)) = mark else {
        return;
    };
    let next = stacks.saturating_add(amount).min(maximum);
    auras::set_owned_stacks(world, ctx.caster, target, 434765, next);
    if next == maximum {
        auras::remove_auras(world, target, auras::RemoveReason::Consumed, |aura| {
            aura.spell_id == 434765 && aura.caster == ctx.caster
        });
    }
}

fn explode_mark(
    world: &mut World,
    target: Entity,
    aura: &auras::AuraInstance,
    reason: auras::RemoveReason,
) {
    if !matches!(
        reason,
        auras::RemoveReason::Expired
            | auras::RemoveReason::Consumed
            | auras::RemoveReason::Cancelled
    ) || !world
        .get::<ActiveSpec>(aura.caster)
        .is_some_and(|spec| spec.0 == 251)
    {
        return;
    }
    let Some(mut explosion) = spell_cast::load_triggered_spell(world, 436304, 1) else {
        return;
    };
    // SimC76f8118 sc_death_knight.cpp8355–8402: Frost uses effect2,
    // multiplied by actual earned mark stacks, and selected Exterminate grants effect3 uses.
    explosion.effects.retain(|effect| effect.index == 1);
    explosion.cast_damage_multiplier = Some(f32::from(aura.stacks));
    if !spell_cast::launch_triggered_spell(world, aura.caster, &explosion, Some(target), 1) {
        return;
    }
    if auras::has_aura(world, aura.caster, 441378) {
        let stacks = script_effect_value(world, aura.caster, 441378, 2) as u8;
        spell_cast::cast_triggered_with_overrides(
            world,
            aura.caster,
            EXTERMINATE,
            Some(aura.caster),
            1,
            TriggeredSpellOverrides {
                aura_stacks: Some(stacks),
                ..Default::default()
            },
        );
    }
}

fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo, foreground: bool) {
    if !foreground
        || !world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 251)
    {
        return;
    }
    if spell.id == REAPERS_MARK && auras::has_aura(world, caster, ECHOING_FURY) {
        // SimC76f8118 sc_death_knight.cpp8530–8537/8591–8595: an actual
        // Mark grants the selected Echoing Fury's authored Exterminate uses.
        let stacks = script_effect_value(world, caster, ECHOING_FURY, 0) as u8;
        assert!(
            spell_cast::cast_triggered_with_overrides(
                world,
                caster,
                EXTERMINATE,
                Some(caster),
                1,
                TriggeredSpellOverrides {
                    aura_stacks: Some(stacks),
                    ..Default::default()
                }
            ),
            "native Echoing Fury Exterminate aura refused"
        );
    } else if matches!(spell.id, 207230 | 49020) {
        let instance = world.get::<ActiveAuras>(caster).and_then(|active| {
            active
                .iter()
                .find(|aura| aura.spell_id == EXTERMINATE && aura.caster == caster)
                .map(|aura| aura.instance_id)
        });
        if let Some(instance) = instance {
            // sc_death_knight.cpp10601–10605/11909: each real spender uses one stack.
            auras::spend_stack(world, caster, instance);
            let target = spell
                .cast_primary_target
                .expect("Exterminate spender target");
            let ctx = EffectCtx {
                caster,
                spell,
                depth: 0,
                dest: None,
                chain_multiplier: 1.0,
            };
            queued_triggers::schedule_full_trigger(world, &ctx, target, 441424, 500);
            queued_triggers::schedule_full_trigger(world, &ctx, target, 441426, 1000);
        }
    }
}

fn after_hits(world: &mut World, ctx: &EffectCtx) {
    if ctx.spell.id == 441424
        && world
            .get::<ActiveSpec>(ctx.caster)
            .is_some_and(|spec| spec.0 == 251)
    {
        spell_cast::cast_triggered(world, ctx.caster, 51124, Some(ctx.caster), ctx.depth + 1);
    }
}

fn filter_damage_effect(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
    effect: u8,
    _primary: Option<Entity>,
    targets: &mut Vec<Entity>,
) {
    // SimC76f8118 sc_death_knight.cpp8239–8242/8300–8302/8516–8517:
    // Frost uses effect2 for Exterminate and effect4 for Mark, not Blood's effect1.
    if matches!(spell.id, 439843 | 441424 | 441426)
        && effect == 0
        && world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 251)
    {
        targets.clear();
    }
}
