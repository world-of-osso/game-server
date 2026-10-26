//! Selected Stormflurry must produce delayed real weapon children without another button press.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

fn strike_hits(run: &Run) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && matches!(event.spell_id, Some(32175 | 32176))
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count()
}

fn cast_strike_observing_delay(run: &mut Run, target: Entity) {
    for _ in 0..1200 {
        let before = strike_hits(run);
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            17364,
            Some(target),
            false,
        ) {
            Ok(()) => {
                assert!(
                    strike_hits(run) <= before + 2,
                    "Stormflurry cannot emit extra weapon hits synchronously with its producing root"
                );
                preparation::finish_pending_setup(run).unwrap();
                return;
            }
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::OnCooldown
                | crate::spell_cast::CastFailure::NoCharges
                | crate::spell_cast::CastFailure::InProgress,
            ) => run.app.update(),
            Err(reason) => panic!("Stormflurry native root: {reason:?}"),
        }
    }
    panic!("Stormflurry fixture Strike did not become ready");
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_stormflurry_earns_delayed_weapon_repeats_without_extra_button_casts() {
    // SimC76f8118 sc_shaman.cpp12041–12073: roll selected native344357 effect0,
    // schedule a200±25ms background Strike; both equipped weapons execute again.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(344357)
    );
    engage(&mut run);
    let target = run.targets[0];
    let mut buttons = 0;
    for _ in 0..40 {
        cast_strike_observing_delay(&mut run, target);
        buttons += 1;
        if strike_hits(&run) > 2 * buttons {
            break;
        }
    }
    let hits = strike_hits(&run);
    assert!(
        hits > 2 * buttons,
        "actual Stormstrike must earn extra native weapon children: buttons={buttons}, weapon_hits={hits}"
    );
    assert_eq!(
        run.app
            .world()
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .unwrap()
            .previous_gcd_cast(1),
        Some(17364),
        "the last foreground action remains its real Strike button"
    );
}
