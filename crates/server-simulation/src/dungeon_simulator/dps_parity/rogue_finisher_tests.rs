use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native260 profile/traits and player actor"]
fn rogue_finisher_starts_and_extends_native_slice_and_dice() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 51667),
        "native Cut to the Chase must be learned"
    );
    assert!(!crate::auras::has_aura(run.app.world(), run.actor, 315496));
    engage(&mut run);
    let mut previous_expiry: Option<f64> = None;
    for _ in 0..2 {
        // This case measures paid four-point extensions, not the separate
        // maximum-effective-point Fang Strike policy.
        crate::auras::remove_auras(
            run.app.world_mut(),
            run.actor,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 1301405 && aura.caster == run.actor,
        );
        {
            let mut powers = run
                .app
                .world_mut()
                .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
                .unwrap();
            let energy = powers.get_mut(PowerType::Energy).unwrap();
            energy.current = energy.max;
            powers.get_mut(PowerType::ComboPoints).unwrap().current = 4.0;
        }
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                2098,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        assert!(
            events
                .iter()
                .any(|event| event.source == Some(run.actor.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(2098)
                    && event.amount > 0),
            "ordinary Dispatch must hit before the extension boundary"
        );
        let aura = run
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(run.actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 315496 && aura.caster == run.actor)
            .expect("landed native finisher must start Slice and Dice");
        let expiry = aura.expires_at.unwrap();
        if let Some(previous) = previous_expiry {
            assert!(
                (expiry - previous - 12.0_f64).abs() < 0.1,
                "a second four-point finisher extends, rather than replacing, the earned duration"
            );
        } else {
            let remaining = expiry - crate::spell_cast::now(run.app.world());
            assert!(
                (10.0..=12.1).contains(&remaining),
                "native3s times four spent combo points: {remaining}"
            );
        }
        previous_expiry = Some(expiry);
    }
}
