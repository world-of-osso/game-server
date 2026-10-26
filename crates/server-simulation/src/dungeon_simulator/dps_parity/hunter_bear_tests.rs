use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_earned_bear_rends_the_actual_pack_on_summon() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 3).expect("native literal BM fixture");
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    run.app.update();
    for ready in [471878, 472324, 472325] {
        let deadline = crate::spell_cast::now(run.app.world()) + 31.0;
        while !crate::auras::has_aura(run.app.world(), run.actor, ready)
            && crate::spell_cast::now(run.app.world()) < deadline
        {
            run.app.update();
        }
        assert!(crate::auras::has_aura(run.app.world(), run.actor, ready));
        crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            34026,
            Some(run.targets[0]),
            false,
        )
        .expect("ordinary Kill Command consumes naturally earned beast");
        preparation::finish_pending_setup(&mut run).unwrap();
    }
    let bear = run
        .app
        .world_mut()
        .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
        .iter(run.app.world())
        .find(|(_, summon)| summon.owner == run.actor && summon.spell_id == 471990)
        .map(|(unit, _)| unit)
        .expect("naturally earned Bear must be an actual owned summon");
    // SimC76f8118 sc_hunter.cpp2057–2060/3218–3234: the real Bear
    // immediately executes native Rend Flesh against its target and pack.
    for target in &run.targets {
        assert!(
            run.app
                .world()
                .get::<crate::auras::ActiveAuras>(*target)
                .is_some_and(|auras| auras
                    .iter()
                    .any(|aura| aura.spell_id == 471999 && aura.caster == bear)),
            "actual Bear must apply its native Rend Flesh to each pack target: bear={bear}, target={target}"
        );
    }
    let before = report::combat(run.app.world(), &run.damage_targets, 1.0);
    for _ in 0..65 {
        run.app.update();
    }
    let after = report::combat(run.app.world(), &run.damage_targets, 1.0);
    let damage = |report: &serde_json::Value| -> u64 {
        report["actions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|action| action["spell_id"] == 471999)
            .map(|action| action["damage"].as_u64().unwrap())
            .sum()
    };
    assert!(
        damage(&after) > damage(&before),
        "real native Bear bleed must tick damage"
    );
}
