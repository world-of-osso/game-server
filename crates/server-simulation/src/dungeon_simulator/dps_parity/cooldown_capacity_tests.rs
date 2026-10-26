use super::*;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use server::simc_apl::{Action, ActionFacts, Facts};

#[test]
#[ignore = "requires private native252 profile/traits and world DB"]
fn native_cooldown_capacity_stays_maximum_after_real_charge_spend() {
    let (mut profile, traits) = read_reference(252).unwrap();
    // Omnium rune selection is unrelated to native charge capacity; isolate this API.
    profile.omnium_talents = None;
    let mut run = prepare_literal_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race: 8 },
    )
    .unwrap();
    let bindings = Bindings {
        actions: [("putrefy".into(), 1247378)].into(),
        ..Default::default()
    };
    let encounter = Encounter::Fixture {
        started_at: 0.0,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let action = Action {
        name: "putrefy".into(),
        source_row: None,
        target: None,
        options: Default::default(),
    };
    let read = |run: &Run| {
        let facts = NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 1);
        let maximum = facts
            .value("cooldown.putrefy.max_charges")
            .expect("native maximum charges");
        assert_eq!(facts.value_for(&action, "max_charges").unwrap(), maximum);
        (maximum, facts.value("cooldown.putrefy.charges").unwrap())
    };
    let (maximum, available) = read(&run);
    assert!(maximum >= 2.0);
    assert_eq!(available, maximum);
    literal_rotation::execution::execute(
        run.app.world_mut(),
        run.actor,
        literal_rotation::execution::Execution::Spell(
            1247378,
            run.targets[0],
            Some(run.targets[0]),
        ),
    )
    .unwrap();
    preparation::finish_pending_setup(&mut run).unwrap();
    let (after_maximum, after_available) = read(&run);
    assert_eq!(after_maximum, maximum);
    // sc_death_knight.cpp12242–12249: selected Putrid Echoes may consume both uses.
    assert!(after_available < available);
}
