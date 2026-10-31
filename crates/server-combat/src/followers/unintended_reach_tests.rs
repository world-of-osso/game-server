use super::{FollowerCatalog, kit, kit_tests, party_unintended_reach};
use crate::spell_cast::area_trigger::SpellAreaTrigger;
use crate::spell_cast::targets::Relation;
use shared::components::Position;
use std::collections::HashSet;

#[test]
fn native_consecration_pulse_radius_is_included() {
    let (mut harness, actor) = kit_tests::at_level_unengaged(209057, 32, |_| {});
    harness
        .app
        .world_mut()
        .resource_mut::<FollowerCatalog>()
        .kits = vec![kit::Entry {
        spec_id: 66,
        spell_id: 26573,
        min_level: 32,
        max_level: 32,
        slot: kit::Slot::Aoe,
        conditions: kit::Conditions::default(),
    }]
    .into();
    let reaches = party_unintended_reach(harness.app.world_mut(), &[actor]);
    assert_eq!(reaches[0].1, 8.0);
}

#[test]
fn feral_kit_reach_includes_the_selected_cat_form_action() {
    let (mut harness, actor) = kit_tests::at_level_unengaged(232800, 32, |_| {});
    harness.advance(40);
    harness
        .app
        .world_mut()
        .resource_mut::<FollowerCatalog>()
        .kits = vec![kit::Entry {
        spec_id: 103,
        spell_id: 213764,
        min_level: 32,
        max_level: 32,
        slot: kit::Slot::Aoe,
        conditions: kit::Conditions::default(),
    }]
    .into();
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(harness.app.world(), actor, 213764)
            .unwrap(),
        106785
    );
    let reaches = party_unintended_reach(harness.app.world_mut(), &[actor]);
    assert!(reaches[0].1 >= 8.0);
}

#[test]
fn active_hostile_field_remains_bounded_from_its_owner_after_movement() {
    let (mut harness, actor) = kit_tests::at_level_unengaged(209057, 32, |_| {});
    let world = harness.app.world_mut();
    let origin = *world.get::<Position>(actor).unwrap();
    let map = *world.get::<crate::maps::InMap>(actor).unwrap();
    world.spawn((
        Position {
            x: origin.x + 35.0,
            ..origin
        },
        map,
        SpellAreaTrigger {
            caster: actor,
            spell_id: 43265,
            trigger_spell: 52212,
            radius: 8.0,
            expires_at: None,
            period_ms: 1000,
            next_tick_at: 0.0,
            relation: Relation::Enemy,
            depth: 1,
            cast_item_level: None,
            cast_item_inventory_type: None,
            inside: HashSet::new(),
        },
    ));
    let reaches = party_unintended_reach(world, &[actor]);
    assert!(reaches[0].1 >= 43.0, "{reaches:?}");
}
