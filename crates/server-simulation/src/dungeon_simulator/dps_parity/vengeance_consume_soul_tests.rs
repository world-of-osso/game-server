use super::*;
use server::soul_fragments::{self, FragmentFilter};
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires canonical world database and MID2 Vengeance profile"]
fn lesser_fragment_heals_from_health_and_recent_damage() {
    let _seed = crate::random::seed_scope(70_448);
    let (profile, traits) = read_reference(581).unwrap();
    let identity = inputs::read_actor(&profile).unwrap();
    let mut run = prepare_reference(profile, &traits, 1, identity).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        263642,
        Some(target),
    )
    .unwrap();
    for _ in 0..20 {
        run.app.update();
    }
    assert_eq!(
        soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Lesser),
        2
    );
    let max = run.app.world().get::<Health>(actor).unwrap().max;
    run.app
        .world_mut()
        .get_mut::<Health>(actor)
        .unwrap()
        .current = max * 0.5;
    assert_eq!(
        soul_fragments::consume_active(run.app.world_mut(), actor, FragmentFilter::Lesser, 1, true)
            .unwrap(),
        1
    );
    for _ in 0..100 {
        run.app.update();
    }
    let heals = |run: &Run| {
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| event.spell_id == Some(203794) && event.kind == CombatLogKind::Heal)
            .map(|event| event.amount)
            .collect::<Vec<_>>()
    };
    let first = heals(&run);
    assert_eq!(
        first.len(),
        1,
        "a consumed lesser fragment must deliver its heal"
    );
    assert!(
        first[0] as f32 >= max * 0.01,
        "minimum health branch: {first:?}, max={max}"
    );
    crate::spell_cast::effects::replay_simc_dummy_bleed(
        run.app.world_mut(),
        target,
        actor,
        max * 0.4,
    );
    assert_eq!(
        soul_fragments::consume_active(run.app.world_mut(), actor, FragmentFilter::Lesser, 1, true)
            .unwrap(),
        1
    );
    for _ in 0..100 {
        run.app.update();
    }
    let second = heals(&run);
    assert_eq!(
        second.len(),
        2,
        "second consumed fragment must deliver its heal"
    );
    assert!(
        second[1] > first[0] * 2,
        "recent incoming damage must increase healing: {second:?}"
    );
}
