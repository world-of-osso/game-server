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

fn create_periodic_capacity(run: &mut Run, cap: u8) {
    for _ in 0..1200 {
        if weapon_stacks(run) < cap {
            return;
        }
        match crate::spell_cast::creature_cast(
            run.app.world_mut(),
            run.actor,
            188196,
            Some(run.targets[0]),
            false,
        ) {
            Ok(()) => {
                if weapon_stacks(run) < cap {
                    return;
                }
            }
            Err(
                crate::spell_cast::CastFailure::OnGlobalCooldown
                | crate::spell_cast::CastFailure::InProgress,
            ) => {}
            Err(reason) => panic!("native linked-mode capacity cast: {reason:?}"),
        }
        run.app.update();
    }
    panic!("real paid lightning must create room after native refunds");
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
    // Native469270 has MELEE_COMBAT_START (Attributes1=512); its real pulses may start swings.
    // Stop fixture melee only after Doom ends so the remaining producer is isolated.
    run.app
        .world_mut()
        .entity_mut(actor)
        .remove::<crate::combat::AutoAttack>();
    let cap = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 344179)
        .expect("overlapping native modes must fund the MW counter")
        .max_stacks;
    // A real Bolt may refund its paid stacks. Create actual room without changing that chance.
    // Sample accepted casts directly; waiting another full GCD hides the available overlap window.
    create_periodic_capacity(&mut run, cap);
    assert!(
        run.app
            .world()
            .get::<crate::combat::AutoAttack>(actor)
            .is_none()
    );
    let before = weapon_stacks(&run);
    assert!(
        before < cap,
        "real paid lightning must leave room for native Static ticks: before={before}, cap={cap}"
    );
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
