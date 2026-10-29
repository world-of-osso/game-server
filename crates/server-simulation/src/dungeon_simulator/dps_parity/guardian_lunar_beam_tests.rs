//! Lunar Beam earns timed native self-heals from its real cast.
use super::*;
use shared::{components::Health, protocol::CombatLogKind};

#[test]
#[ignore = "requires Guardian MID2 profile and native world data"]
fn guardian_lunar_beam_damages_enemies_in_the_original_ground_area() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    let count = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(414613)
                    && event.source == Some(actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.amount > 0
            })
            .count()
    };
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 204066, Some(target), false)
        .unwrap();
    for _ in 0..45 {
        run.app.update();
    }
    let near = count(&run);
    assert_eq!(near, 2, "two ground pulses must damage the nearby enemy");
    run.app
        .world_mut()
        .get_mut::<shared::components::Position>(target)
        .unwrap()
        .x += 100.0;
    for _ in 0..45 {
        run.app.update();
    }
    assert_eq!(
        count(&run),
        near,
        "ground pulses must not follow the enemy out of the cast area"
    );
}

#[test]
#[ignore = "requires Guardian MID2 profile and native world data"]
fn guardian_lunar_beam_heals_on_each_ground_pulse() {
    let mut run = prepare_registered_player(104, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    engage(&mut run);
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 5487, Some(actor), false).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    {
        let mut h = run.app.world_mut().get_mut::<Health>(actor).unwrap();
        h.current = h.max * 0.1;
    }
    let count = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|e| {
                e.kind == CombatLogKind::Heal
                    && e.spell_id == Some(204069)
                    && e.source == Some(actor.to_bits())
                    && e.target == Some(actor.to_bits())
            })
            .count()
    };
    crate::spell_cast::creature_cast(run.app.world_mut(), actor, 204066, Some(target), false)
        .unwrap();
    for _ in 0..10 {
        run.app.update();
    }
    assert_eq!(count(&run), 0);
    for expected in 1..=11 {
        for _ in 0..20 {
            run.app.update();
        }
        assert_eq!(count(&run), expected);
    }
    for _ in 0..20 {
        run.app.update();
    }
    assert_eq!(count(&run), 11);
}
