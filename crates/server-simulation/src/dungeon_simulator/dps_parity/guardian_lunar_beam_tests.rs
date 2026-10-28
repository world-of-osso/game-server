//! Lunar Beam earns timed native self-heals from its real cast.
use super::*;
use shared::{components::Health, protocol::CombatLogKind};

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
