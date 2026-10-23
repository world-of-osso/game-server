//! Pinned76f8118 Ret selections, validated against native70448 traits.
//! TraitDefinition107509 maps85256 to383328; controller aliases grant no spells.
use super::kit::{Conditions, Entry, Slot};
use super::kit_tests::{at_level, uses_spell};
use super::{Follower, FollowerCatalog, spawning};
use crate::class_progression::KnownSpells;
use crate::spell_cast::test_harness::Harness;
use crate::trait_config::Purchase;
use bevy::prelude::*;
use shared::components::{Health, Player, Position};
use shared::protocol::CombatLogKind;

const TEMPLARS_VERDICT: u32 = 85256;
const FINAL_VERDICT: u32 = 383328;
const TEMPLARS_VERDICT_DAMAGE: u32 = 224266;
const JUDGMENT: u32 = 20271;
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
    let (mut h, actor) = at_level(209057, RET_LEVEL, |_| {});
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
    let (known, traits) = crate::class_progression::evaluate_reference_traits(
        h.app.world().resource::<crate::spell_info::ClassSpells>(),
        &profile,
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

fn earn_finisher_power(h: &mut Harness, actor: Entity) {
    let mut generators = 0;
    for _ in 0..600 {
        match crate::spell_cast::creature_cast(
            h.app.world_mut(),
            actor,
            JUDGMENT,
            Some(h.npc),
            false,
        ) {
            Ok(()) => generators += 1,
            Err(crate::spell_cast::CastFailure::OnCooldown)
            | Err(crate::spell_cast::CastFailure::OnGlobalCooldown)
            | Err(crate::spell_cast::CastFailure::NoCharges) => {}
            Err(error) => panic!("native Ret Judgment generator failed: {error:?}"),
        }
        h.advance(1);
        if generators == 3 {
            return;
        }
    }
    panic!("three native Judgment generators did not finish: {generators}");
}

fn cast_controller_verdict(h: &mut Harness, actor: Entity, expected: u32) {
    h.app.world_mut().resource_mut::<FollowerCatalog>().kits = vec![Entry {
        spec_id: 70,
        spell_id: TEMPLARS_VERDICT,
        min_level: RET_LEVEL,
        max_level: RET_LEVEL,
        slot: Slot::Attack,
        conditions: Conditions::default(),
    }]
    .into();
    earn_finisher_power(h, actor);
    uses_spell(h, actor, expected);
    h.advance(10);
}

fn damage(h: &Harness, actor: Entity, spell: u32) -> i64 {
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::Damage
        })
        .map(|event| i64::from(event.amount))
        .sum()
}

#[test]
#[ignore = "native Ret selected Final Verdict must replace the controller base action"]
fn follower_native_action_override_casts_selected_final_verdict_with_real_damage() {
    let _seed = crate::random::seed_scope(70448);
    let (mut h, actor) = ret_fixture(&selected_ret_purchases());
    let known = h.app.world().get::<KnownSpells>(actor).unwrap();
    assert!(known.contains(FINAL_VERDICT));
    assert!(!known.contains(TEMPLARS_VERDICT));
    assert!(h.app.world().get::<Player>(actor).is_none());
    cast_controller_verdict(&mut h, actor, FINAL_VERDICT);
    assert!(damage(&h, actor, FINAL_VERDICT) > 0);
    assert_eq!(damage(&h, actor, TEMPLARS_VERDICT_DAMAGE), 0);
}

#[test]
#[ignore = "native granted-only Ret book must preserve its unmodified base spender"]
fn follower_native_action_override_keeps_unselected_templars_verdict_damage() {
    let _seed = crate::random::seed_scope(70448);
    let (mut h, actor) = ret_fixture(&[]);
    let known = h.app.world().get::<KnownSpells>(actor).unwrap();
    assert!(known.contains(TEMPLARS_VERDICT));
    assert!(!known.contains(FINAL_VERDICT));
    cast_controller_verdict(&mut h, actor, TEMPLARS_VERDICT);
    assert!(damage(&h, actor, TEMPLARS_VERDICT_DAMAGE) > 0);
    assert_eq!(damage(&h, actor, FINAL_VERDICT), 0);
}
