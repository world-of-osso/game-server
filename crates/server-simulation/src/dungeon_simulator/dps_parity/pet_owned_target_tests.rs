//! Controlled pet damage against its owner's actual target aura.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::effects::factor_capture::FactorCapture;

#[test]
#[ignore = "requires native252 profile and actual controlled ghoul"]
fn unholy_controlled_pet_receives_owner_target_pet_debuff() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(252, 1).unwrap();
    engage(&mut run);
    let pet = crate::hunter_pet::summoned_pet(run.app.world(), run.actor).unwrap();
    let spell = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(91778)
        .unwrap()
        .clone();
    let before =
        crate::auras::damage_taken_multiplier(run.app.world(), run.targets[0], pet, &spell);
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        191587,
        Some(run.targets[0]),
        1
    ));
    let amount = run
        .app
        .world()
        .get::<ActiveAuras>(run.targets[0])
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 191587 && aura.caster == run.actor)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.aura_type == 381)
        .unwrap()
        .amount;
    assert!(
        amount > 0.0,
        "native owner disease must carry its pet target bonus"
    );
    let after = crate::auras::damage_taken_multiplier(run.app.world(), run.targets[0], pet, &spell);
    assert!(
        (after / before - (1.0 + amount / 100.0)).abs() < 0.0001,
        "owned target aura must boost this actual pet: {before} -> {after}"
    );
    run.app.insert_resource(FactorCapture::new(
        pet,
        run.targets[0],
        91778,
        "sweeping_claws",
        1,
    ));
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        pet,
        91778,
        Some(run.targets[0]),
        1
    ));
    for _ in 0..60 {
        run.app.update();
        let capture = run.app.world().resource::<FactorCapture>().export();
        if let Some(hit) = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "direct_amount")
        {
            let observed = hit["applied_multipliers"]
                .as_array()
                .unwrap()
                .iter()
                .find(|factor| factor["factor"] == "target_damage")
                .unwrap()["value"]
                .as_f64()
                .unwrap();
            assert!((observed - f64::from(after)).abs() < 0.0001);
            return;
        }
    }
    panic!("actual controlled pet must hit the afflicted target");
}
