//! Native Devourer fragment-pickup admission while casting Consume.
use super::*;
use server::soul_fragments::{self, FragmentFilter, FragmentKind};

#[test]
#[ignore = "requires native spec1480 player profile and DB"]
fn devourer_fragment_pickup_is_available_during_real_consume_cast() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_literal(1480, 1).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    for _ in 0..3 {
        soul_fragments::spawn(run.app.world_mut(), actor, FragmentKind::Lesser, false).unwrap();
    }
    for _ in 0..10 {
        run.app.update();
    }
    assert!(soul_fragments::active_count(run.app.world(), actor, FragmentFilter::Lesser) > 0);
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        473662,
        Some(target),
    )
    .expect("real Devourer Consume cast");
    assert_eq!(
        run.app
            .world()
            .get::<shared::casting::CastState>(actor)
            .unwrap()
            .spell_id,
        473662
    );
    let mut bindings = crate::simc_apl_native::Bindings::default();
    let equipment =
        literal_rotation::equipment::Equipment::load(run.app.world(), actor, &mut bindings)
            .unwrap();
    let prepared = literal_rotation::execution::Prepared::read(
        run.app.world_mut(),
        actor,
        &run.targets,
        &equipment,
        std::iter::empty::<literal_rotation::execution::NativeAction<'_>>(),
        false,
    )
    .unwrap();
    let action = server::simc_apl::Action {
        source_row: None,
        name: "pick_up_fragment".into(),
        target: None,
        options: [("use_off_gcd".into(), "1".into())].into(),
    };
    assert!(
        prepared.permits_row(&action).unwrap(),
        "real Consume must allow pickup while casting"
    );
    assert!(
        prepared.execution(&action, &equipment).unwrap().is_some(),
        "active fragment remains selectable"
    );
}
