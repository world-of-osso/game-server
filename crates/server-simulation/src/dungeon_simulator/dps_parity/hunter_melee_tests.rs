use super::*;

#[test]
#[ignore = "requires private spec253 reference inputs and native world DB"]
fn native_bm_pet_white_swings_read_live_owner_critical_chance() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(253, 1).expect("native literal BM fixture");
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    run.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .crit_pct = 100.0;
    engage(&mut run);
    crate::pet_control::order_companion_pet(run.app.world_mut(), run.actor, Some(run.targets[0]));
    for _ in 0..200 {
        run.app.update();
    }
    let combat = report::combat(run.app.world(), &run.damage_targets, 10.0);
    let swings = combat["actions"]
        .as_array()
        .unwrap()
        .iter()
        .find(|action| action["actor"] == pet.to_bits() && action["spell_id"].is_null())
        .expect("the main pet must make real white swings");
    assert!(
        swings["crit_hits"].as_u64().unwrap() > 0,
        "owner's full native crit must reach pet melee: {swings}"
    );
    assert_eq!(
        swings["noncrit_hits"], 0,
        "pet white swings must inherit the current owner's critical chance, not the creature's fixed crit: {swings}"
    );
}
