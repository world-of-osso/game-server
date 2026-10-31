//! Devourer fact operands against a prepared parity actor.
use super::super::{engage, prepare};
use crate::simc_apl_native::NativeFacts;
use crate::simc_apl_native::{Encounter, FightStyle};
use server::simc_apl::Facts;
use server_follower_apl::devourer_rotation::facts::*;
use server_follower_apl::literal_rotation::equipment::Equipment;

fn fact(
    run: &super::super::Run,
    bindings: &crate::simc_apl_native::Bindings,
    equipment: &Equipment,
    name: &str,
) -> Result<f64, String> {
    let world = run.app.world();
    let actor = run.actor;
    let native = NativeFacts::new(
        world,
        actor,
        bindings,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: FightStyle::Patchwerk,
        },
        1,
    );
    DevourerFacts {
        native,
        equipment,
        world,
        actor,
    }
    .value(name)
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn base_meta_drain_fact_tracks_real_meta_stacks() {
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(actor)
            .is_some_and(|known| known.contains(1247534)),
        "selected Soul Glutton"
    );
    let mut bindings = server_follower_apl::devourer_rotation::bindings::build();
    let equipment =
        Equipment::load(run.app.world_mut(), actor, &mut bindings).expect("native equipment");
    let initial = fact(
        &run,
        &bindings,
        &equipment,
        "void_metamorphosis_base_drain_ps",
    )
    .expect("base drain operand");
    assert!(
        (initial - (16.4 / 0.75)).abs() < 0.001,
        "source initial base drain with selected Soul Glutton: {initial}"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1217605, Some(actor), 1);
    for _ in 0..80 {
        run.app.update();
        if crate::auras::paid_periodic_ticks(run.app.world(), actor, actor, 1217607) > 0 {
            break;
        }
    }
    let ticks = crate::auras::paid_periodic_ticks(run.app.world(), actor, actor, 1217607);
    assert!(ticks > 0, "native Meta periodic drain stack must advance");
    let current = fact(
        &run,
        &bindings,
        &equipment,
        "void_metamorphosis_base_drain_ps",
    )
    .expect("live base drain operand");
    let expected = (15.0 + 1.40 * (0.0775 * ticks as f64).exp()) / 0.75;
    assert!(
        (current - expected).abs() < 0.001,
        "source live base drain: {current} vs {expected}"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn real_retreat_exposes_movement_until_native_duration_expires() {
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    let mut bindings = server_follower_apl::devourer_rotation::bindings::build();
    let equipment =
        Equipment::load(run.app.world_mut(), actor, &mut bindings).expect("native equipment");
    assert_eq!(
        fact(
            &run,
            &bindings,
            &equipment,
            "buff.vengeful_retreat_movement.up"
        )
        .unwrap(),
        0.0,
        "no movement before cast"
    );
    crate::spell_cast::cast_for_player_with_target(run.app.world_mut(), actor, 198793, Some(actor))
        .expect("real Vengeful Retreat");
    assert_eq!(
        fact(
            &run,
            &bindings,
            &equipment,
            "buff.vengeful_retreat_movement.up"
        )
        .unwrap(),
        1.0,
        "movement starts with real cast"
    );
    for _ in 0..30 {
        run.app.update();
    }
    assert_eq!(
        fact(
            &run,
            &bindings,
            &equipment,
            "buff.vengeful_retreat_movement.up"
        )
        .unwrap(),
        0.0,
        "native movement duration expires"
    );
}
