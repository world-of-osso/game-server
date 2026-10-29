//! Preservation Lifebind: 70448 spell 373270 and periodic aura 373267.
use std::collections::HashMap;

use bevy::prelude::*;

use super::scripts::{AuraScript, AuraScripts};
use super::{ActiveAuras, AuraApplication, AuraInstance, AuraSource, RemoveReason};
use crate::spell_cast::effects::{self, EffectCtx};

const TALENT: u32 = 373270;
const BOND: u32 = 373267;
const TRANSFER: u32 = 373268;

#[derive(Component, Default)]
struct PendingTransfers(HashMap<Entity, HashMap<Entity, f32>>);

pub(super) fn register(scripts: &mut AuraScripts) {
    scripts.0.insert(
        BOND,
        AuraScript {
            on_periodic: Some(deliver_transfers),
            on_remove: Some(clear_transfers),
            ..Default::default()
        },
    );
}

/// Verdant Embrace bonds its caster and recipient after its initial heal.
pub fn apply_bond(world: &mut World, caster: Entity, target: Entity) {
    if caster == target || !super::has_aura(world, caster, TALENT) {
        return;
    }
    for unit in [caster, target] {
        super::apply_aura(
            world,
            &AuraApplication {
                caster,
                target: unit,
                spell_id: BOND,
                effect_mask: 1,
                source: AuraSource::Cast,
            },
        );
    }
}

fn has_owned_bond(world: &World, unit: Entity, caster: Entity) -> bool {
    world.get::<ActiveAuras>(unit).is_some_and(|auras| {
        auras
            .iter()
            .any(|aura| aura.spell_id == BOND && aura.caster == caster)
    })
}

/// Accumulate resolved raw healing, including overheal, until the bond's next tick.
pub(crate) fn record_heal(world: &mut World, ctx: &EffectCtx<'_>, target: Entity, amount: f32) {
    let caster = ctx.caster;
    if ctx.spell.id == TRANSFER || amount <= 0.0 || !has_owned_bond(world, caster, caster) {
        return;
    }
    let recipients: Vec<Entity> = if target == caster {
        world
            .query::<(Entity, &ActiveAuras)>()
            .iter(world)
            .filter(|(unit, auras)| {
                *unit != caster
                    && auras
                        .iter()
                        .any(|aura| aura.spell_id == BOND && aura.caster == caster)
            })
            .map(|(unit, _)| unit)
            .collect()
    } else if has_owned_bond(world, target, caster) {
        vec![caster]
    } else {
        return;
    };
    let share = crate::spell_cast::scripts::script_effect_value(world, caster, TALENT, 0) / 100.0;
    let mut entity = world.entity_mut(target);
    let mut pending = entity.entry::<PendingTransfers>().or_default();
    let mut pending = pending.get_mut();
    let transfers = pending.0.entry(caster).or_default();
    for recipient in recipients {
        *transfers.entry(recipient).or_default() += amount * share;
    }
}

fn deliver_transfers(world: &mut World, unit: Entity, aura: &AuraInstance, index: u8) {
    if index != 0 {
        return;
    }
    let Some(transfers) = world
        .get_mut::<PendingTransfers>(unit)
        .and_then(|mut pending| pending.0.remove(&aura.caster))
    else {
        return;
    };
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(TRANSFER)
        .expect("Lifebind requires spell 373268")
        .clone();
    let ctx = EffectCtx {
        caster: aura.caster,
        spell: &spell,
        depth: 1,
        dest: None,
        chain_multiplier: 1.0,
    };
    for (recipient, amount) in transfers {
        if has_owned_bond(world, recipient, aura.caster) {
            effects::apply_fixed_heal(world, &ctx, recipient, amount);
        }
    }
}

fn clear_transfers(world: &mut World, unit: Entity, aura: &AuraInstance, _reason: RemoveReason) {
    if let Some(mut pending) = world.get_mut::<PendingTransfers>(unit) {
        pending.0.remove(&aura.caster);
    }
}
