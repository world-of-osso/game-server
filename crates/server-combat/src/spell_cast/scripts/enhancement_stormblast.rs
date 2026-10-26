//! SimC76f8118 sc_shaman.cpp4449–4469/4702–4711/5597–5600/12125–12147:
//! weapon Strike hits earn Stormsurge; the next root snapshots Stormblast before consumption.
use bevy::prelude::*;
use shared::components::ActiveSpec;

use crate::auras::procs::AuraEvent;
use crate::auras::scripts::{AuraScript, AuraScripts};
use crate::auras::{self, ActiveAuras, AuraInstance};
use crate::class_progression::KnownSpells;
use crate::spell_cast::{self, effects::EffectCtx};
use crate::spell_info::SpellStore;

const STORMSURGE_DRIVER: u32 = 201845;
const STORMSURGE: u32 = 201846;
const STORMBLAST_TALENT: u32 = 319930;
const STORMBLAST: u32 = 470466;
const STORMBLAST_DAMAGE: u32 = 390287;
const MASTERY: u32 = 77223;

#[derive(Component, Clone, Copy)]
struct StrikeSnapshot {
    stormblast: bool,
}

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        STORMSURGE_DRIVER,
        AuraScript {
            check_proc: Some(suppress_every_cast_grant),
            ..Default::default()
        },
    );
    // Native319930's generated every-cast470466 grant is replaced, not layered under this roll.
    scripts.0.insert(
        STORMBLAST_TALENT,
        AuraScript {
            check_proc: Some(suppress_every_cast_grant),
            ..Default::default()
        },
    );
}

fn is_enhancement(world: &World, caster: Entity) -> bool {
    world
        .get::<ActiveSpec>(caster)
        .is_some_and(|spec| spec.0 == 263)
}

fn selected(world: &World, caster: Entity, talent: u32) -> bool {
    world
        .get::<KnownSpells>(caster)
        .is_some_and(|known| known.contains(talent))
}

fn suppress_every_cast_grant(_world: &World, _aura: &AuraInstance, _event: &AuraEvent) -> bool {
    false
}

fn grant_stormsurge(world: &mut World, caster: Entity) {
    spell_cast::cast_triggered(world, caster, STORMSURGE, Some(caster), 1);
    if selected(world, caster, STORMBLAST_TALENT) {
        spell_cast::cast_triggered(world, caster, STORMBLAST, Some(caster), 1);
    }
    // SimC cooldown.cpp370–380: reset(true) restores all Strike charges.
    let category = world
        .resource::<SpellStore>()
        .get(17364)
        .and_then(|spell| spell.categories.as_ref())
        .expect("native Stormstrike charge category")
        .charge_category;
    spell_cast::restore_charges(world, caster, category, u8::MAX);
}

pub(super) fn snapshotted(world: &World, caster: Entity) -> bool {
    world
        .get::<StrikeSnapshot>(caster)
        .is_some_and(|state| state.stormblast)
}

pub(super) fn execute_flurry(
    world: &mut World,
    caster: Entity,
    payload: &crate::spell_info::SpellInfo,
    stormblast: bool,
) {
    let previous = world.get::<StrikeSnapshot>(caster).copied();
    world
        .entity_mut(caster)
        .insert(StrikeSnapshot { stormblast });
    spell_cast::launch_triggered_spell(world, caster, payload, payload.cast_primary_target, 1);
    if let Some(previous) = previous {
        world.entity_mut(caster).insert(previous);
    } else {
        world.entity_mut(caster).remove::<StrikeSnapshot>();
    }
}

pub(super) fn snapshot(world: &mut World, caster: Entity, spell: u32, depth: u8) {
    if depth != 0 || !matches!(spell, 17364 | 115356) || !is_enhancement(world, caster) {
        return;
    }
    let stormblast = auras::has_aura(world, caster, STORMBLAST);
    world
        .entity_mut(caster)
        .insert(StrikeSnapshot { stormblast });
    for buff in [STORMSURGE, STORMBLAST] {
        let instance = world.get::<ActiveAuras>(caster).and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == buff && aura.caster == caster)
                .map(|aura| aura.instance_id)
        });
        if let Some(instance) = instance {
            auras::spend_stack(world, caster, instance);
        }
    }
}

pub(in crate::spell_cast) fn after_weapon_damage(
    world: &mut World,
    ctx: &EffectCtx,
    target: Entity,
    damage: f32,
) {
    if !matches!(ctx.spell.id, 32175 | 32176 | 115357 | 115360)
        || !is_enhancement(world, ctx.caster)
    {
        return;
    }
    copy_weapon_damage(world, ctx, target, damage);
    roll_stormsurge(world, ctx.caster);
}

fn roll_stormsurge(world: &mut World, caster: Entity) {
    // Native TDB family filters select roots, but SimC4702 rolls on each actual weapon child.
    // Execution policy keeps the native201845 chance, mastery modifiers and timer, not that family gate.
    let Some(instance) = world.get::<ActiveAuras>(caster).and_then(|auras| {
        auras
            .iter()
            .find(|aura| aura.spell_id == STORMSURGE_DRIVER && aura.caster == caster)
            .map(|aura| aura.instance_id)
    }) else {
        return;
    };
    if crate::auras::procs::try_native_policy_proc(world, caster, instance) {
        grant_stormsurge(world, caster);
    }
}

fn copy_weapon_damage(world: &mut World, ctx: &EffectCtx, target: Entity, damage: f32) {
    if !selected(world, ctx.caster, STORMBLAST_TALENT)
        || !world
            .get::<StrikeSnapshot>(ctx.caster)
            .is_some_and(|state| state.stormblast)
    {
        return;
    }
    let coefficient = super::script_effect_value(world, ctx.caster, STORMBLAST_TALENT, 0) / 100.0;
    let amount = damage * coefficient * copy_multiplier(world, ctx.caster);
    // Native390287 cannot crit and ignores broad caster modifiers. Its target modifiers remain native.
    spell_cast::cast_triggered_with_effect0_amount(
        world,
        ctx.caster,
        STORMBLAST_DAMAGE,
        Some(target),
        ctx.depth + 1,
        amount,
    );
}

fn copy_multiplier(world: &World, caster: Entity) -> f32 {
    // SimC13184–13187/13212–13220 explicitly add this otherwise-unmasked copy to these two lists.
    let mastery = world
        .get::<ActiveAuras>(caster)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == MASTERY && aura.caster == caster)
                .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
                .map(|effect| effect.amount / 100.0)
        })
        .expect("Enhancement's native mastery aura/effect0 must be active");
    let weapons = super::enhancement_imbues::elemental_weapons_percent(world, caster) / 100.0;
    (1.0 + mastery) * (1.0 + weapons)
}
