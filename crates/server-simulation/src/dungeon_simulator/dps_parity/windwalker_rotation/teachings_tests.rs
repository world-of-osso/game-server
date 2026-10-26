//! Real selected Teachings chain and RSK recovery; no grants or forced proc outcomes.
use super::super::*;
use crate::auras::ActiveAuras;
use crate::class_progression::TraitLoadout;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const TALENT: u32 = 116645;
const BUFF: u32 = 202090;
const CHILD: u32 = 228649;
const TP: u32 = 100780;
const BOK: u32 = 100784;
const RSK: u32 = 107428;
const ZENITH: u32 = 1249625;

fn stacks(run: &Run) -> u8 {
    run.app
        .world()
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == BUFF && aura.caster == run.actor)
        .map_or(0, |aura| aura.stacks)
}

fn rsk_remaining(run: &Run) -> f64 {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    use server::simc_apl::Facts;
    let bindings = Bindings {
        actions: [("rising_sun_kick".into(), RSK)].into(),
        global_cooldown_spell: RSK,
        ..Default::default()
    };
    // RSK uses native charge category1518, not an own/category spell timer.
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
    .value("cooldown.rising_sun_kick.full_recharge_time")
    .unwrap()
}

#[test]
#[ignore = "one actual TP enemy impact earns one Teachings stack, not another from self Energize"]
fn literal_ww_native_teachings_one_stack_per_tiger_palm_impact() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(269, 1).unwrap();
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    engage(&mut run);
    assert_eq!(stacks(&run), 0);
    super::tests::cast(&mut run, TP);
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let impacts = events
        .iter()
        .filter(|event| {
            event.source == Some(run.actor.to_bits())
                && event.spell_id == Some(TP)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .count();
    assert_eq!(impacts, 1, "one actual native Tiger Palm damage impact");
    // SimC76f8118 monk883–889: impact(), not its separate self-Energize recipient.
    assert_eq!(stacks(&run), 1, "one stack per actual TP impact");
}

#[test]
#[ignore = "actual selected TP must earn Teachings; BOK consumes it for real child impacts and recovery"]
fn literal_ww_native_teachings_tp_bok_chain_and_child_recovery() {
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_measurement(269, 1).unwrap();
    assert!(
        run.app
            .world()
            .get::<TraitLoadout>(run.actor)
            .unwrap()
            .active_talent_spells()
            .any(|spell| spell.spell_id == TALENT)
    );
    let cap = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(BUFF)
        .unwrap()
        .aura_options
        .as_ref()
        .unwrap()
        .cumulative_aura as u8;
    assert_eq!(
        cap, 4,
        "native earned buff cap, not a fabricated stack count"
    );
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    engage(&mut run);
    let mut earned = false;
    let mut expected_children = 0;
    let mut actual_children = 0;
    let mut primary = None;
    let mut recovered = false;
    for _ in 0..TICKS {
        let before = stacks(&run);
        let cooldown_before = rsk_remaining(&run);
        let offset = run.app.world().resource::<ObservedCombatEvents>().0.len();
        run.app.update();
        let cooldown_after_tick = rsk_remaining(&run);
        let tick_events: Vec<_> = run.app.world().resource::<ObservedCombatEvents>().0[offset..]
            .iter()
            .filter(|event| event.source == Some(run.actor.to_bits()))
            .cloned()
            .collect();
        drive_player(&mut run).unwrap();
        let fresh: Vec<_> = run.app.world().resource::<ObservedCombatEvents>().0[offset..]
            .iter()
            .filter(|event| event.source == Some(run.actor.to_bits()))
            .cloned()
            .collect();
        let landed_tp = fresh.iter().any(|event| {
            event.spell_id == Some(TP) && event.kind == CombatLogKind::Damage && event.amount > 0
        });
        if landed_tp && before < cap {
            assert!(
                stacks(&run) > before,
                "actual landed TP must earn native Teachings202090 before BOK can consume it"
            );
            earned = true;
        }
        let root_bok = fresh
            .iter()
            .find(|event| event.spell_id == Some(BOK) && event.kind == CombatLogKind::CastSuccess);
        if let Some(event) = root_bok {
            if before > 0 {
                assert_eq!(
                    stacks(&run),
                    0,
                    "actual BOK consumes the earned native batch once"
                );
                expected_children += usize::from(before);
                primary = event.target;
            }
        }
        let child_hits: Vec<_> = fresh
            .iter()
            .filter(|event| {
                event.spell_id == Some(CHILD)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .collect();
        for event in &child_hits {
            assert_eq!(
                event.target, primary,
                "Teachings children retain the actual BOK primary"
            );
        }
        actual_children += child_hits.len();
        assert!(
            actual_children <= expected_children,
            "no unearned child damage"
        );
        // Observe recovery before the literal controller can immediately spend
        // the newly reset RSK in this same native frame.
        let tick_child = tick_events.iter().any(|event| {
            event.spell_id == Some(CHILD) && event.kind == CombatLogKind::Damage && event.amount > 0
        });
        let another_reset_source = tick_events.iter().any(|event| {
            event.kind == CombatLogKind::CastSuccess
                && matches!(event.spell_id, Some(BOK | RSK | 467307 | ZENITH))
        });
        let isolated_child = tick_child && !another_reset_source;
        if isolated_child && cooldown_before > 1.0 && cooldown_after_tick == 0.0 {
            recovered = true;
        }
        if earned && expected_children > 0 && actual_children == expected_children && recovered {
            eprintln!("NATIVE_TEACHINGS children={actual_children} child_rsk_reset={recovered}");
            return;
        }
    }
    panic!(
        "native Teachings chain incomplete: earned={earned} expected={expected_children} actual={actual_children} child_reset={recovered}"
    );
}
