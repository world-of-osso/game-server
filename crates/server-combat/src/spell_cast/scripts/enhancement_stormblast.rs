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
const ELEMENTAL_WEAPONS: u32 = 384355;

#[derive(Component)]
struct StrikeSnapshot {
    stormblast: bool,
}

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        STORMSURGE_DRIVER,
        AuraScript {
            check_proc: Some(check_strike_weapon),
            on_proc: Some(grant_stormsurge),
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

fn check_strike_weapon(world: &World, aura: &AuraInstance, event: &AuraEvent) -> bool {
    is_enhancement(world, aura.caster)
        && matches!(event.spell_id, Some(32175 | 32176 | 115357 | 115360))
        && !event.periodic
}

fn suppress_every_cast_grant(_world: &World, _aura: &AuraInstance, _event: &AuraEvent) -> bool {
    false
}

fn grant_stormsurge(
    world: &mut World,
    caster: Entity,
    _aura: &AuraInstance,
    _target: Option<Entity>,
) {
    // Generic proc admission keeps native201845 chance, mastery/Storm's Wrath modifiers and timer.
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

pub(super) fn copy_weapon_damage(world: &mut World, ctx: &EffectCtx, target: Entity, damage: f32) {
    if !matches!(ctx.spell.id, 32175 | 32176 | 115357 | 115360)
        || !selected(world, ctx.caster, STORMBLAST_TALENT)
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
    let weapons = if selected(world, caster, ELEMENTAL_WEAPONS) {
        let imbues = [319773, 319778]
            .iter()
            .filter(|buff| auras::has_aura(world, caster, **buff))
            .count();
        super::script_effect_value(world, caster, ELEMENTAL_WEAPONS, 0) / 100.0
            * 0.1
            * imbues as f32
    } else {
        0.0
    };
    (1.0 + mastery) * (1.0 + weapons)
}
