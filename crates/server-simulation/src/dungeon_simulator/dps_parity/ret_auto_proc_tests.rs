//! Actual native Player70 swings, not injected proc events or fabricated spell rows.
use super::*;
use crate::spell_cast::{CastFailure, test_harness::ObservedCombatEvents};
use shared::protocol::CombatLogKind;

fn player_in_melee() -> Run {
    let (profile, traits) = read_reference(70).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 3 },
    )
    .unwrap();
    engage(&mut run);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::AutoAttack(run.targets[0]),
    )
    .unwrap();
    run
}

fn applications(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && event.kind == CombatLogKind::AuraApplied
        })
        .count()
}

#[test]
#[ignore = "native70448 Player70 autoattack proc qualification"]
fn ret_auto_proc_art_of_war_earns_buff_and_resets_blade() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = player_in_melee();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 406064));
    for _ in 0..6000 {
        let spell = run
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(184575)
            .unwrap()
            .clone();
        let before = crate::spell_cast::read_native_cast_after_channel(
            run.app.world_mut(),
            run.actor,
            &spell,
            run.targets[0],
        );
        let old_applications = applications(&run, 406086);
        if before.is_ok() {
            match crate::spell_cast::cast_for_player_with_target(
                run.app.world_mut(),
                run.actor,
                184575,
                Some(run.targets[0]),
            ) {
                Ok(())
                | Err(
                    CastFailure::OnGlobalCooldown
                    | CastFailure::NoCharges
                    | CastFailure::InProgress,
                ) => {}
                Err(reason) => panic!("native Blade failed: {reason:?}"),
            }
        }
        run.app.update();
        if applications(&run, 406086) > old_applications && before == Err(CastFailure::NoCharges) {
            assert!(
                crate::spell_cast::read_native_cast_after_channel(
                    run.app.world_mut(),
                    run.actor,
                    &spell,
                    run.targets[0]
                )
                .is_ok(),
                "earned Art of War restores an exhausted Blade charge"
            );
            return;
        }
    }
    panic!("native autoattacks did not earn Art of War while Blade charges were exhausted");
}
