//! Selected native starter books versus controller actions on both paths.
use super::kit::{Conditions, Entry, Slot, Target};
use super::kit_tests::at_level;
use super::{FollowerCatalog, update_ai};
use crate::spell_cast::knowledge::resolve_optional_actor_action as resolve_action;
use crate::spell_cast::test_harness::Harness;
use bevy::prelude::*;
use shared::components::{Health, PowerType};

const RALIA: u32 = 232800;
const LANTHA: u32 = 230861;
const SHUJA: u32 = 209053;

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

fn kit_casts(entry: u32, level: u8, spell: u32, self_target: bool) -> Vec<u32> {
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
        .filter(|event| event.caster == actor.to_bits())
        .map(|event| event.spell_id)
        .collect()
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
#[ignore = "native private DB: selected VisibleSpellID admission"]
fn action_knowledge_kit_selected_visible_feral_berserk() {
    assert!(kit_casts(RALIA, 32, 106951, true).contains(&106951));
}

#[test]
#[ignore = "native private DB: selected VisibleSpellID admission"]
fn action_knowledge_kit_selected_visible_demonology_hand_of_guldan() {
    assert!(kit_casts(LANTHA, 90, 105174, false).contains(&105174));
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
#[ignore = "native private DB: unselected talent control"]
fn action_knowledge_kit_rejects_unselected_incarnation() {
    assert!(!kit_casts(RALIA, 32, 102543, true).contains(&102543));
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
#[ignore = "native private DB: existing Cat Form source override"]
fn action_knowledge_kit_resolves_known_swipe_to_cat_action() {
    assert!(kit_casts(RALIA, 32, 213764, false).contains(&106785));
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
#[ignore = "native private DB: sourced Ascendance parent action"]
fn action_knowledge_kit_preserves_selected_ascendance_actual_action() {
    assert!(kit_casts(SHUJA, 90, 1219480, true).contains(&1219480));
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
#[ignore = "native private DB: every published row across all11native starter books/90levels"]
fn action_knowledge_every_native_starter_kit_row_has_expected_action_admission() {
    use crate::class_progression::{KnownSpells, evaluate_npc_starter};
    let native = crate::spell_cast::test_harness::world_db().unwrap();
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let catalog = super::catalog::load_catalog(&db).unwrap();
    let mut world = World::new();
    world.insert_resource(native.store.clone());
    let actor = world.spawn_empty().id();
    let mut rejected = Vec::new();
    let mut admitted = 0;
    let mut unavailable_passive = 0;
    for profile in &catalog.profiles {
        for level in 1..=90 {
            let (spells, traits) =
                evaluate_npc_starter(&native.classes, profile.class_id, profile.spec_id, level)
                    .unwrap();
            world.entity_mut(actor).insert((
                KnownSpells::from_baseline(spells.iter().map(|spell| spell.spell_id).collect()),
                traits,
                shared::components::ActiveSpec(profile.spec_id),
            ));
            for row in catalog
                .kits
                .iter()
                .filter(|row| row.available(profile.spec_id, level))
            {
                match resolve_action(&world, actor, row.spell_id).unwrap() {
                    Some(actual) => {
                        assert!(
                            !crate::auras::is_passive(native.store.get(actual).unwrap()),
                            "native controller endpoint{actual} must be active"
                        );
                        admitted += 1;
                    }
                    None if crate::auras::is_passive(native.store.get(row.spell_id).unwrap()) => {
                        unavailable_passive += 1;
                    }
                    None => {
                        rejected.push((profile.entry, profile.spec_id, level, row.spell_id));
                    }
                }
            }
        }
    }
    assert_eq!(catalog.profiles.len(), 11);
    assert!(admitted > 0);
    assert!(unavailable_passive > 0);
    assert!(
        rejected.is_empty(),
        "native sourced kit actions rejected: {rejected:?}"
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
