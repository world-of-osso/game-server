//! Damage parity fixture boundaries shared by all specializations.
use super::*;
use shared::components::{Position, Rotation};

#[test]
#[ignore = "opt-in native parity inputs"]
fn parity_dummy_pack_places_actor_behind_initial_and_replacement_targets() {
    let (profile, traits) = read_reference(70).unwrap();
    let mut run = spawn_reference(
        profile,
        &traits,
        3,
        crate::followers::simulator::ParityActor::Player { race: 3 },
    )
    .unwrap();
    for replacement in [false, true] {
        if replacement {
            for index in 1..=2 {
                let target = spawn_dummy(&mut run.app, run.actor, index).unwrap();
                run.targets.push(target);
            }
        }
        let world = run.app.world();
        let actor = world.get::<Position>(run.actor).unwrap();
        for &target in &run.targets {
            let at = world.get::<Position>(target).unwrap();
            let facing = world.get::<Rotation>(target).unwrap().y - std::f32::consts::FRAC_PI_2;
            let front_dot = (actor.x - at.x) * facing.cos() - (actor.z - at.z) * facing.sin();
            assert!(
                front_dot < 0.0,
                "SimC position=back: actor must be behind dummy, actor={actor:?} target={at:?} facing={facing}"
            );
        }
    }
}
