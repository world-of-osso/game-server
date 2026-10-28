//! Observe the real Primal Storm Elemental's Stormfury damage modifiers.
use super::elemental_mechanics_tests::fixture;
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use shared::components::Npc;

#[test]
#[ignore = "requires private Elemental262 profile and native world data"]
fn elemental_storm_pet_captures_actual_stormfury_factor() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = fixture();
    assert!(crate::spell_cast::try_cast_triggered(
        run.app.world_mut(),
        run.actor,
        191634,
        Some(run.targets[0]),
        1
    ));
    let pet = (0..40)
        .find_map(|_| {
            run.app.update();
            let owner = run.actor;
            run.app
                .world_mut()
                .query::<(Entity, &Npc, &crate::spell_cast::summon::SummonedCreature)>()
                .iter(run.app.world())
                .find(|(_, npc, summon)| npc.template_id == 77942 && summon.owner == owner)
                .map(|(entity, _, _)| entity)
        })
        .expect("real Primal Storm Elemental");
    run.app.insert_resource(FactorCapture::new(
        pet,
        run.targets[0],
        269005,
        "stormfury",
        4,
    ));
    for _ in 0..180 {
        run.app.update();
    }
    let capture = run.app.world().resource::<FactorCapture>().export();
    println!("STORM_PET_CAPTURE {capture}");
    assert!(
        capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .any(|event| event["event"] == "direct_amount"),
        "actual Stormfury damage must be captured"
    );
}
