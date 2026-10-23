//! Selected native starter books versus controller actions on both paths.
use super::kit::{Conditions, Entry, Slot, Target};
use super::kit_tests::at_level;
use super::{FollowerCatalog, update_ai};
use crate::dungeon_simulator::dps_parity::literal_rotation::execution::knows_action;
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::components::{Health, PowerType};

const RALIA: u32 = 232800;
const LANTHA: u32 = 230861;

fn fixture(entry: u32, level: u8) -> (Harness, Entity) {
    let (mut h, actor) = at_level(entry, level, |_| {});
    h.app
        .world_mut()
        .resource_mut::<FollowerCatalog>()
        .kits
        .clear();
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 1_000_000_000.0,
        max: 1_000_000_000.0,
    });
    h.advance(40);
    if entry == LANTHA {
        let mut pools = h
            .app
            .world_mut()
            .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
            .unwrap();
        let shards = pools.get_mut(PowerType::SoulShards).unwrap();
        shards.current = shards.max;
    }
    (h, actor)
}

fn kit_casts(entry: u32, level: u8, spell: u32, self_target: bool) -> usize {
    let (mut h, actor) = fixture(entry, level);
    let spec = h
        .app
        .world()
        .get::<super::Follower>(actor)
        .unwrap()
        .profile
        .spec_id;
    let slot = if self_target {
        Slot::Buff
    } else {
        Slot::Attack
    };
    let target = if self_target {
        Target::SelfUnit
    } else {
        Target::Enemy
    };
    h.app.world_mut().resource_mut::<FollowerCatalog>().kits = vec![Entry {
        spec_id: spec,
        spell_id: spell,
        min_level: level,
        max_level: level,
        slot,
        conditions: Conditions {
            target,
            ..Default::default()
        },
    }]
    .into();
    for _ in 0..600 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    h.inbox
        .go
        .iter()
        .filter(|event| event.caster == actor.to_bits() && event.spell_id == spell)
        .count()
}

#[test]
#[ignore = "native private DB: selected VisibleSpellID admission"]
fn action_knowledge_literal_selected_visible_feral_berserk() {
    let (h, actor) = fixture(RALIA, 32);
    assert!(knows_action(h.app.world(), actor, 106951).unwrap());
}

#[test]
#[ignore = "native private DB: selected VisibleSpellID admission"]
fn action_knowledge_literal_selected_visible_demonology_hand_of_guldan() {
    let (h, actor) = fixture(LANTHA, 90);
    assert!(knows_action(h.app.world(), actor, 105174).unwrap());
}

#[test]
#[ignore = "native private DB: selected VisibleSpellID admission"]
fn action_knowledge_kit_selected_visible_feral_berserk() {
    assert!(kit_casts(RALIA, 32, 106951, true) > 0);
}

#[test]
#[ignore = "native private DB: selected VisibleSpellID admission"]
fn action_knowledge_kit_selected_visible_demonology_hand_of_guldan() {
    assert!(kit_casts(LANTHA, 90, 105174, false) > 0);
}

#[test]
#[ignore = "native private DB: unselected talent control"]
fn action_knowledge_literal_rejects_unselected_incarnation() {
    let (h, actor) = fixture(RALIA, 32);
    let traits = h
        .app
        .world()
        .get::<crate::class_progression::TraitLoadout>(actor)
        .unwrap();
    assert!(
        traits
            .active_talent_spells()
            .all(|talent| talent.spell_id != 102543)
    );
    assert!(!knows_action(h.app.world(), actor, 102543).unwrap());
}

#[test]
#[ignore = "native private DB: unselected talent control"]
fn action_knowledge_kit_rejects_unselected_incarnation() {
    assert_eq!(kit_casts(RALIA, 32, 102543, true), 0);
}

#[test]
#[ignore = "native private DB: sourced Demoniac action bridge"]
fn action_knowledge_literal_selected_demoniac_grants_demonbolt() {
    let (h, actor) = fixture(LANTHA, 90);
    assert!(knows_action(h.app.world(), actor, 264178).unwrap());
}
