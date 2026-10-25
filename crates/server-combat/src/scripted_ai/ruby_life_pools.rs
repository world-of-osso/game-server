//! TC a352b1fa ruby_life_pools.cpp periodic aura hooks and explicitly local
//! current-Retail Lua-clock partial bosses. Pinned TC has no Ruby boss AI.
use bevy::prelude::*;

use crate::auras::{
    AuraInstance, handlers,
    scripts::{AuraScript, AuraScripts},
};

pub(super) fn is_creature_script(name: &str) -> bool {
    matches!(name, super::melidrussa::SCRIPT | super::kokia::SCRIPT)
}

pub(super) fn creature_ai(world: &World, actor: Entity) -> Option<Box<dyn super::CreatureAi>> {
    let map = world.get::<crate::maps::InMap>(actor)?;
    let maps = world.get_resource::<crate::maps::MapStore>()?;
    if maps.instance_scripts.get(&map.map_id).map(String::as_str)
        != Some("instance_ruby_life_pools")
    {
        return None;
    }
    let boss = super::instance_script::bound_boss(world, actor)?;
    match world.get::<super::CreatureScriptName>(actor)?.0.as_str() {
        super::melidrussa::SCRIPT => Some(super::melidrussa::creature_ai(boss)),
        super::kokia::SCRIPT => Some(super::kokia::creature_ai(boss)),
        _ => None,
    }
}

const EXECUTED: u32 = 371652;
const ICE_SHIELD_PERIODIC: u32 = 384933;
const ICE_SHIELD: u32 = 372749;
const EXCAVATE_PERIODIC: u32 = 372793;
const EXCAVATE: u32 = 373497;
pub(crate) const STORM_INFUSION: u32 = 395029;
const STORM_FORWARD_OFFSET: f32 = 9.0;
const STORM_HEIGHT_OFFSET: f32 = 4.0;

/// TC Position::RelocateOffset: local(9,0,4) rotated around the destination's
/// WoW orientation. WoW Z is engine Y; WoW Y is negative engine Z.
pub(crate) fn relocate_storm_destination(dest: &mut crate::spell_cast::targets::Dest) {
    dest.position.x += STORM_FORWARD_OFFSET * dest.orientation.cos();
    dest.position.z -= STORM_FORWARD_OFFSET * dest.orientation.sin();
    dest.position.y += STORM_HEIGHT_OFFSET;
}

pub(super) fn register_aura_scripts(mut scripts: ResMut<AuraScripts>) {
    scripts.0.insert(
        EXECUTED,
        AuraScript {
            on_apply: Some(set_executed_flags),
            ..Default::default()
        },
    );
    scripts.0.insert(
        ICE_SHIELD_PERIODIC,
        AuraScript {
            on_periodic: Some(refresh_ice_shield),
            ..Default::default()
        },
    );
    scripts.0.insert(
        EXCAVATE_PERIODIC,
        AuraScript {
            on_periodic: Some(cast_excavate),
            ..Default::default()
        },
    );
}

/// Executed's effect0/STUN OnEffectApply only; source has no flag cleanup hook.
fn set_executed_flags(world: &mut World, target: Entity, aura: &AuraInstance) {
    use shared::components::{UnitFlags, UnitFlags2, UnitFlags3};
    let applied_stun = aura
        .effects
        .iter()
        .any(|effect| effect.index == 0 && effect.aura_type == handlers::MOD_STUN);
    if !applied_stun {
        return;
    }
    let flags = world.get::<UnitFlags>(target).map_or(0, |flags| flags.0);
    let flags2 = world.get::<UnitFlags2>(target).map_or(0, |flags| flags.0);
    let flags3 = world.get::<UnitFlags3>(target).map_or(0, |flags| flags.0);
    world.entity_mut(target).insert((
        UnitFlags(flags | UnitFlags::PREVENT_EMOTES_FROM_CHAT_TEXT),
        UnitFlags2(flags2 | UnitFlags2::FEIGN_DEATH),
        UnitFlags3(flags3 | UnitFlags3::FAKE_DEAD),
    ));
}

fn is_periodic_effect_zero(aura: &AuraInstance, index: u8) -> bool {
    index == 0
        && aura
            .effects
            .iter()
            .any(|effect| effect.index == index && effect.aura_type == handlers::PERIODIC_DUMMY)
}

/// No new aura, absorb refill or caster restriction: source GetAura(372749).
fn refresh_ice_shield(world: &mut World, target: Entity, aura: &AuraInstance, index: u8) {
    if is_periodic_effect_zero(aura, index) {
        crate::auras::refresh_duration(world, target, ICE_SHIELD);
    }
}

fn cast_excavate(world: &mut World, target: Entity, aura: &AuraInstance, index: u8) {
    if is_periodic_effect_zero(aura, index) && world.get_entity(aura.caster).is_ok() {
        if let Err(error) =
            crate::spell_cast::creature_cast(world, aura.caster, EXCAVATE, Some(target), true)
        {
            warn!("Ruby Life Pools Excavate triggered cast failed: {error:?}");
        }
    }
}
