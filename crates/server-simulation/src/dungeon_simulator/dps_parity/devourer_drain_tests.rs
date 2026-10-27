//! Native Devourer metamorphosis drain.
use super::*;

fn enter_metamorphosis(run: &mut Run) {
    let actor = run.actor;
    // Establish the native ordinary-cast admission aura for this isolated fixture.
    // Triggered casts themselves ignore caster-aura restrictions.
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1213809, Some(actor), 1);
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1213809),
        "native Meta admission setup"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1217605, Some(actor), 1);
    // Effect64 queues its native child; let the actual trigger queue run.
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1217607) {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "native Void Metamorphosis must start"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_metamorphosis_drains_fury_over_time() {
    use shared::components::PowerType;
    let mut run = prepare(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    enter_metamorphosis(&mut run);
    let initial = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    // SimC76f8118 sc_demon_hunter.cpp1424–1478,12462–12500: events drain Fury.
    for _ in 0..40 {
        run.app.update();
    }
    let after = crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury)
        .unwrap()
        .0;
    assert!(
        after < initial,
        "Void Metamorphosis must drain Fury: {initial} -> {after}"
    );
}

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_metamorphosis_fills_fury_on_entry() {
    use shared::components::PowerType;
    let mut run = prepare_literal(1480, 1).expect("native Devourer profile");
    let actor = run.actor;
    engage(&mut run);
    crate::spell_cast::powers::change_power(run.app.world_mut(), actor, PowerType::Fury, -1000.0)
        .expect("native Fury pool");
    enter_metamorphosis(&mut run);
    let (current, maximum) =
        crate::spell_cast::powers::power_value(run.app.world(), actor, PowerType::Fury).unwrap();
    assert!(
        maximum > 0.0 && maximum <= 200.0,
        "native Fury cap: {maximum}"
    );
    // SimC76f8118 sc_demon_hunter.cpp12445–12452 grants 200 Fury on entry, capped by the native pool.
    assert_eq!(current, maximum, "Void Metamorphosis entry must fill Fury");
}
