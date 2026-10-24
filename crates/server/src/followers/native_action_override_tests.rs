//! Pinned76f8118 Ret selections, validated against native70448 traits.
//! TraitDefinition107509 maps85256 to383328; controller aliases grant no spells.
use super::kit::{Conditions, Entry, Slot};
use super::kit_tests::{at_level, uses_spell};
use super::{Follower, FollowerCatalog, spawning};
use crate::class_progression::KnownSpells;
use crate::spell_cast::test_harness::Harness;
use crate::trait_config::Purchase;
use bevy::prelude::*;
use shared::components::{Health, Player, Position, PowerType};
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

fn finisher_power_cost(world: &World, actor: Entity, finisher: u32) -> f32 {
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(finisher)
        .unwrap();
    crate::spell_cast::applicable_costs(world, actor, spell)
        .unwrap()
        .into_iter()
        .filter(|(pool, _, _)| *pool == crate::spell_cast::CostPool::Power(PowerType::HolyPower))
        .map(|(_, amount, _)| amount)
        .sum()
}

fn earn_finisher_power(h: &mut Harness, actor: Entity, finisher: u32) {
    use crate::spell_cast::{CastFailure, native_apl};
    let required = finisher_power_cost(h.app.world(), actor, finisher);
    let judgment = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(JUDGMENT)
        .unwrap()
        .clone();
    let target = h.npc;
    let mut previous = None;
    trace_ret_boundary(h, actor, "before funding");
    for frame in 0..600 {
        let current =
            crate::spell_cast::powers::power_value(h.app.world(), actor, PowerType::HolyPower)
                .unwrap()
                .0;
        let charges =
            native_apl::cooldown_value(h.app.world(), actor, &judgment, "charges").unwrap();
        let world = h.app.world();
        let in_combat = world
            .get::<shared::components::CombatStatus>(actor)
            .is_some_and(|status| status.0);
        let has_target_ref = crate::combat_refs::has_active_reference_to(world, actor, target);
        let state = (current, charges, in_combat, has_target_ref);
        if previous != Some(state) {
            eprintln!(
                "native Ret funding frame{frame}: HolyPower={current}/{required}, Judgment charges={charges}, combat={in_combat}, target_ref={has_target_ref}, refs={:?}",
                world.get::<crate::combat_refs::CombatRefs>(actor)
            );
            previous = Some(state);
        }
        if current >= required {
            return;
        }
        match crate::spell_cast::creature_cast(
            h.app.world_mut(),
            actor,
            JUDGMENT,
            Some(target),
            false,
        ) {
            Ok(()) => {}
            Err(
                CastFailure::OnCooldown
                | CastFailure::OnGlobalCooldown
                | CastFailure::NoCharges
                | CastFailure::InProgress
                | CastFailure::NotEnoughPower(_),
            ) => {}
            Err(reason) => panic!("native Ret Judgment generator failed: {reason:?}"),
        }
        h.advance(1);
    }
    trace_ret_boundary(h, actor, "funding timeout");
    panic!(
        "native Ret generators did not earn required{required} Holy Power within600 frames; last state{previous:?}"
    );
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
    earn_finisher_power(h, actor, expected);
    uses_spell(h, actor, expected);
    h.advance(10);
    trace_ret_boundary(h, actor, "after controller verdict");
}

fn trace_ret_boundary(h: &Harness, actor: Entity, stage: &str) {
    let world = h.app.world();
    let stats = world.get::<crate::stats::CharacterStats>(actor);
    eprintln!(
        "native Ret target: {:?}; actor refs: {:?}",
        world.get::<shared::components::Npc>(h.npc),
        world.get::<crate::combat_refs::CombatRefs>(actor)
    );
    let known = world.get::<KnownSpells>(actor).unwrap();
    eprintln!(
        "native Ret known generators: Judgment={}, CrusaderStrike={}, BladeOfJustice={}",
        known.contains(JUDGMENT),
        known.contains(35395),
        known.contains(184575)
    );
    eprintln!(
        "native Ret {stage}: stats={stats:?}; actor_pos={:?}; target_pos={:?}",
        world.get::<Position>(actor),
        world.get::<Position>(h.npc)
    );
    for event in h.inbox.log.iter().filter(|event| {
        event.source == Some(actor.to_bits())
            && event.spell_id.is_some_and(|id| {
                [
                    JUDGMENT,
                    FINAL_VERDICT,
                    TEMPLARS_VERDICT,
                    TEMPLARS_VERDICT_DAMAGE,
                ]
                .contains(&id)
            })
    }) {
        eprintln!("native Ret event {event:?}");
    }
    for cast in h
        .inbox
        .go
        .iter()
        .filter(|cast| cast.caster == actor.to_bits())
    {
        eprintln!("native Ret go {cast:?}");
    }
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
