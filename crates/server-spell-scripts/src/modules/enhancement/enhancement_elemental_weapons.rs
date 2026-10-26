//! SimC76f8118 sc_shaman.cpp13212–13220: dynamic masked damage per imbued hand.
use super::combat;
use bevy::prelude::*;

use combat::auras::scripts::{AuraScript, AuraScripts};
use combat::auras::{self, ActiveAuras, AuraApplication, AuraInstance, AuraSource, RemoveReason};

const TALENT: u32 = 384355;
const MODIFIER: u32 = 408390;
const IMBUES: &[u32] = &[319773, 319778];
const IMBUE_TALENT_FRACTION: f32 = 0.1;

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        TALENT,
        AuraScript {
            on_apply: Some(refresh),
            on_remove: Some(refresh_after_remove),
            ..Default::default()
        },
    );
}

/// Native talent amount ×0.1 per imbued hand; Stormblast shares this explicit exception.
pub(super) fn percent(world: &World, caster: Entity) -> f32 {
    if !auras::has_aura(world, caster, TALENT) {
        return 0.0;
    }
    let hands = IMBUES
        .iter()
        .filter(|buff| auras::has_aura(world, caster, **buff))
        .count();
    super::script_effect_value(world, caster, TALENT, 0) * IMBUE_TALENT_FRACTION * hands as f32
}

pub(super) fn refresh_after_remove(
    world: &mut World,
    unit: Entity,
    aura: &AuraInstance,
    _reason: RemoveReason,
) {
    refresh(world, unit, aura);
}

pub(super) fn refresh(world: &mut World, unit: Entity, _aura: &AuraInstance) {
    let amount = percent(world, unit);
    if amount == 0.0 {
        auras::remove_auras(world, unit, RemoveReason::Unlearned, |aura| {
            aura.spell_id == MODIFIER && aura.caster == unit
        });
        return;
    }
    if !auras::has_aura(world, unit, MODIFIER) {
        auras::apply_aura(
            world,
            &AuraApplication {
                caster: unit,
                target: unit,
                spell_id: MODIFIER,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    update_amount(world, unit, amount);
}

fn update_amount(world: &mut World, unit: Entity, amount: f32) {
    let instance = world
        .get::<ActiveAuras>(unit)
        .and_then(|auras| {
            auras
                .iter()
                .find(|aura| aura.spell_id == MODIFIER && aura.caster == unit)
        })
        .expect("native Elemental Weapons modifier")
        .instance_id;
    // Keep408390's native direct/periodic operations and class masks, replacing only zero amounts.
    for index in [0, 1] {
        assert!(auras::set_effect_amount(
            world, unit, instance, index, amount
        ));
    }
}
