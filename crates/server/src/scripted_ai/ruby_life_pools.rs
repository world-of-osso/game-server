//! TC a352b1fa ruby_life_pools.cpp periodic aura hooks. Boss AIs are absent from
//! this pinned source; no encounter schedules are invented here.
use bevy::prelude::*;

use crate::auras::{
    AuraInstance, handlers,
    scripts::{AuraScript, AuraScripts},
};

const ICE_SHIELD_PERIODIC: u32 = 384933;
const ICE_SHIELD: u32 = 372749;
const EXCAVATE_PERIODIC: u32 = 372793;
const EXCAVATE: u32 = 373497;

pub(super) fn register_aura_scripts(mut scripts: ResMut<AuraScripts>) {
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
