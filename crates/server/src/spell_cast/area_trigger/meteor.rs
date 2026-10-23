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
    if let Err(reason) = cast_at_destination(world, ctx.caster, DELAY_FIELD, dest.position, true) {
        debug!(?reason, "Meteor delay field admission failed");
    }
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
        // TC OnRemove uses TRIGGERED_NONE. In particular, an admitted hardcast
        // rejects this new spell; SimC's unconditional delayed impact differs.
        if let Err(reason) =
            cast_at_destination(world, trigger.caster, METEOR_MISSILE, position, false)
        {
            debug!(?reason, "Meteor expiry missile admission failed");
        }
    }
}

fn cast_at_destination(
    world: &mut World,
    caster: Entity,
    spell_id: u32,
    position: Vec3,
    ignore_in_progress: bool,
) -> Result<(), crate::spell_cast::CastFailure> {
    let mut spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .cloned()
        .ok_or(crate::spell_cast::CastFailure::UnknownSpell)?;
    spell.cast_destination = Some(position.to_array());
    let target =
        crate::spell_cast::validate_start(world, caster, &spell, None, ignore_in_progress)?;
    crate::spell_cast::begin_validated_cast(world, caster, &spell, target)
}

pub(super) fn cast_burn_on_entry(world: &mut World, trigger: &SpellAreaTrigger, unit: Entity) {
    // Source OnUnitEnter waives only cast-in-progress/reporting, not all checks.
    if let Err(reason) = crate::spell_cast::creature_cast_ignoring_in_progress(
        world,
        trigger.caster,
        BURN_DAMAGE,
        Some(unit),
    ) {
        debug!(?reason, "Meteor burn entry admission failed");
    }
}

pub(super) fn remove_exited(world: &mut World, trigger: &SpellAreaTrigger, inside: &[Entity]) {
    if trigger.spell_id != BURN_FIELD {
        return;
    }
    for unit in trigger.inside.iter().filter(|unit| !inside.contains(unit)) {
        for instance in find_owned_burn_instances(world, *unit, trigger.caster) {
            crate::auras::remove_instance(world, *unit, instance, RemoveReason::Expired);
        }
    }
}

fn find_owned_burn_instances(world: &World, unit: Entity, caster: Entity) -> Vec<u32> {
    let Some(auras) = world.get::<ActiveAuras>(unit) else {
        return Vec::new();
    };
    auras
        .iter()
        .filter(|aura| aura.spell_id == BURN_DAMAGE && aura.caster == caster)
        .map(|aura| aura.instance_id)
        .collect()
}
