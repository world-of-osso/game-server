//! Literal Devourer native buff acquisition and reaction facts.
use super::*;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use server::simc_apl::Facts;

#[test]
#[ignore = "requires DPS_PARITY_WORLD_DB and DPS_PARITY_OUTPUT with spec1480 exact profile/traits"]
fn devourer_literal_tracks_meta_trigger_and_voidfall_reaction() {
    let mut run = prepare_measurement(1480, 1).expect("literal Devourer setup");
    let actor = run.actor;
    engage(&mut run);
    let started_at = crate::spell_cast::now(run.app.world());
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1217605, Some(actor), 1);
    for _ in 0..20 {
        if crate::auras::has_aura(run.app.world(), actor, 1217607) {
            break;
        }
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1217607),
        "native Meta must be acquired"
    );
    let bindings = Bindings {
        buffs: [
            ("metamorphosis".into(), 1217607),
            ("voidfall_spending".into(), 1256302),
        ]
        .into(),
        ..Default::default()
    };
    let encounter = Encounter::Fixture {
        started_at,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let facts = NativeFacts::new(run.app.world(), actor, &bindings, encounter, 1);
    let last = facts
        .value("buff.metamorphosis.last_trigger")
        .expect("native Meta trigger is tracked");
    assert!(
        last >= 0.0,
        "real acquired Meta has encounter-relative trigger time: {last}"
    );
    crate::spell_cast::cast_triggered(run.app.world_mut(), actor, 1256302, Some(actor), 1);
    assert!(
        crate::auras::has_aura(run.app.world(), actor, 1256302),
        "native Voidfall Spending must be acquired"
    );
    for _ in 0..30 {
        run.app.update();
    }
    let facts = NativeFacts::new(run.app.world(), actor, &bindings, encounter, 1);
    assert!(
        facts
            .value("buff.voidfall_spending.react")
            .expect("native Voidfall reaction is tracked")
            >= 1.0,
        "acquired stack reacts within pinned maximum 1.4s"
    );
}
