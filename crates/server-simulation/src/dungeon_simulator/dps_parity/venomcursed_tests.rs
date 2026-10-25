//! Native equipped Venomcursed armor bonuses; actual attacks and proc metadata.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::protocol::CombatLogKind;

const DRIVERS: [u32; 3] = [1307906, 1307923, 1307928];

#[test]
#[ignore = "native269 BIS wears actual crit/mastery/haste Venomcursed bonus lists"]
fn simc_apl_venomcursed_native_all_three_gear_stat_procs() {
    let _seed = crate::random::seed_scope(70_448);
    let mut run = prepare_measurement(269, 1).unwrap();
    let actor = run.actor;
    let started = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, started);
    let expected = DRIVERS.map(|id| {
        let parent = run.app.world().get::<ActiveAuras>(actor).unwrap().iter()
            .find(|aura| aura.spell_id == id && aura.caster == actor)
            .expect("actual native equipped Venomcursed driver");
        let effect = |index| parent.effects.iter().find(|effect| effect.index == index).unwrap();
        let positive = effect(0).amount;
        let negative = effect(1).amount;
        assert!(positive > 0.0 && negative < 0.0,
            "native item scaling supplies both signed values: driver{id} {positive}/{negative}");
        assert!(parent.cast_item_level.is_some(), "native equipment supplies item level");
        (effect(0).trigger_spell, positive, negative, parent.cast_item_level)
    });
    let mut observed = [false; 3];
    engage(&mut run);
    for _ in 0..TICKS {
        run.app.update();
        drive_player(&mut run).unwrap();
        let world = run.app.world();
        let auras = world.get::<ActiveAuras>(actor).unwrap();
        for (index, (buff_id, positive, negative, level)) in expected.iter().enumerate() {
            let Some(buff) = auras.iter().find(|aura| aura.spell_id == *buff_id && aura.caster == actor) else {
                continue;
            };
            let amount = |index| buff.effects.iter().find(|effect| effect.index == index)
                .expect("native stat buff retains both rating effects").amount;
            assert!((amount(0) - positive).abs() < 0.001,
                "Venomcursed{buff_id} must use actual positive driver amount{positive}, got{}",amount(0));
            assert!((amount(1) - negative).abs() < 0.001,
                "Venomcursed{buff_id} must use actual negative driver amount{negative}, got{}",amount(1));
            assert_eq!(buff.cast_item_level, *level, "native proc retains its equipped source level");
            observed[index] = true;
        }
        if observed.iter().all(|seen| *seen) {
            assert!(world.resource::<ObservedCombatEvents>().0.iter().any(|event| {
                event.source == Some(actor.to_bits()) && event.kind == CombatLogKind::Damage && event.amount > 0
            }), "actual native damage must earn these procs");
            eprintln!("NATIVE_VENOMCURSED observed={observed:?} expected={expected:?}");
            return;
        }
    }
    panic!("native equipped Venomcursed drivers did not produce all stat buffs: {observed:?}");
}
