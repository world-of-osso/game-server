use super::*;

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_rapid_refreezing_ticks_earn_five_icicles_and_enable_spike() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Consumed,
        |aura| matches!(aura.spell_id, 1246832 | 205473 | 1222865),
    );
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: run.actor,
            target: run.actor,
            spell_id: 1310248,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    for _ in 0..40 {
        run.app.update();
    }
    let icicles = run
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.caster == run.actor && aura.spell_id == 205473)
        .map_or(0, |aura| aura.stacks);
    assert_eq!(
        icicles, 5,
        "actual native1s Rapid Refreezing must pay all five200ms Icicles without the natural passive clock"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1222865),
        "the real native counter cap must enable Spike"
    );
}

#[test]
#[ignore = "requires private native70448 DB and exact Frost64 inputs"]
fn frost_mage_native_rapid_refreezing_comes_from_spike_not_unrelated_successes() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(64, 1).unwrap();
    assert!(crate::auras::has_aura(run.app.world(), run.actor, 1296586));
    let mut earned_from_spike = false;
    for _ in 0..30 {
        crate::auras::remove_auras(
            run.app.world_mut(),
            run.actor,
            crate::auras::RemoveReason::Consumed,
            |aura| matches!(aura.spell_id, 1310248 | 205473 | 1222865),
        );
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                116,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        assert!(
            !crate::auras::has_aura(run.app.world(), run.actor, 1310248),
            "a real Frostbolt success must not spend or produce the Spike-only tier proc"
        );
        for spell_id in [205473, 205473, 205473, 205473, 205473, 1222865] {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: run.actor,
                    target: run.actor,
                    spell_id,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        literal_rotation::execution::execute(
            run.app.world_mut(),
            run.actor,
            literal_rotation::execution::Execution::Spell(
                199786,
                run.targets[0],
                Some(run.targets[0]),
            ),
        )
        .unwrap();
        preparation::finish_pending_setup(&mut run).unwrap();
        earned_from_spike |= crate::auras::has_aura(run.app.world(), run.actor, 1310248);
    }
    assert!(
        earned_from_spike,
        "actual native Spike executions must earn the selected tier proc with pinned accumulated RNG"
    );
}
