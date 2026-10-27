//! SimC76f8118 sc_demon_hunter.cpp6529–6553: Collapsing Star spends its souls.
use super::combat;
use bevy::prelude::*;
use combat::auras::{self, ActiveAuras, RemoveReason};
use combat::class_progression::KnownSpells;
use combat::spell_cast::{
    self, effects::EffectCtx, script_registry::SpellScriptHook, scripts::script_effect_value,
};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

#[derive(Component, Default)]
struct PendingApocalypse(Vec<f64>);

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::AfterNativeLaunch(after_launch),
    SpellScriptHook::AfterFireDirectHit(schedule_apocalypse),
    SpellScriptHook::AfterNativeCastUpdate(deliver_apocalypse),
    SpellScriptHook::TargetContextDamageMultiplier(secondary_multiplier),
];

fn schedule_apocalypse(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, _: bool) {
    if ctx.spell.id != 1221162 || ctx.spell.cast_primary_target != Some(target)
        || !world.get::<KnownSpells>(ctx.caster).is_some_and(|known| known.contains(1227707))
    {
        return;
    }
    // SimC76f8118 cpp6516–6523 grants the buff 1.2s after the primary impact.
    let deadline = spell_cast::now(world) + 1.2;
    world.entity_mut(ctx.caster).entry::<PendingApocalypse>().or_default().into_mut().0.push(deadline);
}

fn deliver_apocalypse(world: &mut World) {
    let now = spell_cast::now(world);
    let casters: Vec<_> = world.query_filtered::<Entity, With<PendingApocalypse>>().iter(world).collect();
    for caster in casters {
        let due = {
            let mut pending = world.get_mut::<PendingApocalypse>(caster).expect("pending Apocalypse");
            let before = pending.0.len();
            pending.0.retain(|deadline| *deadline > now);
            before - pending.0.len()
        };
        for _ in 0..due {
            if !spell_cast::try_cast_triggered(world, caster, 1227338, Some(caster), 1) {
                error!(?caster, "native Impending Apocalypse was refused");
            }
        }
    }
}

fn secondary_multiplier(world: &World, ctx: &EffectCtx<'_>, target: Entity, periodic: bool) -> f32 {
    if periodic || ctx.spell.id != 1221162 {
        return 1.0;
    }
    let primary = ctx
        .spell
        .cast_primary_target
        .expect("native Collapsing Star primary target");
    if target == primary {
        1.0
    } else {
        // SimC76f8118 cpp6488–6495 uses the native parent effect2 percentage.
        script_effect_value(world, ctx.caster, 1221150, 1) / 100.0
    }
}

fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if spell.id != 1221150 || world.get::<ActiveSpec>(caster).map(|spec| spec.0) != Some(1480) {
        return;
    }
    let Some(stacks) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.caster == caster && aura.spell_id == 1227702)
            .map(|aura| aura.stacks)
    }) else {
        return;
    };
    let cost = script_effect_value(world, caster, 1221167, 0) as u8;
    let remaining = stacks.saturating_sub(cost);
    auras::remove_auras(world, caster, RemoveReason::Consumed, |aura| {
        aura.caster == caster
            && (aura.spell_id == 1221171 || (aura.spell_id == 1227702 && remaining == 0))
    });
    if remaining > 0 {
        assert!(
            auras::set_owned_stacks(world, caster, caster, 1227702, remaining),
            "native Collapsing Star soul stack"
        );
    }
    // SimC76f8118 cpp6464–6474,6532–6533 executes the native damage child.
    let target = spell
        .cast_primary_target
        .expect("native Collapsing Star target");
    if !spell_cast::try_cast_triggered(world, caster, 1221162, Some(target), 1) {
        warn!(
            ?caster,
            ?target,
            "native Collapsing Star damage 1221162 was refused"
        );
    }
}
