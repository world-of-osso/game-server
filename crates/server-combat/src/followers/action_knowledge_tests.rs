//! Selected native starter books versus controller actions on both paths.
use super::kit_tests::at_level;
use crate::spell_cast::knowledge::resolve_optional_actor_action as resolve_action;
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::components::{Health, PowerType};

const RALIA: u32 = 232800;
const LANTHA: u32 = 230861;
const SHUJA: u32 = 209053;

fn fixture(entry: u32, level: u8) -> (Harness, Entity) {
    let (mut h, actor) = at_level(entry, level, |_| {});
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

#[test]
#[ignore = "native private DB: selected VisibleSpellID admission"]
fn action_knowledge_literal_selected_visible_feral_berserk() {
    let (h, actor) = fixture(RALIA, 32);
    assert_eq!(
        resolve_action(h.app.world(), actor, 106951).unwrap(),
        Some(106951)
    );
}

#[test]
#[ignore = "native private DB: selected VisibleSpellID admission"]
fn action_knowledge_literal_selected_visible_demonology_hand_of_guldan() {
    let (h, actor) = fixture(LANTHA, 90);
    assert_eq!(
        resolve_action(h.app.world(), actor, 105174).unwrap(),
        Some(105174)
    );
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
    assert_eq!(resolve_action(h.app.world(), actor, 102543).unwrap(), None);
}

#[test]
#[ignore = "native private DB: existing Cat Form source override"]
fn action_knowledge_literal_knows_cat_swipe_through_known_base() {
    let (h, actor) = fixture(RALIA, 32);
    assert_eq!(
        resolve_action(h.app.world(), actor, 106785).unwrap(),
        Some(106785)
    );
}

#[test]
#[ignore = "native private DB: sourced Ascendance parent action"]
fn action_knowledge_literal_selected_ascendance_parent_grants_actual_action() {
    let (h, actor) = fixture(SHUJA, 90);
    assert_eq!(
        resolve_action(h.app.world(), actor, 1219480).unwrap(),
        Some(1219480)
    );
}

#[test]
#[ignore = "native private DB: resolve passive talent to visible cast action"]
fn action_knowledge_literal_resolves_passive_berserk_to_visible_cast() {
    let (h, actor) = fixture(RALIA, 32);
    assert_eq!(
        resolve_action(h.app.world(), actor, 343223).unwrap(),
        Some(106951)
    );
}

#[test]
#[ignore = "native private DB: resolve passive talent to visible cast action"]
fn action_knowledge_literal_resolves_passive_hand_of_guldan_to_visible_cast() {
    let (h, actor) = fixture(LANTHA, 90);
    assert_eq!(
        resolve_action(h.app.world(), actor, 1250273).unwrap(),
        Some(105174)
    );
}

#[test]
#[ignore = "native private DB: resolve selected Frenzy override"]
fn action_knowledge_literal_resolves_base_frenzy_to_selected_frantic_frenzy() {
    let (h, actor) = fixture(RALIA, 90);
    assert_eq!(
        resolve_action(h.app.world(), actor, 274837).unwrap(),
        Some(1243807)
    );
}

#[test]
#[ignore = "native private DB: sourced Demoniac action bridge"]
fn action_knowledge_literal_selected_demoniac_grants_demonbolt() {
    let (h, actor) = fixture(LANTHA, 90);
    assert_eq!(
        resolve_action(h.app.world(), actor, 264178).unwrap(),
        Some(264178)
    );
}
