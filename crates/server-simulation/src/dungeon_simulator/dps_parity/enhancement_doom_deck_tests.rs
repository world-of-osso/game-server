//! Real MW spending must earn background Doom Winds without pressing its ordinary button.
use super::*;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use server::simc_apl::Facts;
use shared::protocol::CombatLogKind;

pub(super) fn try_cast_native_generator(run: &mut Run, target: Entity) {
    for spell in [470057, 17364] {
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            spell,
            Some(target),
            false,
        ) {
            Ok(()) => return,
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::OnCooldown
                | crate::spell_cast::CastFailure::NoCharges
                | crate::spell_cast::CastFailure::InProgress,
            ) => {}
            Err(reason) => panic!("native generator{spell}: {reason:?}"),
        }
    }
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_real_maelstrom_spending_earns_doom_without_starting_button_cooldown() {
    // SimC76f8118 sc_shaman.cpp11809–11827/13099–13110:
    // selected Ascendance draws three spaced successes/600 actual MW outside its active window.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(run.actor)
            .unwrap()
            .contains(114051)
    );
    engage(&mut run);
    let target = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(target),
    )
    .unwrap();
    let mut spent = 0;
    let mut earned = false;
    for _ in 0..40000 {
        let paid = enhancement_generation_tests::try_spend_native_maelstrom(&mut run, target);
        spent += paid;
        if paid == 0 {
            try_cast_native_generator(&mut run, target);
        }
        run.app.update();
        earned = crate::auras::has_aura(run.app.world(), run.actor, 466772);
        if earned || spent >= 600 {
            break;
        }
    }
    assert!(
        earned || spent >= 600,
        "actual generators must fund a complete600-card deck: spent={spent}"
    );
    assert!(
        earned,
        "real MW spending must earn background Doom within600 cards: spent={spent}"
    );
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 114051),
        "the proc grants Doom, not Ascendance"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 384437),
        "earned Doom owns its real Static Accumulation producer"
    );
    let hits = |run: &Run| {
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.spell_id == Some(469270)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .count()
    };
    let initial_hits = hits(&run);
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        hits(&run) > initial_hits,
        "earned Doom emits actual periodic native damage"
    );
    let bindings = Bindings {
        actions: [("doom_winds".into(), 384352)].into(),
        global_cooldown_spell: 384352,
        ..Default::default()
    };
    let remains = NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 3000.0,
            style: FightStyle::Patchwerk,
        },
        1,
    )
    .value("cooldown.doom_winds.remains")
    .unwrap();
    assert_eq!(
        remains, 0.0,
        "background Doom cannot start the ordinary button cooldown"
    );
}
