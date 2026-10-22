//! TC a352b1fa spell_mage.cpp1544–1595: Meteor's scripted AT lifecycle.
//! Native153564 supplies the 1s missile delay and child175396/351140 dispatch.
use bevy::prelude::*;

use super::{SpellAreaTrigger, catalog::AuraFieldCatalog, targets::Relation};
use crate::auras::{ActiveAuras, RemoveReason};
use crate::spell_cast::effects::EffectCtx;
use crate::spell_info::SpellEffect;

pub(crate) const METEOR: u32 = 153561;
pub(super) const DELAY_FIELD: u32 = 177345;
const DELAY_PROPERTIES: i32 = 3467;
const METEOR_MISSILE: u32 = 153564;
pub(super) const BURN_FIELD: u32 = 175396;
pub(super) const BURN_PROPERTIES: u32 = 1712;
pub(super) const BURN_TEMPLATE: u32 = 6211;
pub(super) const BURN_DAMAGE: u32 = 155158;

pub(crate) fn cast_meteor(world: &mut World, ctx: &EffectCtx, _effect: &SpellEffect) {
    let Some(dest) = ctx.dest else {
        warn!("Meteor153561 has no authored destination");
        return;
    };
    crate::spell_cast::cast_triggered_at_destination(
        world,
        ctx.caster,
        DELAY_FIELD,
        dest.position,
        ctx.depth + 1,
    );
}

pub(super) fn is_delay_field(spell: u32, properties: i32) -> bool {
    spell == DELAY_FIELD && properties == DELAY_PROPERTIES
}

pub(super) fn burn_binding(world: &World) -> Result<(f32, Relation, u32), String> {
    let definition = world
        .get_resource::<AuraFieldCatalog>()
        .ok_or("Meteor Burn field catalog absent")?
        .definition(BURN_PROPERTIES)?;
    Ok((
        definition.radius_at(0.0),
        definition.relation,
        definition.trigger_spell,
    ))
}

pub(super) fn expire_delay(world: &mut World, trigger: &SpellAreaTrigger, position: Vec3) {
    if trigger.spell_id == DELAY_FIELD {
        crate::spell_cast::cast_triggered_at_destination(
            world,
            trigger.caster,
            METEOR_MISSILE,
            position,
            trigger.depth,
        );
    }
}

pub(super) fn remove_exited(world: &mut World, trigger: &SpellAreaTrigger, inside: &[Entity]) {
    if trigger.spell_id != BURN_FIELD {
        return;
    }
    for unit in trigger.inside.iter().filter(|unit| !inside.contains(unit)) {
        let instances: Vec<_> = world
            .get::<ActiveAuras>(*unit)
            .into_iter()
            .flat_map(|auras| auras.iter())
            .filter(|aura| aura.spell_id == BURN_DAMAGE && aura.caster == trigger.caster)
            .map(|aura| aura.instance_id)
            .collect();
        for instance in instances {
            crate::auras::remove_instance(world, *unit, instance, RemoveReason::Expired);
        }
    }
}
