//! Real Momentum damage/channel/attack-speed behavior, without state grants.
use super::*;
use crate::auras::{ActiveAuras, AuraInstance};
use crate::spell_cast::effects::factor_capture::FactorCapture;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const MOMENTUM: u32 = 451294;
const DAMAGE_BUFF: u32 = 451297;
const SPEED_BUFF: u32 = 451298;
const FOF: u32 = 113656;
const TICK: u32 = 117418;

type Run = super::super::Run;

fn owned_aura(run: &Run, spell: u32) -> Option<AuraInstance> {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)?
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == run.actor)
        .cloned()
}

fn tick_hits(run: &Run) -> usize {
    run.app
        .world()
        .resource::<ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(TICK)
                && event.kind == CombatLogKind::Damage
        })
        .count()
}

#[test]
#[ignore = "native Momentum: FoF-only stacks, haste damage, end reset and60-percent8s speed"]
fn literal_ww_native_momentum_matches_simc_damage_and_end_behavior() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::prepare_measurement(269, 1).unwrap();
    let started = run.app.world().resource::<Time>().elapsed();
    super::super::install_fixture_lifetimes(&mut run, started);
    super::super::engage(&mut run);
    assert!(owned_aura(&run, MOMENTUM).is_some());
    super::tests::cast(&mut run, 100780);
    assert!(
        owned_aura(&run, DAMAGE_BUFF).is_none(),
        "TP damage must not earn FoF-only Momentum stacks"
    );
    for _ in 0..2 {
        super::tests::cast(&mut run, 100780);
    }
    let haste = run
        .app
        .world()
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap()
        .haste_pct;
    let expected_haste = 1.0 + f64::from(haste) / 100.0;
    assert!(haste > 0.0, "actual frozen gear supplies haste");
    let before = tick_hits(&run);
    run.app.insert_resource(FactorCapture::new(
        run.actor,
        run.targets[0],
        TICK,
        "momentum",
        10,
    ));
    super::tests::cast(&mut run, FOF);
    let mut observed = 0;
    for _ in 0..200 {
        if owned_aura(&run, FOF).is_none() {
            break;
        }
        let hits = tick_hits(&run) - before;
        if hits > 0 {
            let buff = owned_aura(&run, DAMAGE_BUFF).expect("real FoF damage earns451297");
            assert_eq!(usize::from(buff.stacks), hits.min(10));
            assert_eq!(
                buff.effects
                    .iter()
                    .find(|effect| effect.index == 0)
                    .unwrap()
                    .amount,
                10.0
            );
            observed = hits;
        }
        run.app.update();
    }
    assert!(observed > 0 && owned_aura(&run, FOF).is_none());
    assert!(
        owned_aura(&run, DAMAGE_BUFF).is_none(),
        "FoF end clears its stacks after the final tick"
    );
    let speed = owned_aura(&run, SPEED_BUFF).expect("FoF end grants the real native speed aura");
    assert_eq!(speed.duration_ms, 8000);
    let effect = speed
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap();
    assert_eq!((effect.aura_type, effect.amount), (342, 60.0));
    let capture = run.app.world().resource::<FactorCapture>().export();
    let rows: Vec<_> = capture["events"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["event"] == "result" && row["spell_id"] == TICK)
        .collect();
    assert!(!rows.is_empty(), "actual FoF damage factor observations");
    for row in rows {
        let actual = row["factors"]["native_script_product"].as_f64().unwrap();
        assert!(
            (actual - expected_haste).abs() < 0.00001,
            "FoF haste component actual={actual} expected={expected_haste}"
        );
    }
    for _ in 0..170 {
        run.app.update();
    }
    assert!(
        owned_aura(&run, SPEED_BUFF).is_none(),
        "native8s buff expires without refresh"
    );
}
