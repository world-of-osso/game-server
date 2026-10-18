//! Native Sprout self-aura395 must create an actual same-copy owned field.
use super::*;
use crate::spell_cast::area_trigger::SpellAreaTrigger;

const SPROUT_AURA: u32 = 374161;

#[test]
fn azure_leymor_native_sprout_self_auras_create_fourteen_same_copy_fields() {
    let (mut h, leymor) = super::azure_sprouts::prepare_arcane_power_prerequisites();
    let children = h.creatures_of(190509);
    assert_eq!(children.len(), 14);
    let map = *h.app.world().get::<InMap>(leymor).unwrap();
    for &child in &children {
        assert!(h.has_aura(child, SPROUT_AURA));
        assert_eq!(h.app.world().get::<InMap>(child), Some(&map));
    }
    let fields: Vec<_> = h
        .app
        .world_mut()
        .query::<(&SpellAreaTrigger, &Position, &InMap)>()
        .iter(h.app.world())
        .filter(|(field, _, _)| field.spell_id == SPROUT_AURA)
        .map(|(field, position, copy)| (field.caster, *position, *copy))
        .collect();
    assert_eq!(
        fields.len(),
        14,
        "one actual aura395-owned field per native self-aura"
    );
    for child in children {
        let owned: Vec<_> = fields
            .iter()
            .filter(|(caster, _, _)| *caster == child)
            .collect();
        assert_eq!(owned.len(), 1);
        let (_, position, copy) = owned[0];
        assert_eq!(*copy, map);
        let owner_position = h.app.world().get::<Position>(child).unwrap();
        assert_eq!(
            (position.x, position.y, position.z),
            (owner_position.x, owner_position.y, owner_position.z)
        );
    }
    // Entry/exit, scale curve,374523 action and aura/death cleanup need separate proof.
}

fn find_sprout_fields(h: &mut Harness) -> Vec<(Entity, Entity)> {
    h.app
        .world_mut()
        .query::<(Entity, &SpellAreaTrigger)>()
        .iter(h.app.world())
        .filter(|(_, field)| field.spell_id == SPROUT_AURA)
        .map(|(entity, field)| (entity, field.caster))
        .collect()
}

fn find_sprout_aura_instance(h: &Harness, target: Entity) -> u32 {
    h.app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == SPROUT_AURA)
        .unwrap()
        .instance_id
}

#[test]
fn azure_aura395_native_fields_preserve_other_instances_through_removal_refresh_and_death() {
    let (mut h, _leymor) = super::azure_sprouts::prepare_arcane_power_prerequisites();
    let children = h.creatures_of(190509);
    assert_eq!(children.len(), 14);
    let initial = find_sprout_fields(&mut h);
    assert_eq!(initial.len(), 14, "native field creation prerequisite");
    let child = children[0];
    let selected_field = initial.iter().find(|(_, owner)| *owner == child).unwrap().0;
    let aura_id = find_sprout_aura_instance(&h, child);
    crate::auras::remove_instance(
        h.app.world_mut(),
        child,
        aura_id,
        crate::auras::RemoveReason::Expired,
    );
    assert!(!h.has_aura(child, SPROUT_AURA));
    assert!(h.app.world().get_entity(selected_field).is_err());
    let survivors = find_sprout_fields(&mut h);
    assert_eq!(survivors.len(), 13);
    for &(field, owner) in &initial {
        if owner != child {
            assert!(
                survivors.contains(&(field, owner)),
                "unrelated field retained"
            );
            assert!(h.has_aura(owner, SPROUT_AURA));
        }
    }
    crate::spell_cast::cast_triggered(h.app.world_mut(), child, SPROUT_AURA, Some(child), 1);
    let restored = find_sprout_fields(&mut h);
    assert_eq!(restored.len(), 14);
    let replacement = restored
        .iter()
        .find(|(_, owner)| *owner == child)
        .unwrap()
        .0;
    assert_ne!(replacement, selected_field);
    crate::spell_cast::cast_triggered(h.app.world_mut(), child, SPROUT_AURA, Some(child), 1);
    let refreshed = find_sprout_fields(&mut h);
    assert_eq!(refreshed.len(), 14);
    assert!(
        refreshed.contains(&(replacement, child)),
        "refresh must retain the field instead of creating a duplicate"
    );
    let player = h.player();
    crate::combat::kill_unit(h.app.world_mut(), player, child);
    h.advance(0.1);
    assert!(!crate::spell_cast::targets::is_alive(h.app.world(), child));
    assert!(!h.has_aura(child, SPROUT_AURA));
    assert!(h.app.world().get_entity(replacement).is_err());
    let after_death = find_sprout_fields(&mut h);
    assert_eq!(after_death.len(), 13);
    for survivor in survivors {
        assert!(after_death.contains(&survivor));
    }
}

