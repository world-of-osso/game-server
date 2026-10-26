//! Native target-owned duration fact, independently of any specialization driver.
use super::*;
use server::simc_apl::Facts;

#[test]
#[ignore = "requires private267 profile and native world DB"]
fn native_dot_duration_tracks_actual_owned_wither_state() {
    let mut run = prepare_literal(267, 1).unwrap();
    let bindings = crate::simc_apl_native::Bindings {
        buffs: [("wither".into(), 445474)].into(),
        ..Default::default()
    };
    let read_duration = |run: &Run| {
        crate::simc_apl_native::NativeFacts::new(
            run.app.world(),
            run.actor,
            &bindings,
            crate::simc_apl_native::Encounter::Fixture {
                started_at: crate::spell_cast::now(run.app.world()),
                duration: 300.0,
                style: crate::simc_apl_native::FightStyle::Patchwerk,
            },
            1,
        )
        .with_targets(Some(run.targets[0]), &run.targets)
        .value("dot.wither.duration")
        .unwrap()
    };
    assert_eq!(read_duration(&run), 0.0, "inactive DoT has no source state");
    let (caster, target) = (run.actor, run.targets[0]);
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: 445474,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let fresh_ms = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 445474 && aura.caster == caster)
        .unwrap()
        .duration_ms;
    assert!(fresh_ms > 0);
    assert_eq!(read_duration(&run), f64::from(fresh_ms) / 1000.0);
    crate::auras::remove_auras(
        run.app.world_mut(),
        target,
        crate::auras::RemoveReason::Dispelled,
        |aura| aura.spell_id == 445474 && aura.caster == caster,
    );
    assert_eq!(read_duration(&run), 0.0);
}
