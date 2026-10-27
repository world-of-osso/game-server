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
];

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
    // SimC76f8118 sc_death_knight.cpp8239–8242/8300–8302: Frost uses effect2,
    // while effect1 is the Blood coefficient.
    if matches!(spell.id, 441424 | 441426)
        && effect == 0
        && world
            .get::<ActiveSpec>(caster)
            .is_some_and(|spec| spec.0 == 251)
    {
        targets.clear();
    }
}
