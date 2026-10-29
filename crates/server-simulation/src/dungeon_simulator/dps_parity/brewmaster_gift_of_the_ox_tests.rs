//! Gift of the Ox comes from real Blackout Kick impacts.
use super::*;
use shared::{components::Health, protocol::CombatLogKind};

fn heals(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| event.spell_id == Some(spell) && event.kind == CombatLogKind::Heal)
        .count()
}

fn earn_orbs(run: &mut Run) {
    let actor = run.actor;
    let target = run.targets[0];
    for _ in 0..6 {
        crate::spell_cast::creature_cast(run.app.world_mut(), actor, 205523, Some(target), false)
            .unwrap();
        for _ in 0..400 {
            run.app.update();
        }
    }
}

#[test]
#[ignore = "requires canonical Brewmaster profile and world database"]
fn brewmaster_gift_orbs_expire_or_are_consumed_by_expel_harm() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(268, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    {
        let mut health = run.app.world_mut().get_mut::<Health>(actor).unwrap();
        health.current = health.max * 0.3;
    }
    earn_orbs(&mut run);
    assert_eq!(heals(&run, 124507), 0);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 322101, Some(actor), false)
        .unwrap();
    for _ in 0..10 {
        run.app.update();
    }
    assert!(heals(&run, 124507) > 0, "Expel Harm consumes earned orbs");
    let expired = heals(&run, 178173);
    for _ in 0..3500 {
        run.app.update();
    }
    assert_eq!(
        heals(&run, 178173),
        expired,
        "consumed orbs must not expire later"
    );

    earn_orbs(&mut run);
    let before = heals(&run, 178173);
    for _ in 0..3500 {
        run.app.update();
    }
    assert!(
        heals(&run, 178173) > before,
        "unconsumed orbs heal when they expire"
    );
}
