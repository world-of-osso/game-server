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
    SpellScriptHook::DeclaredReach(declared_reach),
    SpellScriptHook::OwnsPolicyProc(|spell| spell == EXTERMINATE),
    SpellScriptHook::AfterElementalLaunch(after_launch),
    SpellScriptHook::AfterResolvedHits(after_hits),
    SpellScriptHook::TargetFilter(filter_damage_effect),
    SpellScriptHook::LateAuraRegistration(register_mark),
    SpellScriptHook::ResolvedDamage(stack_mark),
    SpellScriptHook::CriticalChance(wave_crit_chance),
];

fn declared_reach(
    world: &World,
    caster: Entity,
    spell: &SpellInfo,
) -> combat::spell_cast::script_registry::ScriptReach {
    use combat::spell_cast::script_registry::{DeclaredReach, ScriptReach, reach_source_selected};
    let store = world.resource::<combat::spell_info::SpellStore>();
    let wave =
        matches!(spell.id, REAPERS_MARK | 435802) && reach_source_selected(world, caster, 439851);
    let rupture =
        matches!(spell.id, REAPERS_MARK | 436304) && reach_source_selected(world, caster, 437161);
    if !wave && !rupture {
        return ScriptReach::None;
    }
    let caster_radius = if wave {
        store
            .get(435802)
            .and_then(|spell| spell.misc.as_ref())
            .and_then(|misc| misc.range)
            .expect("native Wave of Souls range")
            .max_hostile
    } else {
        0.0
    };
    let target_radius = if rupture {
        store
            .get(439594)
            .expect("native Soul Rupture")
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .expect("native Soul Rupture effect")
            .radius[1]
            .expect("native Soul Rupture radius")
            .radius
    } else {
        0.0
    };
    ScriptReach::Area(DeclaredReach {
        caster_radius,
        target_radius,
    })
}

fn register_mark(scripts: &mut auras::scripts::AuraScripts) {
    scripts.0.entry(434765).or_default().on_remove = Some(explode_mark);
}

