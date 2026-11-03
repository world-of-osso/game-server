//! Pinned76f8118 Ret selections, validated against native70448 traits.
//! TraitDefinition107509 maps85256 to383328; controller aliases grant no spells.
use super::kit_tests::at_level;
use super::{Follower, spawning};
use crate::class_progression::KnownSpells;
use crate::spell_cast::test_harness::Harness;
use crate::trait_config::Purchase;
use bevy::prelude::*;
use shared::components::{Health, Position};

const RET_LEVEL: u8 = 90;

#[derive(serde::Deserialize)]
struct Allocation {
    node_id: u32,
    entry_id: u32,
    active_ranks: u32,
}

fn selected_ret_purchases() -> Vec<Purchase> {
    let allocations: Vec<Allocation> = serde_json::from_str(include_str!(
        "../../tests/fixtures/ret_selected_traits_76f8118.json"
    ))
    .unwrap();
    allocations
        .into_iter()
        .filter(|entry| entry.active_ranks > 0)
        .map(|entry| Purchase {
            node_id: entry.node_id,
            entry_id: entry.entry_id,
            ranks: entry.active_ranks,
        })
        .collect()
}

fn ret_fixture(purchases: &[Purchase]) -> (Harness, Entity) {
    let (mut h, actor) = at_level(209057, RET_LEVEL, |app| {
        app.add_plugins(crate::stats::StatsPlugin);
    });
    let mut profile = h
        .app
        .world()
        .get::<Follower>(actor)
        .unwrap()
        .profile
        .clone();
    profile.spec_id = 70;
    profile.role = shared::group::GroupRole::Dps;
    let owner = h.caster;
    spawning::initialize(h.app.world_mut(), owner, actor, profile.clone()).unwrap();
    let identity = shared::components::Player {
        name: String::new(),
        class: profile.class_id,
        race: 1,
        appearance: Default::default(),
    };
    let (known, traits) = crate::class_progression::evaluate_player_reference_traits(
        h.app.world().resource::<crate::spell_info::ClassSpells>(),
        &identity,
        profile.spec_id,
        RET_LEVEL,
        purchases,
    )
    .unwrap();
    h.app.world_mut().entity_mut(actor).insert((known, traits));
    h.app.world_mut().get_mut::<Position>(h.npc).unwrap().x = 1.5;
    // Keep a level90 native generator/spender sequence alive; not a damage modifier.
    h.app.world_mut().entity_mut(h.npc).insert(Health {
        current: 1_000_000_000.0,
        max: 1_000_000_000.0,
    });
    h.advance(3);
    (h, actor)
}

#[test]
#[ignore = "native private DB: selected alias must not expose a passive endpoint"]
fn action_knowledge_passive_endpoint_is_not_a_controller_action() {
    let (h, actor) = ret_fixture(&selected_ret_purchases());
    let world = h.app.world();
    let known = world.get::<KnownSpells>(actor).unwrap();
    assert!(known.contains(404542));
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(404542)
        .unwrap();
    assert!(crate::auras::is_passive(spell));
    let resolve = crate::spell_cast::knowledge::resolve_optional_actor_action;
    assert_eq!(resolve(world, actor, 35395).unwrap(), None);
    assert_eq!(resolve(world, actor, 404542).unwrap(), None);
    assert!(world.get::<KnownSpells>(actor).unwrap().contains(404542));
}
