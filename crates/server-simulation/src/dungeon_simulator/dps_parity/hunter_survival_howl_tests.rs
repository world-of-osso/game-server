use super::*;

#[test]
#[ignore = "requires selected native255 Pack Leader inputs and world DB"]
fn native_survival_kill_command_consumes_naturally_earned_pack_leader_wyvern() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(255, 1).expect("native literal Survival Pack Leader fixture");
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471876),
        "fixture must select actual Pack Leader Howl, not Sentinel"
    );
    engage(&mut run);
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), run.actor, run.targets[0]);
    run.app.update();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471877),
        "ordinary combat entry must start the native Howl clock"
    );
    let deadline = crate::spell_cast::now(run.app.world()) + 31.0;
    while !crate::auras::has_aura(run.app.world(), run.actor, 471878)
        && crate::spell_cast::now(run.app.world()) < deadline
    {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471878),
        "native clock must naturally earn Wyvern readiness"
    );
    crate::spell_cast::creature_cast(
        run.app.world_mut(),
        run.actor,
        259489,
        Some(run.targets[0]),
        false,
    )
    .expect("ordinary native Survival Kill Command");
    // SimC76f8118 sc_hunter.cpp6604: the shared Kill Command execution
    // consumes Howl for Survival as well as Beast Mastery.
    assert!(
        !crate::auras::has_aura(run.app.world(), run.actor, 471878),
        "real Survival259489 must consume naturally earned Wyvern readiness"
    );
    let owner = run.actor;
    assert!(
        run.app
            .world_mut()
            .query::<&crate::spell_cast::summon::SummonedCreature>()
            .iter(run.app.world())
            .any(|summon| summon.owner == owner && summon.spell_id == 1222271),
        "Survival consumption must create the actual owned Wyvern"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471881),
        "actual Survival Wyvern must earn native Cry"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 471877),
        "consumption must rearm the next real Howl clock"
    );
}
