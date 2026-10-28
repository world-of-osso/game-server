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
fn enhancement_crackling_stacks_empower_real_doom_damage() {
    fn damage(keep: bool) -> f64 {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(263, 1).unwrap();
        engage(&mut run);
        let actor = run.actor;
        let target = run.targets[0];
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 384352, Some(actor), 1);
        preparation::finish_pending_setup(&mut run).unwrap();
        assert!(auras::has_aura(run.app.world(), actor, 224127));
        if !keep {
            auras::remove_auras(
                run.app.world_mut(),
                actor,
                auras::RemoveReason::Cancelled,
                |aura| aura.spell_id == 224127,
            );
        }
        run.app.insert_resource(
            crate::spell_cast::effects::factor_capture::FactorCapture::new(
                actor, target, 469270, "doom", 1,
            ),
        );
        let _damage_seed = crate::random::seed_scope(818181);
        crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 469270, Some(target), 1);
        preparation::finish_pending_setup(&mut run).unwrap();
        let capture = run
            .app
            .world()
            .resource::<crate::spell_cast::effects::factor_capture::FactorCapture>()
            .export();
        let event = capture["events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|event| event["event"] == "result" && event["spell_id"] == 469270)
            .expect("actual Doom pulse");
        event["factors"]["raw_amount"].as_f64().unwrap()
    }
    let boosted = damage(true);
    let plain = damage(false);
    assert!(
        (boosted / plain - 1.21).abs() < 0.001,
        "two native10% effect products: {boosted}/{plain}"
    );
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn enhancement_doom_earns_independent_feral_and_rolling_crackling_stacks() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    engage(&mut run);
    let actor = run.actor;
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 384352, Some(actor), 1);
    preparation::finish_pending_setup(&mut run).unwrap();
    let aura = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 224127 && aura.caster == actor)
        .expect("actual Doom activation earns Crackling Surge");
    assert_eq!(aura.stacks, 2);
    let mut deadlines = aura.stack_expirations().to_vec();
    deadlines.sort_by(f64::total_cmp);
    assert_eq!(deadlines.len(), 2);
    assert!(
        (deadlines[1] - deadlines[0] - 4.0).abs() < 0.01,
        "8s Feral and 12s Rolling stacks expire independently: {deadlines:?}"
    );
    for _ in 0..400 {
        if crate::spell_cast::now(run.app.world()) > deadlines[0] + 0.1 {
            break;
        }
        run.app.update();
    }
    let remaining = run
        .app
        .world()
        .get::<ActiveAuras>(actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 224127 && aura.caster == actor)
        .expect("Rolling stack remains after Feral expires");
    assert_eq!(remaining.stacks, 1);
    for _ in 0..200 {
        if crate::spell_cast::now(run.app.world()) > deadlines[1] + 0.1 {
            break;
        }
        run.app.update();
    }
    assert!(!auras::has_aura(run.app.world(), actor, 224127));
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
