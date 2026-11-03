//! Hammer and Anvil is earned by actual Protection Paladin impacts.
use super::*;
use shared::protocol::CombatLogKind;

fn assert_hammer_and_anvil(producer: u32, critical: bool) {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(66, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    let traits = run
        .app
        .world()
        .get::<crate::class_progression::TraitLoadout>(actor)
        .unwrap();
    assert!(
        traits
            .active_talent_spells()
            .any(|talent| talent.spell_id == 433718)
    );
    if producer == 375576 {
        assert!(
            traits
                .active_talent_spells()
                .any(|talent| talent.spell_id == 1271553)
        );
    }
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .crit_pct = 100.0;
    let start = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    if producer == 1241413 {
        assert!(crate::spell_cast::try_cast_triggered(
            run.app.world_mut(),
            actor,
            producer,
            Some(target),
            1
        ));
    } else {
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, producer, Some(target), false)
            .unwrap();
    }
    for _ in 0..20 {
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    if critical {
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(producer)
                    && event.kind == CombatLogKind::Damage
                    && event.crit)
        );
    }
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(433717)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
}

#[test]
#[ignore = "requires canonical world database and MID2 Protection inputs"]
fn judgment_critical_strike_triggers_hammer_and_anvil() {
    assert_hammer_and_anvil(275779, true);
}

#[test]
#[ignore = "requires canonical world database and MID2 Protection inputs"]
fn hammer_of_wrath_critical_strike_triggers_hammer_and_anvil() {
    assert_hammer_and_anvil(1241413, true);
}

#[test]
#[ignore = "requires canonical world database and MID2 Protection inputs"]
fn divine_toll_triggers_resounding_strike() {
    assert_hammer_and_anvil(375576, false);
}
