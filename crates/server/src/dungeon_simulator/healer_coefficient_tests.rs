//! Baseline amount probes, not a legal talent build or healer-policy comparison.
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "explicit seven-spec core-heal coefficient ledger qualification"]
fn healer_benchmark_core_direct_heal_amounts_match_db2_tc_ledger() {
    // DB2 SpellEffect, DifficultyID0. TC a352b1fa Unit.cpp7345–7456:
    // unmodified heal = base + healing power × BonusCoefficient (+ AP term).
    // These rows all have base0/AP0; Variance0.05 means ±2.5%, not ±5%.
    let cases: [(u32, u8, u32, f32, f32); 15] = [
        (65, 2, 19750, 315.6, 0.05),
        (65, 2, 82326, 3250.0, 0.05),
        (65, 2, 25914, 1117.0, 0.05),
        (256, 5, 2061, 682.0, 0.05),
        (256, 5, 47750, 401.1, 0.05),
        (257, 5, 2060, 972.861, 0.05),
        (257, 5, 2061, 682.0, 0.05),
        (264, 7, 77472, 1501.0, 0.05),
        (264, 7, 8004, 1100.0, 0.05),
        (270, 10, 116670, 258.0, 0.05),
        (270, 10, 124682, 861.12, 0.0),
        (105, 11, 8936, 536.0, 0.05),
        (105, 11, 18562, 1037.0, 0.05),
        (1468, 13, 361509, 780.0, 0.05),
        (1468, 13, 364343, 240.0, 0.0),
    ];
    for (spec, class, spell, expected, variance) in cases {
        assert!(
            expected > 0.0,
            "spec{spec} spell{spell}: coefficient source pending"
        );
        let spells = if spell == 18562 {
            vec![spell, 774]
        } else {
            vec![spell]
        };
        let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &spells, class).unwrap();
        if spell == 18562 {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: h.caster,
                    target: h.caster,
                    spell_id: 774,
                    effect_mask: 1,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        h.app
            .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
        h.app.world_mut().get_mut::<Health>(h.caster).unwrap().max = 100_000.0;
        h.app
            .world_mut()
            .get_mut::<Health>(h.caster)
            .unwrap()
            .current = 1.0;
        crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, spell, Some(h.caster), true)
            .unwrap();
        h.advance(1);
        let events = &h
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let heal = events
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(spell)
                    && event.target == Some(h.caster.to_bits())
            })
            .unwrap_or_else(|| {
                panic!("spec{spec} spell{spell}: native direct heal absent: {events:?}")
            });
        let delta = variance * 0.5;
        let range =
            (expected * (1.0 - delta)).round() as i32..=(expected * (1.0 + delta)).round() as i32;
        assert!(
            range.contains(&heal.amount),
            "spec{spec} spell{spell}: {} outside {range:?}",
            heal.amount
        );
        assert_eq!(heal.overflow, 0);
        println!(
            "HEAL_COEFFICIENT spec={spec} spell={spell} amount={} expected={expected} range={range:?}",
            heal.amount
        );
    }
}
