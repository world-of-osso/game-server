//! SimC76f8118 sc_demon_hunter.cpp5830–5856,5874–5889: Voidblade and Pierce execute their native child.
use super::combat;
use bevy::prelude::*;
use combat::spell_cast::{self, script_registry::SpellScriptHook};
use combat::spell_info::SpellInfo;
use shared::components::ActiveSpec;

pub(super) const HOOKS: &[SpellScriptHook] = &[
    SpellScriptHook::EarlyAuraRegistration(register),
    SpellScriptHook::BeforeNativeOnCast(trigger_voidstep),
    SpellScriptHook::AfterFireDirectHit(apply_devourers_bite),
    SpellScriptHook::AfterFireDirectHit(reset_immolation_on_hunt),
    SpellScriptHook::ResolvedDamage(apply_hunt_secondary_dots),
    SpellScriptHook::AfterNativeLaunch(after_launch),
];

fn register(scripts: &mut combat::auras::scripts::AuraScripts) {
    scripts.0.insert(
        1239525,
        combat::auras::scripts::AuraScript {
            on_remove: Some(clear_reapers_replacement),
            ..Default::default()
        },
    );
}

pub(super) fn clear_reapers_replacement(
    world: &mut World,
    caster: Entity,
    _: &combat::auras::AuraInstance,
    _: combat::auras::RemoveReason,
) {
    combat::auras::remove_auras(
        world,
        caster,
        combat::auras::RemoveReason::Consumed,
        |aura| aura.caster == caster && aura.spell_id == 1245523,
    );
}

fn trigger_voidstep(world: &mut World, caster: Entity, spell: &SpellInfo, depth: u8) {
    if spell.id != 198793
        || world.get::<ActiveSpec>(caster).map(|spec| spec.0) != Some(1480)
        || !combat::auras::has_aura(world, caster, 1223157)
    {
        return;
    }
    // SimC76f8118 cpp8542–8556,8610–8615 delivers Voidstep before Retreat consumes it.
    let target = spell
        .cast_primary_target
        .expect("Retreat retains its target");
    if !spell_cast::try_cast_triggered(world, caster, 1239526, Some(target), depth + 1) {
        error!(?caster, "native Voidstep damage was refused");
    }
    combat::auras::remove_auras(
        world,
        caster,
        combat::auras::RemoveReason::Consumed,
        |aura| aura.caster == caster && aura.spell_id == 1223157,
    );
}

fn apply_devourers_bite(
    world: &mut World,
    ctx: &spell_cast::effects::EffectCtx<'_>,
    target: Entity,
    _: bool,
) {
    if !matches!(ctx.spell.id, 1245414 | 1246169)
        || ctx.spell.cast_primary_target != Some(target)
        || world.get::<ActiveSpec>(ctx.caster).map(|spec| spec.0) != Some(1480)
        || !world
            .get::<combat::class_progression::KnownSpells>(ctx.caster)
            .is_some_and(|known| known.contains(1240201))
    {
        return;
    }
    // SimC76f8118 cpp5518–5523,5841–5846: landed primary hit applies the target debuff.
    if !spell_cast::try_cast_triggered(world, ctx.caster, 1241532, Some(target), ctx.depth + 1) {
        error!(?target, "native Devourer's Bite was refused");
    }
}

fn apply_hunt_secondary_dots(
    world: &mut World,
    ctx: &spell_cast::effects::EffectCtx<'_>,
    target: Entity,
    _: bool,
    amount: f32,
) {
    if ctx.spell.id == 1246169 && amount > 0.0
        && world.get::<ActiveSpec>(ctx.caster).map(|spec| spec.0) == Some(1480)
    {
        super::havoc_children::apply_hunt_secondary_dots(world, ctx, target, 1246168);
    }
}

fn reset_immolation_on_hunt(
    world: &mut World,
    ctx: &spell_cast::effects::EffectCtx<'_>,
    target: Entity,
    _: bool,
) {
    if ctx.spell.id != 1246169
        || ctx.spell.cast_primary_target != Some(target)
        || world.get::<ActiveSpec>(ctx.caster).map(|spec| spec.0) != Some(1480)
        || !world
            .get::<combat::class_progression::KnownSpells>(ctx.caster)
            .is_some_and(|known| known.contains(452409))
    {
        return;
    }
    // SimC76f8118 cpp5529–5536 restores one Soul Immolation charge on primary Hunt impact.
    super::devourer_metamorphosis::restore_spell_charges(world, ctx.caster, 1241937, 1);
}

fn after_launch(world: &mut World, caster: Entity, spell: &SpellInfo) {
    if world.get::<ActiveSpec>(caster).map(|spec| spec.0) != Some(1480) {
        return;
    }
    // SimC76f8118 cpp3203–3219,5502,5830: Hunt and Voidblade/Pierce grant Hungering Slash.
    if matches!(spell.id, 1245412 | 1245483 | 1246167 | 1259431)
        && world
            .get::<combat::class_progression::KnownSpells>(caster)
            .is_some_and(|known| known.contains(1239519))
    {
        grant_hungering_slash(world, caster);
    }
    if !matches!(spell.id, 1245412 | 1245483) {
        return;
    }
    let target = spell.cast_primary_target.expect("native Voidblade target");
    if !spell_cast::try_cast_triggered(world, caster, 1245414, Some(target), 1) {
        warn!(
            ?caster,
            ?target,
            "native Voidblade child 1245414 was refused"
        );
    }
}

fn grant_hungering_slash(world: &mut World, caster: Entity) {
    if !spell_cast::try_cast_triggered(world, caster, 1239525, Some(caster), 1) {
        error!(?caster, "native Hungering Slash aura was refused");
    }
    grant_reapers_replacement(world, caster);
}

pub(super) fn grant_reapers_replacement(world: &mut World, caster: Entity) {
    if combat::auras::has_aura(world, caster, 1217607)
        && combat::auras::has_aura(world, caster, 1239525)
        && world
            .get::<combat::class_progression::KnownSpells>(caster)
            .is_some_and(|known| known.contains(452402))
        && !spell_cast::try_cast_triggered(world, caster, 1245523, Some(caster), 1)
    {
        error!(?caster, "native Reaper's Toll replacement was refused");
    }
}
