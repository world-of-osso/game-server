//! TC a352b1fa Leymor Explosive Brand and Telash Frost Bomb expiry hooks.
//! Boss schedules, energy/jump phases and rune/ground area actions remain unported.
use bevy::prelude::*;

use crate::auras::{
    AuraInstance, RemoveReason, handlers,
    scripts::{AuraScript, AuraScripts},
};

pub(crate) const FROST_BOMB_CAST: u32 = 386781;
const FROST_BOMB_AURA: u32 = 386881;
const FROZEN_GROUND: u32 = 387149;
const FROST_BOMB_DAMAGE: u32 = 386910;
const EXPLOSIVE_BRAND: u32 = 374567;
const EXPLOSIVE_BRAND_DAMAGE: u32 = 374570;

pub(super) fn register_aura_scripts(mut scripts: ResMut<AuraScripts>) {
    scripts.0.insert(
        EXPLOSIVE_BRAND,
        AuraScript {
            on_remove: Some(explode_expired_brand),
            ..Default::default()
        },
    );
    scripts.0.insert(
        FROST_BOMB_AURA,
        AuraScript {
            on_remove: Some(explode_expired_frost_bomb),
            ..Default::default()
        },
    );
}

/// OnEffectHitTarget, effect0/SPELL_EFFECT_DUMMY.
pub(crate) fn apply_frost_bomb(world: &mut World, caster: Entity, target: Entity, _depth: u8) {
    cast_azure_child(world, caster, target, FROST_BOMB_AURA);
}

/// OnEffectRemove, effect0/SPELL_AURA_DUMMY, AURA_REMOVE_BY_EXPIRE only.
fn explode_expired_frost_bomb(
    world: &mut World,
    target: Entity,
    aura: &AuraInstance,
    reason: RemoveReason,
) {
    let removed_dummy = aura
        .effects
        .iter()
        .any(|effect| effect.index == 0 && effect.aura_type == handlers::DUMMY);
    if reason != RemoveReason::Expired || !removed_dummy || world.get_entity(aura.caster).is_err() {
        return;
    }
    cast_azure_child(world, aura.caster, target, FROZEN_GROUND);
    cast_azure_child(world, aura.caster, target, FROST_BOMB_DAMAGE);
}

/// AfterEffectRemove, effect0/SPELL_AURA_DUMMY, AURA_REMOVE_BY_EXPIRE only.
fn explode_expired_brand(
    world: &mut World,
    target: Entity,
    aura: &AuraInstance,
    reason: RemoveReason,
) {
    let removed_dummy = aura
        .effects
        .iter()
        .any(|effect| effect.index == 0 && effect.aura_type == handlers::DUMMY);
    if reason != RemoveReason::Expired || !removed_dummy || world.get_entity(aura.caster).is_err() {
        return;
    }
    cast_azure_child(world, aura.caster, target, EXPLOSIVE_BRAND_DAMAGE);
}

fn cast_azure_child(world: &mut World, caster: Entity, target: Entity, spell: u32) {
    if let Err(error) = crate::spell_cast::creature_cast(world, caster, spell, Some(target), true) {
        warn!("Azure child {spell} from {caster} to {target} failed: {error:?}");
    }
}
