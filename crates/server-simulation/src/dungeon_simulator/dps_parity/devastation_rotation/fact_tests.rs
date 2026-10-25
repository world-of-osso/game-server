//! Literal dot facts must observe the real selected actor's native release aura.
use super::{DevFacts, bindings};
use crate::dungeon_simulator::dps_parity::{engage, literal_rotation, prepare_literal};
use crate::simc_apl_native::Encounter;
use server::simc_apl::Facts;

#[test]
#[ignore = "requires private native DB and exact Devastation profile"]
fn devastation_fire_breath_facts_follow_actual_release() {
    let mut run = prepare_literal(1467, 1).unwrap();
    engage(&mut run);
    let mut bindings = bindings::build();
    let equipment =
        literal_rotation::equipment::Equipment::load(run.app.world(), run.actor, &mut bindings)
            .unwrap();
    let read = |run: &crate::dungeon_simulator::dps_parity::Run, name: &str| {
        DevFacts::new(
            run.app.world(),
            run.actor,
            &bindings,
            &equipment,
            Encounter::Fixture {
                started_at: 0.0,
                duration: 300.0,
                style: crate::simc_apl_native::FightStyle::Patchwerk,
            },
            &run.targets,
        )
        .value(name)
        .unwrap()
    };
    assert_eq!(read(&run, "active_dot.fire_breath_damage"), 0.0);
    crate::spell_cast::creature_cast_empowered_with_targets(
        run.app.world_mut(),
        run.actor,
        357208,
        crate::spell_cast::CastTargets {
            unit: run.actor,
            primary: Some(run.targets[0]),
            destination: None,
        },
        1,
    )
    .unwrap();
    for _ in 0..200 {
        run.app.update();
        if read(&run, "active_dot.fire_breath_damage") == 1.0 {
            assert!(read(&run, "dot.fire_breath_damage.remains") > 0.0);
            return;
        }
    }
    panic!("native Fire Breath release never became an active owned dot");
}
