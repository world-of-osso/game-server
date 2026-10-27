//! Native Zero In must respond to real critical auto-attacks, not ordinary hits.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_zero_in_first_main_hand_hit_requires_a_critical() {
    let mut checked_normal = false;
    let mut checked_critical = false;
    for seed in 1..=16 {
        let _seed = crate::random::seed_scope(seed);
        let mut run = prepare_literal(260, 1).unwrap();
        engage(&mut run);
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
        )
        .unwrap();
        let mut first = None;
        for _ in 0..100 {
            run.app.update();
            let events = &run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0;
            let hits: Vec<_> = events
                .iter()
                .filter(|event| {
                    event.source == Some(run.actor.to_bits())
                        && event.target == Some(run.targets[0].to_bits())
                        && event.spell_id.is_none()
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                })
                .collect();
            if hits.len() == 1 {
                first = Some(hits[0].crit);
                break;
            }
            if hits.len() > 1 {
                break;
            }
        }
        let Some(critical) = first else {
            continue;
        };
        run.app
            .world_mut()
            .entity_mut(run.actor)
            .remove::<crate::combat::AutoAttack>();
        for _ in 0..2 {
            run.app.update();
        }
        let earned = crate::auras::has_aura(run.app.world(), run.actor, 1259486);
        // SimC76f8118 sc_rogue.cpp3422–3430: the actual non-dagger main-hand
        // white hit earns Zero In only on critical results.
        assert_eq!(
            earned, critical,
            "first real main-hand hit critical={critical}"
        );
        checked_critical |= critical;
        checked_normal |= !critical;
        if checked_critical && checked_normal {
            break;
        }
    }
    assert!(
        checked_critical && checked_normal,
        "real native swings must exercise both hit outcomes"
    );
}
