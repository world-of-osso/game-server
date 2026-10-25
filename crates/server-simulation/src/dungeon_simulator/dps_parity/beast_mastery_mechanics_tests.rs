use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn beast_mastery_pet_damage_reads_native_owner_spell_modifiers() {
    use server::simc_apl::Facts;
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    let bindings = crate::simc_apl_native::Bindings {
        actions: [("pet_command".into(), 83381)].into(),
        ..Default::default()
    };
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: 300.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    let damage = |run: &Run| {
        crate::simc_apl_native::NativeFacts::new(run.app.world(), pet, &bindings, encounter, 1)
            .with_targets(Some(run.targets[0]), &run.targets)
            .value("action.pet_command.damage")
            .unwrap()
    };
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        193455,
        Some(run.targets[0]),
        false,
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let bonus = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(1276720)
        .unwrap()
        .effects[0]
        .base_points;
    let after = damage(&run);
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.actor,
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 1276720,
    );
    let before = damage(&run);
    let expected = before * (1.0 + f64::from(bonus) / 100.0);
    assert!(
        (after - expected).abs() < expected * 0.001,
        "native owner's Nature's Ally amount must modify actual pet damage: before={before}, after={after}, expected={expected}"
    );
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn beast_mastery_native_pet_direct_attacks_can_crit() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    engage(&mut run);
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    for spell in [83381, 49966] {
        for _ in 0..64 {
            crate::spell_cast::cast_triggered(
                run.app.world_mut(),
                pet,
                spell,
                Some(run.targets[0]),
                1,
            );
        }
    }
    let combat = report::combat(run.app.world(), &run.damage_targets, 1.0);
    for spell in [83381, 49966] {
        let action = combat["actions"]
            .as_array()
            .unwrap()
            .iter()
            .find(|action| action["spell_id"] == spell)
            .unwrap();
        assert!(
            action["crit_hits"].as_u64().unwrap() > 0,
            "real pet spells must inherit the owner's native critical chance: {spell}; {action}"
        );
    }
}

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn beast_mastery_barbed_and_cobra_arm_consumable_natures_ally() {
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    engage(&mut run);
    for producer in [217200, 193455] {
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            producer,
            Some(run.targets[0]),
            false,
        )
        .expect("ordinary native producer");
        preparation::finish_pending_setup(&mut run).unwrap();
        assert!(
            crate::auras::has_aura(run.app.world(), run.actor, 1276720),
            "successful native Barbed/Cobra must arm selected Nature's Ally: {producer}"
        );
        if producer == 217200 {
            assert!(
                crate::auras::has_aura(run.app.world(), run.actor, 246152),
                "Nature's Ally must not replace the existing native Barbed Focus hook"
            );
        }
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            34026,
            Some(run.targets[0]),
            false,
        )
        .expect("ordinary Kill Command");
        preparation::finish_pending_setup(&mut run).unwrap();
        assert!(
            !crate::auras::has_aura(run.app.world(), run.actor, 1276720),
            "Kill Command consumes the actual ready buff"
        );
        for _ in 0..200 {
            run.app.update();
        }
    }
    let combat = report::combat(run.app.world(), &run.damage_targets, 20.0);
    assert!(
        combat["actions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|action| action["spell_id"] == 83381
                && ["hits", "misses", "dodges", "parries"]
                    .iter()
                    .map(|key| action[key].as_u64().unwrap())
                    .sum::<u64>()
                    == 2),
        "both admitted Kill Commands must attempt real pet attacks, including native avoidance: {combat}"
    );
}
