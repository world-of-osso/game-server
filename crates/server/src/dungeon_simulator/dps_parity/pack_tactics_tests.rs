//! Native321014 description links Barbed217200 to instant Focus child1282660.
//! Real source spells/gear/traits; no assigned resource values or rotation edits.
use super::*;
use crate::auras::RemoveReason;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

const PACK_TACTICS: u32 = 321014;
const PACK_FOCUS: u32 = 1282660;
const BARBED_SHOT: u32 = 217200;
const COBRA_SHOT: u32 = 193455;

fn prepare_player() -> Run {
    let (profile, traits) = read_reference(253).unwrap();
    let run = prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 3 },
    )
    .unwrap();
    assert!(crate::auras::has_aura(
        run.app.world(),
        run.actor,
        PACK_TACTICS
    ));
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(run.actor)
        .unwrap();
    assert!(known.contains(BARBED_SHOT) && known.contains(COBRA_SHOT));
    run
}

fn cast_ready_spell(run: &mut Run, spell: u32) {
    let actor = run.actor;
    let target = run.targets[0];
    for _ in 0..200 {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            actor,
            spell,
            Some(target),
            false,
        ) {
            Ok(()) => return,
            Err(crate::spell_cast::CastFailure::OnGlobalCooldown) => run.app.update(),
            Err(error) => panic!("ordinary native spell{spell}: {error:?}"),
        }
    }
    panic!("native GCD never admitted spell{spell}");
}

fn wait_for_barbed_gcd(run: &mut Run) {
    for _ in 0..200 {
        let world = run.app.world();
        let spell = world
            .resource::<crate::spell_info::SpellStore>()
            .get(BARBED_SHOT)
            .unwrap();
        let now = crate::spell_cast::history_now(world);
        if !world
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .is_some_and(|history| history.on_gcd(spell, now))
        {
            return;
        }
        run.app.update();
    }
    panic!("native Barbed GCD did not recover");
}

fn native_grant(run: &Run) -> i32 {
    run.app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(PACK_FOCUS)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.effect == 30)
        .unwrap()
        .base_points as i32
}

fn assert_immediate_barbed_grant(run: &mut Run) -> i32 {
    wait_for_barbed_gcd(run);
    let actor = run.actor;
    let target = run.targets[0];
    let before =
        crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Focus).unwrap();
    let first = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    let at = crate::spell_cast::history_now(run.app.world());
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, BARBED_SHOT, Some(target), false)
        .unwrap();
    let world = run.app.world();
    assert_eq!(
        crate::spell_cast::history_now(world),
        at,
        "instant reward precedes any frame/DoT tick"
    );
    let grants: Vec<_> = world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[first..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Energize
                && event.source == Some(actor.to_bits())
                && event.target == Some(actor.to_bits())
                && event.spell_id == Some(PACK_FOCUS)
        })
        .collect();
    assert_eq!(
        grants.len(),
        1,
        "ordinary Barbed launch must execute native Pack Focus once"
    );
    assert_eq!(grants[0].amount, native_grant(run));
    let after = crate::spell_cast::powers::power_value(world, actor, PowerType::Focus).unwrap();
    assert_eq!(after.0, (before.0 + native_grant(run) as f32).min(before.1));
    grants[0].overflow
}

#[test]
#[ignore = "native Pack launch/overflow; requires current prepared253 parity inputs"]
fn dps_parity_pack_tactics_native_launch_grants_before_tick_and_preserves_overflow() {
    let mut run = prepare_player();
    let amount = native_grant(&run);
    assert_eq!(
        assert_immediate_barbed_grant(&mut run),
        amount,
        "first full-pool grant overflows"
    );
    cast_ready_spell(&mut run, COBRA_SHOT);
    wait_for_barbed_gcd(&mut run);
    let (current, maximum) =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::Focus)
            .unwrap();
    assert!(
        current < maximum,
        "real Cobra cost must leave native resource room"
    );
    assert!(
        assert_immediate_barbed_grant(&mut run) < amount,
        "second native grant is partly realized"
    );
}

#[test]
#[ignore = "native inactive Pack control; requires current prepared253 parity inputs"]
fn dps_parity_pack_tactics_native_removed_passive_does_not_grant_instant_focus() {
    let mut run = prepare_player();
    let actor = run.actor;
    let removed = crate::auras::remove_auras(
        run.app.world_mut(),
        actor,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == PACK_TACTICS,
    );
    assert_eq!(removed.len(), 1);
    cast_ready_spell(&mut run, BARBED_SHOT);
    let world = run.app.world();
    assert!(!crate::auras::has_aura(world, actor, PACK_TACTICS));
    assert!(
        !world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(
                |event| event.source == Some(actor.to_bits()) && event.spell_id == Some(PACK_FOCUS)
            )
    );
}

#[test]
#[ignore = "native Austin Pack count; requires current prepared253 parity inputs"]
fn dps_parity_pack_tactics_native_dungeon_kit_grants_once_per_barbed_cast() {
    let (profile, traits) = read_reference(253).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    engage(&mut run);
    let (seconds, _, _) = advance_combat(&mut run).unwrap();
    let receipt = report::combat(run.app.world(), &run.damage_targets, seconds);
    let actor = run.actor.to_bits();
    let casts = receipt["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["actor"] == actor && row["spell_id"] == BARBED_SHOT)
        .unwrap()["count"]
        .as_u64()
        .unwrap();
    assert!(casts > 0);
    let gain = receipt["energize"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| {
            row["source"] == actor && row["target"] == actor && row["spell_id"] == PACK_FOCUS
        })
        .expect("native dungeon kit Barbed casts must produce Pack Focus receipts");
    assert_eq!(gain["events"].as_u64().unwrap(), casts);
    assert_eq!(
        gain["amount"].as_i64().unwrap(),
        native_grant(&run) as i64 * casts as i64
    );
    assert!(gain["effective_gain"].as_i64().unwrap() > 0);
}
