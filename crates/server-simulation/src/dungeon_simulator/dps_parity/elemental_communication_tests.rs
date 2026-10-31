//! SimC76f8118 sc_shaman.cpp6654–6658/7436–7438/13025–13033:
//! ordinary builder executions draw native Routine Communication's shuffled deck.
use super::elemental_mechanics_tests::cast;
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const COMMUNICATION: u32 = 443445;
const SUMMON: u32 = 445624;

fn count_summons(run: &Run) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(SUMMON)
                && event.kind == CombatLogKind::Summon
        })
        .count()
}

fn native_deck(run: &Run) -> (usize, usize) {
    let aura = super::elemental_mechanics_tests::own_aura(run, COMMUNICATION)
        .expect("selected native Routine Communication");
    let amount = |index| {
        aura.effects
            .iter()
            .find(|effect| effect.index == index)
            .unwrap()
            .amount as usize
    };
    let positive = amount(4);
    let total = amount(5);
    assert_eq!((positive, total), (4, 50), "native70448 deck inputs");
    (positive, total)
}

fn fixture_without_fellowship(targets: usize) -> Run {
    // The separate Fellowship deck can summon on expiry during these 50 casts.
    let (profile, mut traits) = read_reference(262).unwrap();
    traits.retain(|entry| entry.entry_id != 123632);
    let identity = inputs::read_actor(&profile).unwrap();
    let mut run = prepare_literal_reference(profile, &traits, targets, identity).unwrap();
    engage(&mut run);
    run
}

fn assert_native_builder_deck(mut run: Run, spell: u32) {
    let (positive, total) = native_deck(&run);
    let traits = run
        .app
        .world()
        .get::<crate::class_progression::TraitLoadout>(run.actor)
        .expect("native actor trait loadout");
    assert!(
        traits
            .active_talent_spells()
            .any(|entry| entry.spell_id == COMMUNICATION),
        "Communication must be selected, not merely present as an aura"
    );
    run.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    // Delayed Overloads and other full-mask background children must not draw.
    for _ in 0..total {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(run.targets[0]),
            1,
        ));
    }
    assert_eq!(count_summons(&run), 0, "background builders never summon");
    for _ in 0..total {
        cast(&mut run, spell);
        let recovery_ticks = match spell {
            188389 => 120,
            470057 => 200,
            _ => 0,
        };
        for _ in 0..recovery_ticks {
            run.app.update();
        }
    }
    assert_eq!(
        count_summons(&run),
        positive * if spell == 470057 { run.targets.len() } else { 1 },
        "each eligible execution or Voltaic target draws once from the native deck"
    );
    if matches!(spell, 188389 | 470057) {
        // This test checks the deck; separate tests cover Ancestor echoes.
        return;
    }
    let hits = run
        .app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.spell_id == Some(447419) || event.spell_id == Some(447425))
        .filter(|event| event.kind == CombatLogKind::Damage && event.amount > 0)
        .count();
    assert!(
        hits > 0,
        "earned Ancestors must execute native damage, not only summon logs"
    );
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_communication_native_bolt_executions_draw_four_of_fifty() {
    let _seed = crate::random::seed_scope(70448);
    assert_native_builder_deck(fixture_without_fellowship(1), 188196);
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_communication_voltaic_draws_for_each_available_target() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture_without_fellowship(3);
    let owner = fixture_player(&run);
    for index in 3..7 {
        let target = spawn_dummy(&mut run.app, run.actor, index).unwrap();
        crate::combat_refs::set_in_combat_with(run.app.world_mut(), owner, target);
        run.targets.push(target);
    }
    assert_native_builder_deck(run, 470057);
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_communication_ascendance_draws_for_each_entry_target() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture_without_fellowship(3);
    let (positive, total) = native_deck(&run);
    let before = count_summons(&run);
    for _ in 0..total {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            run.actor,
            1219480,
            Some(run.actor),
            1,
        ));
    }
    assert_eq!(
        count_summons(&run) - before,
        positive * 3,
        "each of three eligible entry targets draws from the native deck"
    );
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_communication_native_flame_shock_draws_four_of_fifty() {
    let _seed = crate::random::seed_scope(70448);
    assert_native_builder_deck(fixture_without_fellowship(1), 188389);
}

#[test]
#[ignore = "requires private262 native DB/profile"]
fn elemental_communication_native_chain_draws_once_per_cast_not_recipient() {
    let _seed = crate::random::seed_scope(70448);
    assert_native_builder_deck(fixture_without_fellowship(3), 188443);
}
