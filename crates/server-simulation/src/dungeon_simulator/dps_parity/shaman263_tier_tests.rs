//! Native MID2 set pieces must drive their actual periodic payload and recharge relationship.
use super::*;
use crate::auras::ActiveAuras;
use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
use crate::spell_cast::test_harness::ObservedCombatEvents;
use server::simc_apl::Facts;
use shared::protocol::CombatLogKind;

fn cast(run: &mut Run, spell: u32, target: Entity) {
    crate::spell_cast::creature_cast(run.app.world_mut(), run.actor, spell, Some(target), false)
        .unwrap();
    preparation::finish_pending_setup(run).unwrap();
}

fn crash_recharge(run: &Run) -> f64 {
    let bindings = Bindings {
        actions: [("crash_lightning".into(), 187874)].into(),
        global_cooldown_spell: 187874,
        ..Default::default()
    };
    NativeFacts::new(
        run.app.world(),
        run.actor,
        &bindings,
        Encounter::Fixture {
            started_at: 0.0,
            duration: 300.0,
            style: FightStyle::Patchwerk,
        },
        1,
    )
    .value("cooldown.crash_lightning.full_recharge_time")
    .unwrap()
}

#[test]
#[ignore = "native263 reference inputs and private world database"]
fn shaman263_mid2_burning_core_ticks_fire_nova_and_funds_short_circuit() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_literal(263, 1).unwrap();
    for spell in [1296627, 1296628] {
        assert!(
            crate::auras::has_aura(run.app.world(), run.actor, spell),
            "actual native2065 set eligibility {spell}"
        );
    }
    engage(&mut run);
    let target = run.targets[0];
    cast(&mut run, 187874, target);
    cast(&mut run, 470057, target);
    let core = run
        .app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1299975 && aura.caster == run.actor);
    assert!(
        core.is_some(),
        "real Voltaic Blaze applies this caster's Burning Core, not an invented tick source"
    );
    let before = crash_recharge(&run);
    assert!(before > 0.0, "actual Crash spent its native charge");
    let started = crate::spell_cast::now(run.app.world());
    for _ in 0..60 {
        run.app.update();
    }
    assert!(
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(target.to_bits())
                    && event.spell_id == Some(333977)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            }),
        "actual Burning Core ticks dispatch native Fire Nova damage from its owner"
    );
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1299991),
        "real Fire Nova earns Short Circuit from the equipped four-piece"
    );
    let elapsed = crate::spell_cast::now(run.app.world()) - started;
    assert!(
        crash_recharge(&run) + 0.05 < (before - elapsed).max(0.0),
        "four-piece reduces the running Crash recharge beyond ordinary elapsed time"
    );
}
