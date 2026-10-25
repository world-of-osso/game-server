//! Real source-driven FoF and Zenith children, without granting state.
use super::*;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

type Run = super::super::Run;
const FOF: u32 = 113656;
const FOF_DAMAGE: u32 = 117418;
const JADEFIRE_DAMAGE: u32 = 1248815;
const ZENITH: u32 = 1249625;
const STOMP: u32 = 1272696;
const RSK: u32 = 107428;

fn prepare() -> Run {
    let mut run = super::super::prepare_measurement(269, 1).unwrap();
    let started = run.app.world().resource::<Time>().elapsed();
    super::super::install_fixture_lifetimes(&mut run, started);
    super::super::engage(&mut run);
    for _ in 0..3 {
        super::tests::cast(&mut run, 100780);
    }
    run
}

fn hits(run: &Run, spell: u32) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(spell)
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
        .count()
}

fn rsk_cooldown(run: &Run) -> f64 {
    let world = run.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(RSK)
        .unwrap();
    crate::spell_cast::cooldown_query::cooldown_value(world, run.actor, spell, "remains").unwrap()
}

#[test]
#[ignore = "actual FoF dispatches MID2 extra damage and selected Jadefire at channel end"]
fn literal_ww_native_source_children_fof_mid2_and_jadefire() {
    let mut run = prepare();
    assert!(
        crate::auras::has_aura(run.app.world(), run.actor, 1296621),
        "real equipped MID2 two-piece"
    );
    let before = hits(&run, FOF_DAMAGE);
    super::tests::cast(&mut run, FOF);
    for _ in 0..200 {
        if !crate::auras::has_aura(run.app.world(), run.actor, FOF) {
            break;
        }
        run.app.update();
    }
    assert!(
        hits(&run, JADEFIRE_DAMAGE) > 0,
        "selected Jadefire must actually damage after FoF"
    );
    assert_eq!(
        hits(&run, FOF_DAMAGE) - before,
        6,
        "native five ticks plus one actual MID2 child"
    );
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        !events
            .iter()
            .any(|event| event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(FOF)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "SimC FoF root has no separate direct AP damage"
    );
}

#[test]
#[ignore = "actual Zenith starts native Stomp damage and resets the spent RSK cooldown"]
fn literal_ww_native_source_children_zenith_stomp_and_rsk_reset() {
    let mut run = prepare();
    super::tests::cast(&mut run, RSK);
    assert!(
        rsk_cooldown(&run) > 1.0,
        "ordinary RSK really spends its cooldown"
    );
    super::tests::cast(&mut run, ZENITH);
    assert!(
        hits(&run, STOMP) > 0,
        "Zenith must actually dispatch its selected Stomp child"
    );
    assert!(
        rsk_cooldown(&run) <= 0.002,
        "Zenith resets actual RSK recovery"
    );
}
