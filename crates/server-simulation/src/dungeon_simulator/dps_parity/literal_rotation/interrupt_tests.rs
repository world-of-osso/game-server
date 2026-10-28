//! Literal interrupt readiness follows the selected target's real cast state.
use super::*;
use execution::NativeAction;

fn check_interrupt(spec: u32, name: &str, spell: u32) {
    let mut run = super::super::prepare_literal(spec, 1).unwrap();
    super::super::engage(&mut run);
    let equipment = Equipment::load(
        run.app.world(),
        run.actor,
        &mut crate::simc_apl_native::Bindings::default(),
    )
    .unwrap();
    let action = Action {
        source_row: None,
        name: name.into(),
        target: None,
        options: Default::default(),
    };
    let native = NativeAction {
        name,
        spell,
        talent: None,
        self_target: false,
    };
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        [native],
        false,
    )
    .unwrap();
    assert!(
        prepared.execution(&action, &equipment).unwrap().is_none(),
        "{name} cannot target an idle enemy"
    );
    run.app
        .world_mut()
        .entity_mut(run.targets[0])
        .insert(shared::casting::CastState::normal(
            116,
            run.actor.to_bits(),
            30.0,
            true,
        ));
    let prepared = Prepared::read(
        run.app.world_mut(),
        run.actor,
        &run.targets,
        &equipment,
        [native],
        false,
    )
    .unwrap();
    let execution = prepared
        .execution(&action, &equipment)
        .unwrap()
        .expect("casting enemy permits interrupt");
    execution::execute(run.app.world_mut(), run.actor, execution).unwrap();
    assert!(
        run.app
            .world()
            .get::<shared::casting::CastState>(run.targets[0])
            .is_none(),
        "native interrupt stops the target cast"
    );
}

#[test]
#[ignore = "requires native 260 profile"]
fn literal_kick_requires_target_casting() {
    check_interrupt(260, "kick", 1766);
}

#[test]
#[ignore = "requires native 255 profile"]
fn literal_muzzle_requires_target_casting() {
    check_interrupt(255, "muzzle", 187707);
}
