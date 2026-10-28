//! Observe the real Primal Storm Elemental's Stormfury damage modifiers.
use super::elemental_mechanics_tests::fixture;
use super::*;
use crate::spell_cast::effects::factor_capture::FactorCapture;
use shared::components::Npc;

#[test]
#[ignore = "requires private Elemental262 profile and native world data"]
fn elemental_storm_pet_uses_owner_passives_but_not_owner_mastery() {
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
    let hit = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .find(|event| {
            event["event"] == "result" && event["time"].as_f64().is_some_and(|time| time > 2.5)
        })
        .expect("actual Stormfury damage after its first scheduled tick");
    let factor = hit["factors"]["native_spell_mod_product"].as_f64().unwrap();
    // SimC76f8118 pet.cpp217–228 and sc_shaman.cpp13224: the pet's own
    // mastery is zero. Owner Elemental Shaman, Resonance and Feedback Loop
    // retain their native -10%, +3% and +10% passive effects.
    let expected = 0.9 * 1.03 * 1.1;
    assert!(
        (factor - expected).abs() < 0.001,
        "real Stormfury keeps owner passives without owner mastery: {factor}, expected {expected}"
    );
}
