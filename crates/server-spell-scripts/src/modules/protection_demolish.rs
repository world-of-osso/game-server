//! Protection Demolish's native channel ticks and Decimator bleed.
use super::combat;
use bevy::prelude::*;
use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::{ActiveAuras, AuraInstance};
use combat::spell_cast::{self, effects::EffectCtx, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::LateAuraRegistration(register),
    SpellScriptHook::ResolvedDamage(decimator),
    SpellScriptHook::ResolvedDamage(apply_wrecked),
    SpellScriptHook::SnapshotNativeAuraAmount(scale_wrecked),
];

fn protection(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 73)
}

fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        436358,
        AuraScript {
            on_periodic: Some(demolish_tick),
            ..Default::default()
        },
    );
}

fn demolish_tick(world: &mut World, target: Entity, aura: &AuraInstance, index: u8) {
    if index != 0 || !protection(world, aura.caster) {
        return;
    }
    // SimC76f8118 sc_warrior.cpp 3873–3892: native 100ms channel
    // ticks 1, 3 and 8 execute the three authored damage spells.
    let tick = aura
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .expect("Demolish requires its native periodic effect")
        .periodic_ticks();
    let spell = match tick {
        1 => 440884,
        3 => 440886,
        8 => 440888,
        _ => return,
    };
    spell_cast::cast_triggered(world, aura.caster, spell, Some(target), 1);
    if tick == 8 && super::cast_relationships::has_talent(world, aura.caster, 1270710) {
        for buff in [1270843, 1270846] {
            if !spell_cast::try_cast_triggered(world, aura.caster, buff, Some(aura.caster), 1) {
                warn!(?buff, "Celeritous Conclusion buff refused");
            }
        }
    }
}

fn scale_wrecked(_: &World, spell: &SpellInfo, index: u8, amount: f32) -> f32 {
    match (spell.id, index) {
        // VERIFIED_DB2: tooltip uses tenths; keep Protection's label-4007 modifier.
        (447513, 0 | 1) => amount / 10.0,
        // INFERRED_WCL: Devastator crit ratios show no Wrecked effect in reports
        // WVLfZxkmRFcMjnv9/35, RWb3vtjQJnmC9rfG/8, ya4H3X78fx19gpJF/1,
        // jxLNVC2MRw8vtJ74/8; SimC76f8118 sc_warrior.cpp:1193-1197 omits it.
        // Retire when retail crit ratio rises with Wrecked stacks.
        (447513, 2) => 0.0,
        _ => amount,
    }
}

fn apply_wrecked(world: &mut World, ctx: &EffectCtx, target: Entity, _: bool, _: f32) {
    if !matches!(ctx.spell.id, 440884 | 440886 | 440888)
        || !protection(world, ctx.caster)
        || !super::cast_relationships::has_talent(world, ctx.caster, 429636)
    {
        return;
    }
    let stacks = world
        .get::<ActiveAuras>(ctx.caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == 440989 && aura.caster == ctx.caster)
                .map(|aura| aura.stacks)
        })
        .unwrap_or(0);
    if stacks == 0 {
        return;
    }
    // SimC sc_warrior.cpp3840–3845: each impact adds the current Might stacks.
    if !spell_cast::cast_triggered_with_overrides(
        world,
        ctx.caster,
        447513,
        Some(target),
        ctx.depth + 1,
        spell_cast::TriggeredSpellOverrides {
            aura_stacks: Some(stacks),
            ..Default::default()
        },
    ) {
        warn!(caster=?ctx.caster, ?target, "Wrecked application refused");
    }
}

fn decimator(world: &mut World, ctx: &EffectCtx, target: Entity, _: bool, amount: f32) {
    // SimC76f8118 sc_warrior.cpp 3842–3848: final damage applies
    // native Deep Wounds on each target when Decimator is selected.
    if ctx.spell.id == 440888
        && amount > 0.0
        && protection(world, ctx.caster)
        && combat::auras::has_aura(world, ctx.caster, 1270704)
    {
        spell_cast::cast_triggered(world, ctx.caster, 262115, Some(target), ctx.depth + 1);
    }
}
