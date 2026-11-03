//! Reference actors start with their final regenerating-resource capacity.
use super::*;
use shared::components::PowerType;

#[test]
#[ignore = "requires prepared259 native profile and world data"]
fn assassination_reference_actors_start_full_after_loadout_before_pull() {
    use crate::followers::simulator::ParityActor;

    let (profile, traits) = read_reference(259).unwrap();
    let race = inputs::read_reference_race(&profile.race).unwrap();
    for actor in [ParityActor::Follower, ParityActor::Player { race }] {
        let (actor_profile, _) = read_reference(259).unwrap();
        let run = prepare_measurement_reference(actor_profile, &traits, 1, actor, |_| Ok(()))
            .expect("reference Rogue completes native precombat");
        let energy =
            crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::Energy)
                .unwrap();
        assert_eq!(
            energy,
            (250.0, 250.0),
            "post-loadout energy must be full at pull"
        );
        let combo = crate::spell_cast::powers::power_value(
            run.app.world(),
            run.actor,
            PowerType::ComboPoints,
        )
        .unwrap();
        assert_eq!(combo.0, 0.0, "non-regenerating combo points remain empty");
    }

    let mut overridden = profile;
    overridden
        .options
        .insert("rogue.energy".into(), json!("25"));
    let run =
        prepare_measurement_reference(overridden, &traits, 1, ParityActor::Follower, |_| Ok(()))
            .expect("explicit starting energy remains valid");
    let energy =
        crate::spell_cast::powers::power_value(run.app.world(), run.actor, PowerType::Energy)
            .unwrap()
            .0;
    assert!(
        (25.0..250.0).contains(&energy),
        "precombat must preserve explicit starting energy, got {energy}"
    );
}
