//! Literal Feral lifecycle must retain the selected actor and native precombat.
use super::*;

#[test]
#[ignore = "requires spec103 profile/traits and native private DB"]
fn feral_literal_clearcasting_reaction_observes_real_proc_before_aoe_decision() {
    use server::simc_apl::Facts;
    let _seed = crate::random::seed_scope(103135700);
    let mut run = prepare_measurement(103, 3).unwrap();
    let actor = run.actor;
    let target = run.targets[0];
    literal_rotation::execution::execute(
        run.app.world_mut(),
        actor,
        literal_rotation::execution::Execution::AutoAttack(target),
    )
    .unwrap();
    for _ in 0..6000 {
        run.app.update();
        if crate::auras::has_aura(run.app.world(), actor, 135700) {
            break;
        }
    }
    let aura = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 135700 && aura.caster == actor)
        .expect("native white swings earn actual Clearcasting");
    let stacks = f64::from(aura.stacks);
    let acquired = crate::spell_cast::now(run.app.world());
    run.app
        .world_mut()
        .entity_mut(actor)
        .remove::<crate::combat::AutoAttack>();
    let bindings = crate::simc_apl_native::Bindings {
        buffs: [("clearcasting".into(), 135700)].into(),
        ..Default::default()
    };
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: acquired,
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let read = |run: &Run| {
        crate::simc_apl_native::NativeFacts::new(run.app.world(), actor, &bindings, encounter, 3)
            .value("buff.clearcasting.react")
            .unwrap()
    };
    assert_eq!(read(&run), 0.0, "a new proc is not immediately reacted to");
    while crate::spell_cast::now(run.app.world()) <= acquired + 1.4 {
        run.app.update();
    }
    assert_eq!(
        read(&run),
        stacks,
        "the pinned103 reaction deadline was observed at acquisition"
    );
}

#[test]
#[ignore = "requires spec103 profile/traits and native private DB"]
fn feral_literal_player_precombat_owns_cat_prowl_and_controller() {
    use shared::components::Player;
    for count in [1, 3] {
        let run = prepare_measurement(103, count).expect("exact selected Feral fixture");
        assert!(run.app.world().get::<Player>(run.actor).is_some());
        let driver = run.spec_driver.as_ref().expect("literal Feral controller");
        assert_eq!(driver.label(), "LITERAL_SIMC_FERAL_76f8118");
        assert!(
            run.player_rotation.is_none(),
            "no AssistedCombat replacement"
        );
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 768));
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 5215));
        assert!(driver.report()["native_precombat_snapshot"].is_object());
    }
}
