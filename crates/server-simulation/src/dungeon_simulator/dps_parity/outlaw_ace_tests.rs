//! Ace Up Your Sleeve must be earned by real Between the Eyes hits.
use super::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires private native260 player profile and traits"]
fn outlaw_ace_up_your_sleeve_earns_points_and_resets_bte() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(260, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 381828));
    engage(&mut run);
    let mut earned = false;
    for _ in 0..32 {
        {
            let mut powers = run
                .app
                .world_mut()
                .get_mut::<crate::spell_cast::powers::PowerPools>(run.actor)
                .unwrap();
            let energy = powers.get_mut(PowerType::Energy).unwrap();
            energy.current = energy.max;
            powers.get_mut(PowerType::ComboPoints).unwrap().current = 7.0;
        }
        run.app
            .world_mut()
            .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .clear();
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                315341,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        for _ in 0..10 {
            run.app.update();
        }
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        earned = events.iter().any(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(394120)
                && event.kind == CombatLogKind::Energize
        });
        let spell = run
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(315341)
            .unwrap();
        let remaining = run
            .app
            .world()
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .unwrap()
            .remaining_cooldown_ms(spell, crate::spell_cast::now(run.app.world()));
        if earned {
            assert_eq!(
                remaining, 0,
                "earned Ace must reset the running native cooldown"
            );
            break;
        }
        for _ in 0..((remaining as f64 / 50.0).ceil() as usize + 1) {
            run.app.update();
        }
    }
    // SimC76f8118 sc_rogue.cpp3852–3859: a landed BtE rolls the native
    // chance per effective point, then pays native394120 and resets BtE.
    assert!(
        earned,
        "real Between the Eyes must eventually earn Ace Up Your Sleeve"
    );
}
