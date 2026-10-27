//! Native Devourer Soul Immolation tick relationship.
use super::*;
use shared::{components::PowerType, protocol::CombatLogKind};

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_soul_immolation_ticks_grant_native_fury() {
    let mut run = prepare_literal(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Fury, -1000.0)
        .expect("native Fury pool");
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        1241937,
        Some(actor),
    )
    .expect("native Soul Immolation cast");
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1241937),
        "native heal-over-time setup"
    );
    // SimC76f8118 sc_demon_hunter.cpp5930–5958: the native HoT tick energizes via1242475.
    for _ in 0..40 {
        run.app.update();
    }
    let fury = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    assert!(
        run.app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(actor.to_bits())
                    && event.spell_id == Some(1242475)
                    && event.kind == CombatLogKind::Energize
                    && event.amount > 0
            }),
        "Soul Immolation ticks must execute native Fury payload; Fury={fury}"
    );
    assert!(
        fury > 0.0,
        "Soul Immolation must increase the native Fury pool"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_soul_immolation_periodic_ticks_spawn_lesser_fragment() {
    let mut run = prepare_literal(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    crate::spell_cast::cast_for_player_with_target(
        run.app.world_mut(),
        actor,
        1241937,
        Some(actor),
    )
    .expect("native Soul Immolation cast");
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1241937),
        "native heal-over-time active"
    );
    for _ in 0..80 {
        run.app.update();
    }
    let ticks = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(actor.to_bits())
                && event.spell_id == Some(1242475)
                && event.kind == CombatLogKind::Energize
        })
        .count();
    assert!(
        ticks >= 2,
        "native Soul Immolation produced at least two ticks; got {ticks}"
    );
    assert!(
        server::soul_fragments::active_count(
            run.app.world(),
            actor,
            server::soul_fragments::FragmentFilter::Lesser
        ) > 0,
        "Soul Immolation ticks must spawn a lesser fragment"
    );
}
