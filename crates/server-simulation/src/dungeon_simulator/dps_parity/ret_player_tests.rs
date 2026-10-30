//! The measured Ret player uses the pinned literal rotation.
use super::*;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires frozen native 70 player profile and world database"]
fn class_follower_ret_dungeon_apl_casts_in_native_pull() {
    use crate::followers::simulator::ParityActor;
    use crate::simc_apl_native::Encounter;

    let (profile, traits) = read_reference(70).unwrap();
    let race = inputs::read_reference_race(&profile.race).unwrap();
    let mut run = prepare_reference(profile, &traits, 1, ParityActor::Player { race }).unwrap();
    let encounter = Encounter::Dungeon {
        started_at: crate::spell_cast::now(run.app.world()),
    };
    let mut rotation =
        ret_rotation::Rotation::load(run.app.world_mut(), run.actor, encounter).unwrap();
    engage(&mut run);
    for _ in 0..100 {
        rotation
            .drive(run.app.world_mut(), run.actor, &run.targets)
            .unwrap();
        run.app.update();
    }
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && matches!(event.spell_id, Some(20271 | 184575 | 85256 | 255937))),
        "the literal Ret dungeon actor must cast a real rotational spell"
    );
}

#[test]
#[ignore = "requires frozen native 70 player profile"]
fn ret_player_uses_literal_rotation_for_three_targets() {
    let _scope = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(70, 3).unwrap();
    assert!(
        run.app
            .world()
            .get::<shared::components::Player>(run.actor)
            .is_some()
    );
    assert!(run.spec_driver.is_some());
    engage(&mut run);
    advance_combat(&mut run).unwrap();
    assert!(run.player_rotation.is_none());
    let events = &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(53385)
                && event.kind == CombatLogKind::CastSuccess),
        "the pinned multitarget APL executes native Divine Storm"
    );
}