fn replace_self_aura_from_caster(h: &mut Harness, caster: Entity, target: Entity) -> Entity {
    crate::auras::remove_auras(
        h.app.world_mut(),
        target,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == SPROUT_AURA,
    );
    // Native aura application boundary; this is not an ordinary self-target cast.
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: SPROUT_AURA,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    find_sprout_field_at_target(h, caster, target)
}

fn find_sprout_field_at_target(h: &mut Harness, caster: Entity, target: Entity) -> Entity {
    let target_position = *h.app.world().get::<Position>(target).unwrap();
    let target_map = *h.app.world().get::<InMap>(target).unwrap();
    let matching: Vec<_> = h
        .app
        .world_mut()
        .query::<(Entity, &SpellAreaTrigger, &Position, &InMap)>()
        .iter(h.app.world())
        .filter(|(_, field, _, _)| field.spell_id == SPROUT_AURA && field.caster == caster)
        .filter(|(_, _, position, map)| {
            let field_at = (position.x, position.y, position.z);
            let target_at = (target_position.x, target_position.y, target_position.z);
            field_at == target_at && **map == target_map
        })
        .map(|(entity, _, _, _)| entity)
        .collect();
    assert_eq!(
        matching.len(),
        1,
        "original caster differs from aura target"
    );
    matching[0]
}

#[test]
fn azure_aura395_same_caster_keeps_different_target_instances_independent() {
    let (mut h, leymor) = super::azure_sprouts::prepare_arcane_power_prerequisites();
    let children = h.creatures_of(190509);
    assert_eq!(children.len(), 14);
    let first_target = children[0];
    let first_position = *h.app.world().get::<Position>(first_target).unwrap();
    let second_target = *children
        .iter()
        .find(|&&child| {
            let position = h.app.world().get::<Position>(child).unwrap();
            (position.x, position.y, position.z)
                != (first_position.x, first_position.y, first_position.z)
        })
        .expect("distinct native attachment positions");
    let first_field = replace_self_aura_from_caster(&mut h, leymor, first_target);
    let second_field = replace_self_aura_from_caster(&mut h, leymor, second_target);
    assert_ne!(first_field, second_field);
    let first_aura = find_sprout_aura_instance(&h, first_target);
    let second_aura = find_sprout_aura_instance(&h, second_target);
    let initial = find_sprout_fields(&mut h);
    assert_eq!(initial.len(), 14);
    crate::auras::remove_instance(
        h.app.world_mut(),
        first_target,
        first_aura,
        crate::auras::RemoveReason::Cancelled,
    );
    assert!(h.app.world().get_entity(first_field).is_err());
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(second_target)
            .unwrap()
            .get(second_aura)
            .is_some()
    );
    let remaining = find_sprout_fields(&mut h);
    assert_eq!(remaining.len(), 13);
    assert!(remaining.contains(&(second_field, leymor)));
    for binding in initial
        .into_iter()
        .filter(|(field, _)| *field != first_field)
    {
        assert!(
            remaining.contains(&binding),
            "unrelated exact field retained"
        );
    }
}

#[test]
fn azure_aura395_attachment_follows_its_target_and_copy_departure_retires_only_its_field() {
    let (mut h, leymor) = super::azure_sprouts::prepare_arcane_power_prerequisites();
    let children = h.creatures_of(190509);
    assert_eq!(children.len(), 14);
    let target = children[0];
    let original = *h.app.world().get::<Position>(target).unwrap();
    let destination = children
        .iter()
        .filter_map(|&child| h.app.world().get::<Position>(child).copied())
        .find(|position| {
            (position.x, position.y, position.z) != (original.x, original.y, original.z)
        })
        .expect("another actual native attachment position");
    let field = replace_self_aura_from_caster(&mut h, leymor, target);
    let initial = find_sprout_fields(&mut h);
    assert_eq!(initial.len(), 14);
    h.app.world_mut().entity_mut(target).insert(destination);
    h.advance(0.05);
    let at = h.app.world().get::<Position>(field).unwrap();
    assert_eq!(
        (at.x, at.y, at.z),
        (destination.x, destination.y, destination.z)
    );
    assert_eq!(
        h.app.world().get::<SpellAreaTrigger>(field).unwrap().caster,
        leymor
    );
    let map = *h.app.world().get::<InMap>(target).unwrap();
    h.app.world_mut().entity_mut(target).insert(InMap {
        instance_id: map.instance_id + 1,
        ..map
    });
    h.advance(0.05);
    // Generic summon ownership cleanup may despawn the departing child; its field
    // must retire either way. Do not assert that a cross-copy probe survives it.
    assert!(h.app.world().get_entity(field).is_err());
    let remaining = find_sprout_fields(&mut h);
    assert_eq!(remaining.len(), 13);
    for binding in initial.into_iter().filter(|(entity, _)| *entity != field) {
        assert!(remaining.contains(&binding));
    }
}
