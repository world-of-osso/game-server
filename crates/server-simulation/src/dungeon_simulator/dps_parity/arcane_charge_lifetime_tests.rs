//! Native resource-backed36032 survives paid Blast overflow; Barrage consumes it.
use super::*;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::ObservedCombatEvents;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

fn charge_state(run: &Run) -> (f32, u8, u8) {
    let world = run.app.world();
    let power = crate::spell_cast::powers::power_value(world, run.actor, PowerType::ArcaneCharges)
        .unwrap()
        .0;
    let aura = world
        .get::<ActiveAuras>(run.actor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 36032 && aura.caster == run.actor);
    (
        power,
        aura.map_or(0, |aura| aura.stacks),
        aura.map_or(0, |aura| aura.charges),
    )
}

fn cast_and_finish(run: &mut Run, spell: u32) {
    super::arcane_tests::start_cast(run, spell);
    preparation::finish_pending_setup(run).unwrap();
    for _ in 0..20 {
        run.app.update();
    }
}

#[test]
#[ignore = "requires private native70448 Arcane player profile"]
fn arcane_charge_lifetime_native_paid_blast_overflow_keeps_buff_until_barrage() {
    // SimC76f8118 sc_mage.cpp2858–2865 gains charges on Blast;
    //2734–2738 expires them on Barrage, not on any spell they modify.
    // Native36032 retains ProcCharges1; its native proc mask matches Barrage.
    for second in [30451, 44425] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = prepare_literal(62, 1).unwrap();
        super::arcane_blast_timing_tests::set_native_charges(&mut run, 0.0);
        super::arcane_blast_timing_tests::set_native_charges(&mut run, 4.0);
        assert_eq!(
            charge_state(&run),
            (4.0, 4, 1),
            "authored starting resource/aura"
        );
        cast_and_finish(&mut run, 30451);
        let after_first = charge_state(&run);
        cast_and_finish(&mut run, second);
        let after_second = charge_state(&run);
        let hits = run
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.source == Some(run.actor.to_bits())
                    && event.target == Some(run.targets[0].to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .filter_map(|event| event.spell_id)
            .collect::<Vec<_>>();
        assert_eq!(
            hits.iter().filter(|spell| **spell == 30451).count(),
            if second == 30451 { 2 } else { 1 },
            "actual paid Blast impacts: {hits:?}"
        );
        if second == 44425 {
            assert_eq!(
                hits.iter().filter(|spell| **spell == 44425).count(),
                1,
                "actual Barrage impact"
            );
            assert_eq!(
                after_second,
                (0.0, 0, 0),
                "real Barrage spends the resource and aura"
            );
        } else {
            assert_eq!(
                after_second.0, 4.0,
                "second paid Blast also overflows the real capped pool"
            );
        }
        assert_eq!(
            after_first,
            (4.0, 4, 1),
            "first paid Blast must not spend Barrage-only36032; after second{second}: {after_second:?}"
        );
        if second == 30451 {
            assert_eq!(
                after_second,
                (4.0, 4, 1),
                "repeated paid Blast keeps the native charge modifier"
            );
        }
    }
}
