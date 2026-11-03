//! TC a352b1fa spell_mage.cpp1544–1595: Meteor's scripted AT lifecycle.
//! Native153564 supplies the 1s missile delay and child175396/351140 dispatch.
use bevy::prelude::*;

use super::{SpellAreaTrigger, catalog::AuraFieldCatalog, targets::Relation};
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

/// Observe existing TC delay fields and admitted missiles; never retain a separate timer.
pub(crate) fn pending_impact_due(world: &World, caster: Entity) -> Result<Option<f64>, String> {
    let mut earliest = world
        .resource::<crate::spell_cast::missiles::PendingSpellHits>()
        .earliest_due(caster, METEOR_MISSILE);
    let Some(mut query) = world.try_query::<(&SpellAreaTrigger, &shared::components::Position)>()
    else {
        return Ok(earliest);
    };
    for (trigger, position) in query
        .iter(world)
        .filter(|(trigger, _)| trigger.caster == caster && trigger.spell_id == DELAY_FIELD)
    {
        let Some(expires) = trigger.expires_at else {
            continue;
        };
        let missile = world
            .resource::<crate::spell_info::SpellStore>()
            .get(METEOR_MISSILE)
            .ok_or("native Meteor continuation153564 definition absent")?;
        let destination = Vec3::new(position.x, position.y, position.z);
        let delay =
            crate::spell_cast::missiles::destination_delay_ms(world, caster, missile, destination);
        let due = expires + delay as f64 / 1000.0;
        earliest = Some(earliest.map_or(due, |existing| existing.min(due)));
    }
    Ok(earliest)
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
        // SimC76f8118 sc_mage.cpp4721/4741–4744: the scheduled impact is
        // unconditional travel, not a new foreground cast competing with Pyroblast.
        if let Err(reason) =
            cast_at_destination(world, trigger.caster, METEOR_MISSILE, position, true)
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

#[cfg(test)]
mod order_tests {
    use super::*;
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_npc};
    use shared::protocol::CombatLogKind;

    #[test]
    fn simultaneous_burn_exits_emit_aura_removals_in_entity_order() {
        let mut h = harness(Vec::new(), &[BURN_DAMAGE], 8).expect("real spell fixture");
        h.app.init_resource::<ObservedCombatEvents>();
        let mut units = vec![h.npc];
        units.extend((1..7).map(|i| spawn_npc(&mut h.app, i as f32)));
        for &target in &units {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: h.caster,
                    target,
                    spell_id: BURN_DAMAGE,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
            assert!(crate::auras::has_aura(h.app.world(), target, BURN_DAMAGE));
        }
        h.app
            .world_mut()
            .resource_mut::<ObservedCombatEvents>()
            .0
            .clear();
        let trigger = SpellAreaTrigger {
            caster: h.caster,
            primary_target: None,
            spell_id: BURN_FIELD,
            trigger_spell: BURN_DAMAGE,
            radius: 5.0,
            expires_at: None,
            period_ms: 0,
            next_tick_at: 0.0,
            relation: Relation::Enemy,
            depth: 0,
            cast_item_level: None,
            cast_item_inventory_type: None,
            inside: units.iter().copied().collect(),
        };
        super::super::remove_exited(h.app.world_mut(), &trigger, &[]);
        let removed: Vec<_> = h
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::AuraRemoved && event.spell_id == Some(BURN_DAMAGE)
            })
            .filter_map(|event| event.target.and_then(Entity::try_from_bits))
            .collect();
        units.sort_unstable();
        assert_eq!(removed, units);
    }
}
