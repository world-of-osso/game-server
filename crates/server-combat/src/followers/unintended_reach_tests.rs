use super::{FollowerActionSpells, kit_tests, party_unintended_reach};
use crate::spell_cast::area_trigger::SpellAreaTrigger;
use crate::spell_cast::targets::Relation;
use shared::components::Position;
use std::collections::BTreeSet;

#[test]
fn holy_paladin_modifier_does_not_inherit_divine_toll_reach() {
    let (harness, actor) = kit_tests::at_level_unengaged(209057, 32, |_| {});
    let world = harness.app.world();
    let modifier = super::spell_unintended_reach(world, actor, 1258016, actor).unwrap();
    let toll = super::spell_unintended_reach(world, actor, 375576, actor).unwrap();
    assert_eq!(modifier, (0.0, 0.0));
    assert_eq!(toll.0, 30.0);
}

#[test]
fn native_trigger_and_periodic_trigger_inherit_child_area_reach() {
    let (harness, actor) = kit_tests::at_level_unengaged(209057, 32, |_| {});
    let world = harness.app.world();
    for spell in [1680, 1949] {
        let reach = super::spell_unintended_reach(world, actor, spell, actor).unwrap();
        assert!(
            reach.0 > 0.0,
            "spell {spell} must include its native child area: {reach:?}"
        );
    }
}

#[test]
fn native_consecration_pulse_radius_is_included() {
    let (mut harness, actor) = kit_tests::at_level_unengaged(209057, 32, |_| {});
    harness
        .app
        .world_mut()
        .entity_mut(actor)
        .insert(FollowerActionSpells(vec![26573]));
    let reaches = party_unintended_reach(harness.app.world_mut(), &[actor]);
    assert_eq!(reaches[0].1, 8.0);
}

#[test]
fn feral_action_reach_includes_the_selected_cat_form_action() {
    let (mut harness, actor) = kit_tests::at_level_unengaged(232800, 32, |_| {});
    harness.advance(40);
    harness
        .app
        .world_mut()
        .entity_mut(actor)
        .insert(FollowerActionSpells(vec![213764]));
    assert_eq!(
        crate::spell_cast::knowledge::resolve_actor_action(harness.app.world(), actor, 213764)
            .unwrap(),
        106785
    );
    let reaches = party_unintended_reach(harness.app.world_mut(), &[actor]);
    assert!(reaches[0].1 >= 8.0);
}

#[test]
fn walking_away_from_a_fixed_hostile_field_does_not_grow_reach() {
    let (mut harness, actor) = kit_tests::at_level_unengaged(209057, 32, |_| {});
    let world = harness.app.world_mut();
    let without_field = party_unintended_reach(world, &[actor]);
    let origin = *world.get::<Position>(actor).unwrap();
    let map = *world.get::<crate::maps::InMap>(actor).unwrap();
    world.spawn((
        origin,
        map,
        SpellAreaTrigger {
            caster: actor,
            primary_target: None,
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
            inside: BTreeSet::new(),
        },
    ));
    let mut reaches = Vec::new();
    for distance in [0.0, 35.0, 70.0] {
        world.get_mut::<Position>(actor).unwrap().x = origin.x + distance;
        reaches.push(party_unintended_reach(world, &[actor]));
    }
    assert!(
        reaches.iter().all(|reach| *reach == without_field),
        "a field left behind must not grow the fight-position reach: {without_field:?} -> {reaches:?}"
    );
}
