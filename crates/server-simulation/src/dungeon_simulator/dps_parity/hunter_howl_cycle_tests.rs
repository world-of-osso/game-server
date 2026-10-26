use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_howl_combat_clock_cycles_real_wyvern_boar_and_bear() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 3).expect("native literal BM fixture");
    crate::test_data::install_native_collision(&mut run.app).expect("native summon ground and LOS");
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    run.app.update();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471877),
        "combat entry must start the actual native Howl cooldown"
    );
    for (ready, summon) in [
        (471878, Some(1222271)),
        (472324, None),
        (472325, Some(471990)),
    ] {
        let deadline = crate::spell_cast::now(run.app.world()) + 31.0;
        while !crate::auras::has_aura(run.app.world(), run.actor, ready)
            && crate::spell_cast::now(run.app.world()) < deadline
        {
            run.app.update();
        }
        assert!(
            crate::auras::has_aura(run.app.world(), run.actor, ready),
            "the native cooldown must earn the next beast: {ready}"
        );
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            34026,
            Some(run.targets[0]),
            false,
        )
        .expect("ordinary native Kill Command consumes the next beast");
        assert!(!crate::auras::has_aura(run.app.world(), run.actor, ready));
        assert!(crate::auras::has_aura(run.app.world(), run.actor, 471877));
        if let Some(spell) = summon {
            let owner = run.actor;
            assert!(
                run.app
                    .world_mut()
                    .query::<&crate::spell_cast::summon::SummonedCreature>()
                    .iter(run.app.world())
                    .any(|summon| summon.owner == owner && summon.spell_id == spell),
                "consumption must create the actual owned native beast: {spell}"
            );
        } else {
            let combat = report::combat(run.app.world(), &run.damage_targets, 1.0);
            for spell in [471936, 471938] {
                assert!(
                    combat["actions"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|action| action["spell_id"] == spell
                            && action["damage"].as_u64().unwrap() > 0),
                    "native Boar must execute both impact and cleave: {spell}; {combat}"
                );
            }
        }
    }
}
