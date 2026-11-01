//! Production follower precombat initialization for Devourer Entropy.
use super::follower_dps_diagnostic::*;
use super::*;
use crate::simc_apl_native::{Encounter, FightStyle};
use server_follower_apl::devourer_rotation::Rotation;

#[test]
#[ignore = "requires FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_devourer_entropy_seeds_precombat_stacks_once() {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").expect("snapshot path");
    let mut app = build_snapshot_app(std::path::Path::new(&path)).unwrap();
    let copy = create_open_follower_copy(&mut app, 2813).unwrap();
    let (mut app, actor, target, _) = prepare_class_follower(app, copy, 1480, 90);
    let encounter = Encounter::Fixture {
        started_at: crate::spell_cast::now(app.world()),
        duration: 120.0,
        style: FightStyle::Patchwerk,
    };
    let mut rotation = Rotation::load(app.world_mut(), actor, encounter).unwrap();
    let count = |world: &bevy::prelude::World| {
        world
            .get::<crate::auras::ActiveAuras>(actor)
            .and_then(|auras| {
                auras
                    .iter()
                    .find(|aura| aura.caster == actor && aura.spell_id == 1225789)
            })
            .map_or(0, |aura| aura.stacks)
    };
    assert_eq!(count(app.world()), 0);
    rotation
        .precombat_step(app.world_mut(), actor, &[target])
        .unwrap();
    assert_eq!(
        count(app.world()),
        25,
        "native Entropy effect2 initializes 25 stacks"
    );
    rotation
        .precombat_step(app.world_mut(), actor, &[target])
        .unwrap();
    assert_eq!(
        count(app.world()),
        25,
        "subsequent polls must not seed again"
    );
}
