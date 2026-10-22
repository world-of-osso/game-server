//! Native cast lifetimes: topping an anchor does not top its real group targets.
use bevy::prelude::*;
use shared::casting::{CastState, CastType};
use shared::components::{Health, Position};
use shared::protocol::CombatLogKind;

use super::healer_triage::cancel_topped_cast;
use crate::spell_cast::test_harness::Harness;

fn native_priest_group() -> (Harness, Entity, [Entity; 5]) {
    let (mut h, _, mut party) = super::kit_abundance_tests::native_party(90);
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[215011]);
    h.app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .extend(native.templates);
    let priest = super::kit_abundance_tests::add_native_friend(&mut h, 215011);
    party[0] = priest;
    super::kit_abundance_tests::join_native_party(&mut h, &party);
    (h, priest, party)
}

fn injure(h: &mut Harness, targets: &[Entity]) {
    for target in targets {
        let mut health = h.app.world_mut().get_mut::<Health>(*target).unwrap();
        health.current = health.max * 0.8;
    }
}

fn advance_near_finish(h: &mut Harness, caster: Entity) {
    for _ in 0..200 {
        let cast = h
            .app
            .world()
            .get::<CastState>(caster)
            .expect("native cast still active");
        assert_eq!(cast.cast_type, CastType::Normal);
        if cast.duration - cast.elapsed <= 0.1 {
            return;
        }
        h.advance(1);
    }
    panic!("native cast did not approach completion");
}

fn top_with_native_heal(h: &mut Harness, caster: Entity, target: Entity) {
    for _ in 0..20 {
        let health = h.app.world().get::<Health>(target).unwrap();
        if health.current == health.max {
            return;
        }
        assert!(crate::spell_cast::try_cast_triggered(
            h.app.world_mut(),
            caster,
            2061,
            Some(target),
            1
        ));
    }
    panic!("real native Flash Heal must top its patient");
}

#[test]
fn group_heal_topped_anchor_keeps_prayer_until_native_cluster_heals() {
    let (mut h, priest, party) = native_priest_group();
    let anchor = party[1];
    injure(&mut h, &party[1..]);
    crate::spell_cast::creature_cast(h.app.world_mut(), priest, 596, Some(anchor), false).unwrap();
    advance_near_finish(&mut h, priest);
    top_with_native_heal(&mut h, priest, anchor);
    assert!(party[2..].iter().all(|unit| {
        let health = h.app.world().get::<Health>(*unit).unwrap();
        health.current < health.max
    }));
    cancel_topped_cast(h.app.world_mut(), priest);
    assert!(
        h.app.world().get::<CastState>(priest).is_some(),
        "full anchor cannot cancel native596 while its actual recipients need healing"
    );
    h.advance(5);
    let healed: std::collections::BTreeSet<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(596)
                && event.source == Some(priest.to_bits())
                && event.amount > event.overflow
        })
        .filter_map(|event| event.target)
        .collect();
    assert!(
        healed.len() >= 3,
        "completed native cast must usefully heal its real cluster"
    );
}

#[test]
fn group_heal_topped_anchor_cancels_when_only_outside_radius_patients_are_injured() {
    let (mut h, priest, party) = native_priest_group();
    let anchor = party[1];
    let radius = super::kit_area::radius_for_spell(h.app.world(), priest, 596).unwrap();
    let center = *h.app.world().get::<Position>(anchor).unwrap();
    for target in &party[2..] {
        h.app.world_mut().entity_mut(*target).insert(Position {
            x: center.x + radius * 2.0,
            ..center
        });
    }
    injure(&mut h, &party[1..]);
    crate::spell_cast::creature_cast(h.app.world_mut(), priest, 596, Some(anchor), false).unwrap();
    advance_near_finish(&mut h, priest);
    top_with_native_heal(&mut h, priest, anchor);
    cancel_topped_cast(h.app.world_mut(), priest);
    assert!(
        h.app.world().get::<CastState>(priest).is_none(),
        "injury outside the native effect radius must not preserve an empty group heal"
    );
}

#[test]
fn group_heal_full_caster_keeps_native_tranquility_channel_for_injured_allies() {
    let (mut h, healer, party) = super::kit_abundance_tests::native_party(70);
    injure(&mut h, &party[1..]);
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 740, Some(healer), false).unwrap();
    cancel_topped_cast(h.app.world_mut(), healer);
    assert!(
        h.app
            .world()
            .get::<CastState>(healer)
            .is_some_and(|cast| cast.cast_type == CastType::Channel && cast.spell_id == 740)
    );
    h.advance(50);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(157982)
                && event.source == Some(healer.to_bits())
                && event.amount > event.overflow)
    );
}

#[test]
fn group_heal_topped_wild_growth_anchor_keeps_native_periodic_recipients() {
    let (mut h, healer, party) = super::kit_abundance_tests::native_party(40);
    let anchor = party[1];
    injure(&mut h, &party[1..]);
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 48438, Some(anchor), false)
        .unwrap();
    if h.app.world().get::<CastState>(healer).is_some() {
        advance_near_finish(&mut h, healer);
    }
    top_with_native_heal(&mut h, healer, anchor);
    cancel_topped_cast(h.app.world_mut(), healer);
    h.advance(50);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(48438)
                && event.source == Some(healer.to_bits())
                && event.target != Some(anchor.to_bits())
                && event.amount > event.overflow)
    );
}
