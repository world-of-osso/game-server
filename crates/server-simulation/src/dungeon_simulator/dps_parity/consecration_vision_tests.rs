use super::*;
use shared::protocol::CombatLogKind;

fn first_consecration_tick(targets: usize, selected: bool) -> i32 {
    let _seed = crate::random::seed_scope(70448);
    let (profile, traits) = read_reference(66).unwrap();
    let mut run = spawn_reference(
        profile,
        &traits,
        targets,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    run.app
        .world_mut()
        .entity_mut(run.actor)
        .insert(literal_rotation::LiteralParityFixture);
    for _ in 0..4 {
        run.app.update();
    }
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1245354));
    if !selected {
        crate::auras::remove_auras(
            run.app.world_mut(),
            run.actor,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 1245354,
        );
    }
    let target = run.targets[0];
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        26573,
        Some(run.actor),
        false,
    )
    .unwrap();
    for _ in 0..80 {
        run.app.update();
        let events = &run
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        if let Some(event) = events.iter().find(|event| {
            event.source == Some(run.actor.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(81297)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }) {
            return event.amount;
        }
    }
    panic!("native Consecration did not damage the target");
}

#[test]
#[ignore = "requires native Protection reference profile and world DB"]
fn selected_vision_doubles_only_a_single_target_consecration_pulse() {
    let single_base = first_consecration_tick(1, false);
    let single_selected = first_consecration_tick(1, true);
    let multi_base = first_consecration_tick(2, false);
    let multi_selected = first_consecration_tick(2, true);
    assert!(
        (single_selected - 2 * single_base).abs() <= 2,
        "single target: {single_base} -> {single_selected}"
    );
    assert!(
        (multi_selected - multi_base).abs() <= 1,
        "two targets: {multi_base} -> {multi_selected}"
    );
}