fn stack_mark(world: &mut World, ctx: &EffectCtx, target: Entity, critical: bool, damage: f32) {
    if damage <= 0.0
        || !world
            .get::<ActiveSpec>(ctx.caster)
            .is_some_and(|spec| spec.0 == 251)
    {
        return;
    }
    if ctx.spell.id == 435802 && critical {
        let primary = world
            .get::<WaveState>(ctx.caster)
            .is_some_and(|state| state.primary == target);
        let modifier = if primary && auras::has_aura(world, ctx.caster, 443560) {
            1.0 + script_effect_value(world, ctx.caster, 443560, 1) / 100.0
        } else {
            1.0
        };
        let value = script_effect_value(world, ctx.caster, 439851, 0) * modifier;
        spell_cast::cast_triggered_with_effect0_amount(
            world,
            ctx.caster,
            443404,
            Some(target),
            ctx.depth + 1,
            value,
        );
    }
    if ctx.spell.id == 436304 {
        trigger_soul_rupture(world, ctx, target, damage);
        return;
    }
    if ctx.spell.id == 439843 {
        if auras::has_aura(world, ctx.caster, 434905) {
            // SimC76f8118 sc_death_knight.cpp8550–8554: Grim Reaper's
            // actual Mark impact grants a predictable Killing Machine stack.
            spell_cast::cast_triggered(world, ctx.caster, 51124, Some(ctx.caster), ctx.depth + 1);
            #[cfg(any(test, feature = "test-support"))]
            combat::simc_apl_native::reaction::predict(world, ctx.caster, 51124);
        }
        return;
    }
    if ctx.spell.id == 441426 && auras::has_aura(world, ctx.caster, 441894) {
        spell_cast::cast_triggered(world, ctx.caster, 55095, Some(target), ctx.depth + 1);
    }
    if matches!(ctx.spell.id, 436304 | 66198 | 458169 | 439539) {
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

fn trigger_soul_rupture(world: &mut World, ctx: &EffectCtx, target: Entity, damage: f32) {
    if !auras::has_aura(world, ctx.caster, 437161) {
        return;
    }
    let Some(mut rupture) = spell_cast::load_triggered_spell(world, 439594, ctx.depth + 1) else {
        return;
    };
    let percent = script_effect_value(world, ctx.caster, 437161, 1);
    let effect = rupture
        .effects
        .iter_mut()
        .find(|effect| effect.index == 0)
        .expect("Soul Rupture damage");
    effect.base_points = damage * percent / 100.0;
    let radius = effect.radius[1]
        .as_ref()
        .expect("Soul Rupture area radius")
        .radius;
    let center =
        combat::spell_cast::targets::position(world, target).expect("Mark target position");
    let targets = combat::spell_cast::targets::area_units(
        world,
        ctx.caster,
        center,
        radius,
        combat::spell_cast::targets::Relation::Enemy,
        0,
    );
    if targets.iter().all(|candidate| *candidate == target)
        && auras::has_aura(world, ctx.caster, 443560)
    {
        spell_cast::cast_triggered(world, ctx.caster, 469169, Some(ctx.caster), ctx.depth + 1);
    }
    spell_cast::launch_triggered_spell(world, ctx.caster, &rupture, Some(target), ctx.depth + 1);
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
    let grim = if auras::has_aura(world, aura.caster, 434905) {
        let health = world
            .get::<shared::components::Health>(target)
            .expect("Mark target health");
        let missing = (1.0 - health.current / health.max).clamp(0.0, 1.0);
        let maximum = script_effect_value(world, aura.caster, 434905, 0) / 100.0;
        1.0 + missing * maximum
    } else {
        1.0
    };
    explosion.cast_damage_multiplier = Some(f32::from(aura.stacks) * grim);
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
    if spell.id == REAPERS_MARK && auras::has_aura(world, caster, 439851) {
        let target = spell
            .cast_primary_target
            .expect("Wave of Souls primary target");
        let ctx = EffectCtx {
            caster,
            spell,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        };
        let first = (combat::random::random::<f64>() * 100.0) as u32;
        let radial = (-2.0 * (1.0 - combat::random::random::<f64>()).ln()).sqrt();
        let angle = std::f64::consts::TAU * combat::random::random::<f64>();
        let second = (1000.0 + 200.0 * radial * angle.cos()).max(0.0) as u32;
        queued_triggers::schedule_callback(world, &ctx, target, first, wave_outbound);
        queued_triggers::schedule_callback(world, &ctx, target, first + second, wave_return);
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
        if instance.is_some() {
            // sc_death_knight.cpp10601–10605/11909: each real spender schedules one
            // hit; native charged spell-mod consumption spends the corresponding use.
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

#[derive(Component)]
struct WaveState {
    primary: Entity,
    returning: bool,
}

fn wave_crit_chance(world: &World, caster: Entity, _: Entity, spell: u32) -> Option<f32> {
    (spell == 435802
        && world
            .get::<WaveState>(caster)
            .is_some_and(|state| state.returning))
    .then_some(100.0)
}

fn wave_outbound(world: &mut World, caster: Entity, primary: Entity, depth: u8) {
    launch_wave(world, caster, primary, depth, false);
}

fn wave_return(world: &mut World, caster: Entity, primary: Entity, depth: u8) {
    launch_wave(world, caster, primary, depth, true);
}

fn launch_wave(world: &mut World, caster: Entity, primary: Entity, depth: u8, returning: bool) {
    let Some(mut spell) = spell_cast::load_triggered_spell(world, 435802, depth) else {
        return;
    };
    spell.effects.retain(|effect| effect.index == 1);
    let radius = spell
        .misc
        .as_ref()
        .and_then(|misc| misc.range.as_ref())
        .expect("Wave of Souls range")
        .max_hostile;
    let center =
        combat::spell_cast::targets::position(world, caster).expect("Wave caster position");
    let mut targets = combat::spell_cast::targets::area_units(
        world,
        caster,
        center,
        radius,
        combat::spell_cast::targets::Relation::Enemy,
        0,
    );
    if let Some(index) = targets.iter().position(|target| *target == primary) {
        targets.swap(0, index);
    }
    world
        .entity_mut(caster)
        .insert(WaveState { primary, returning });
    for target in targets {
        spell.cast_damage_multiplier = Some(
            if target == primary && auras::has_aura(world, caster, 443560) {
                1.0 + script_effect_value(world, caster, 443560, 1) / 100.0
            } else {
                1.0
            },
        );
        spell_cast::launch_triggered_spell(world, caster, &spell, Some(target), depth);
    }
    world.entity_mut(caster).remove::<WaveState>();
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
    if spell.id == 439594 && effect == 0 {
        targets.retain(|target| Some(*target) != _primary);
    }
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
