//! Real overlapping Ascendance and Doom Winds must retain their shared MW producer.
use super::enhancement_generation_tests::cast;
use super::*;
use crate::auras::{self, ActiveAuras};

fn weapon_stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 344179 && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_linked_modes_keep_earning_maelstrom_until_the_last_real_mode_ends() {
    // SimC76f8118 sc_shaman.cpp12840–12860: Doom expiry cannot remove Static while Ascendance is up.
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    cast(&mut run, 114051, actor);
    assert!(auras::has_aura(run.app.world(), actor, 114051));
    assert!(auras::has_aura(run.app.world(), actor, 466772));
    assert!(auras::has_aura(run.app.world(), actor, 384437));
    assert!(
        run.app
            .world()
            .get::<crate::combat::AutoAttack>(actor)
            .is_none()
    );
    for _ in 0..400 {
        if !auras::has_aura(run.app.world(), actor, 466772) {
            break;
        }
        run.app.update();
    }
    assert!(!auras::has_aura(run.app.world(), actor, 466772));
    assert!(auras::has_aura(run.app.world(), actor, 114051));
    assert!(
        auras::has_aura(run.app.world(), actor, 384437),
        "ending Doom Winds cannot remove the live Ascendance-owned Static Accumulation"
    );
    let target = run.targets[0];
    cast(&mut run, 188196, target);
    let before = weapon_stacks(&run);
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        weapon_stacks(&run) > before,
        "retained native Static ticks must still fund real MW without melee"
    );
    for _ in 0..400 {
        if !auras::has_aura(run.app.world(), actor, 114051) {
            break;
        }
        run.app.update();
    }
    assert!(!auras::has_aura(run.app.world(), actor, 114051));
    assert!(!auras::has_aura(run.app.world(), actor, 384437));
}
