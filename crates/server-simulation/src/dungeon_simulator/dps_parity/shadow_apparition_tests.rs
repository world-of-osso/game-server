//! SimC76f8118 sc_priest_shadow.cpp1296/2609–2650/439–445.
//! Real Slam applies VT before its weak Madness generates travelling apparitions.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native258 Player/profile/traits"]
fn shadow_apparition_slam_madness_reaches_each_owned_vt_and_bursts_at_parent() {
    let mut run = prepare_literal(258, 3).unwrap();
    engage_fixture_opponents(&mut run);
    let actor = run.actor;
    let primary = run.targets[0];
    assert!(crate::auras::has_aura(run.app.world(), actor, 341491));
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::Spell(1227280, primary, None),
    )
    .unwrap();
    for _ in 0..200 {
        run.app.update();
    }
    for &target in &run.targets {
        assert!(
            run.app
                .world()
                .get::<ActiveAuras>(target)
                .unwrap()
                .iter()
                .any(|aura| aura.caster == actor && aura.spell_id == 34914),
            "real Slam supplies owned VT"
        );
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(actor.to_bits())
                && event.spell_id == Some(335467)
                && event.amount > 0),
        "real weak Madness source landed"
    );
    for &target in &run.targets {
        let hits = |spells: &[u32]| {
            events
                .iter()
                .filter(|event| {
                    event.source == Some(actor.to_bits())
                        && event.target == Some(target.to_bits())
                        && event.kind == CombatLogKind::Damage
                        && event.amount > 0
                        && event.spell_id.is_some_and(|spell| spells.contains(&spell))
                })
                .count()
        };
        assert_eq!(
            hits(&[413231, 1264176]),
            1,
            "one Madness impact sends one normal/void apparition to owned VT target{target:?}"
        );
        assert_eq!(
            hits(&[1231479]),
            1,
            "only the parent apparition delivers one Shadeburst to nearby target{target:?}"
        );
    }
}
