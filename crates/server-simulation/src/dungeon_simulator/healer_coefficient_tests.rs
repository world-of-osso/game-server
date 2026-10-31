//! Baseline amount probes, not a legal talent build or healer-policy comparison.
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "checks native Emerald Blossom and Fluttering Seedlings"]
fn preservation_blossom_generates_seedlings_for_unfilled_slots() {
    let spells = [359793, 355913, 355916, 361361];
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &spells, 13).unwrap();
    h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    let allies: Vec<_> = [2.0, 3.0, 4.0]
        .into_iter()
        .map(|x| crate::spell_cast::test_harness::spawn_player(&mut h.app, 13, x))
        .collect();
    for unit in [h.caster, allies[0], allies[1], allies[2]] {
        *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
            current: 100_000.0,
            max: 100_000.0,
        };
    }
    h.app
        .world_mut()
        .get_mut::<Health>(allies[0])
        .unwrap()
        .current = 1.0;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 359793,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 355913, Some(allies[0]), true)
        .unwrap();
    h.advance(45);
    let events = &h
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let count = events
        .iter()
        .filter(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(361361))
        .count();
    assert_eq!(count, 3, "one base seedling and two unfilled Blossom slots");
    let start = events.len();
    for unit in [h.caster, allies[0], allies[1]] {
        h.app.world_mut().get_mut::<Health>(unit).unwrap().current = 1.0;
    }
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 355913, Some(allies[0]), true)
        .unwrap();
    h.advance(45);
    let events = &h
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[start..];
    let count = events
        .iter()
        .filter(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(361361))
        .count();
    assert_eq!(
        count, 1,
        "three Blossom recipients leave only the base seedling"
    );
}

#[test]
#[ignore = "checks Life-Giver's Flame against native Fire Breath damage"]
fn preservation_life_givers_flame_splits_fire_breath_damage() {
    let spells = [371426, 371441, 357209];
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &spells, 13).unwrap();
    h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    let ally = crate::spell_cast::test_harness::spawn_player(&mut h.app, 13, 3.0);
    for unit in [h.caster, ally] {
        *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
            current: 1.0,
            max: 100_000.0,
        };
    }
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 100.0;
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 100_000.0,
        max: 100_000.0,
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 371426,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    for _ in 0..2 {
        crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 357209, Some(h.npc), true)
            .unwrap();
    }
    h.advance(8);
    let events = &h
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let damage: Vec<_> = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage && event.spell_id == Some(357209) && !event.periodic
        })
        .collect();
    assert_eq!(damage.len(), 2, "both Fire Breath hits must land");
    assert!(
        damage.iter().all(|event| event.crit),
        "both hits must critically strike"
    );
    let noncritical_damage: f32 = damage.iter().map(|event| event.amount as f32 / 2.0).sum();
    for target in [h.caster, ally] {
        let heal = events
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(371441)
                    && event.target == Some(target.to_bits())
            })
            .expect("Life-Giver's Flame must heal each injured ally");
        assert!(
            (heal.amount as f32 - noncritical_damage * 0.8 / 2.0).abs() <= 2.0,
            "{heal:?}"
        );
    }
}

#[test]
#[ignore = "checks Consume Flame against the native Dream Breath and Verdant Embrace"]
fn preservation_consume_flame_consumes_dream_breath_on_verdant_embrace() {
    let spells = [
        444088, 355941, 360995, 361195, 361361, 355913, 355916, 445495,
    ];
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &spells, 13).unwrap();
    h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    *h.app.world_mut().get_mut::<Health>(h.caster).unwrap() = Health {
        current: 1.0,
        max: 100_000.0,
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 444088,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    // The rank-1 empower release sets the child's duration to eight 2-second ticks.
    let mut dream = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(355941)
        .unwrap()
        .clone();
    let duration = dream.misc.as_mut().unwrap().duration.as_mut().unwrap();
    duration.duration_ms = 16_000;
    duration.max_duration_ms = 16_000;
    crate::spell_cast::execute_cast(h.app.world_mut(), h.caster, &dream, Some(h.caster)).unwrap();
    h.advance(1);
    let expiry = |h: &crate::spell_cast::test_harness::Harness, phase: &str| {
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 355941 && aura.caster == h.caster)
            .unwrap_or_else(|| panic!("Dream Breath aura absent {phase}"))
            .expires_at
            .unwrap()
    };
    let before = expiry(&h, "before Verdant Embrace");
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 360995, Some(h.caster), 1);
    let after = expiry(&h, "after Verdant Embrace");
    assert!((before - after - 4.0).abs() < 0.001);
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
                && event.spell_id == Some(445495)
                && event.target == Some(h.caster.to_bits())
        })
        .expect("Verdant Embrace must detonate the consumed Dream Breath");
    // 70448: 4s / 2s × 0.5352 × 100 SP × 290%.
    assert!((309..=311).contains(&heal.amount), "{heal:?}");

    let before = expiry(&h, "before Fluttering Seedlings");
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 361361, Some(h.caster), 1);
    let after = expiry(&h, "after Fluttering Seedlings");
    assert!((before - after - 2.0).abs() < 0.001);
    h.advance(1);
    let events = &h
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let heals: Vec<_> = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(445495)
                && event.target == Some(h.caster.to_bits())
        })
        .collect();
    assert_eq!(heals.len(), 2);
    assert!((154..=156).contains(&heals[1].amount), "{:?}", heals[1]);

    let before = expiry(&h, "before Emerald Blossom");
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 355913, Some(h.caster), true)
        .unwrap();
    h.advance(45);
    let after = expiry(&h, "after Emerald Blossom");
    assert!((before - after - 2.0).abs() < 0.001);
    let events = &h
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let heals: Vec<_> = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(445495)
                && event.target == Some(h.caster.to_bits())
        })
        .collect();
    assert_eq!(heals.len(), 3);
    assert!((154..=156).contains(&heals[2].amount), "{:?}", heals[2]);
}

#[test]
#[ignore = "checks Preservation Dream Breath's native healing coefficients"]
fn preservation_dream_breath_child_matches_native_coefficients() {
    // 70448 SpellEffect 889437/1037451: HoT 0.5352, direct 0.9216.
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[355941], 13).unwrap();
    h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    *h.app.world_mut().get_mut::<Health>(h.caster).unwrap() = Health {
        current: 1.0,
        max: 100_000.0,
    };
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 355941, Some(h.caster), true)
        .unwrap();
    h.advance(45);
    let events = &h
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    for (periodic, expected) in [(false, 92), (true, 54)] {
        let heal = events.iter().find(|event| {
            event.kind == CombatLogKind::Heal
                && event.periodic == periodic
                && event.spell_id == Some(355941)
                && event.target == Some(h.caster.to_bits())
        });
        let heal = heal.expect("Dream Breath child must deliver the native heal");
        assert!((heal.amount - expected).abs() <= 1, "{heal:?}");
        assert_eq!(heal.overflow, 0);
    }
}

#[test]
#[ignore = "checks Preservation Reversion's native periodic healing coefficient"]
fn preservation_reversion_tick_matches_native_coefficient() {
    // 70448 SpellEffect 907578: coefficient 0.4275, period 2000 ms, variance 0.
    let mut h = crate::spell_cast::test_harness::harness(Vec::new(), &[366155], 13).unwrap();
    h.app
        .init_resource::<crate::spell_cast::test_harness::ObservedCombatEvents>();
    *h.app.world_mut().get_mut::<Health>(h.caster).unwrap() = Health {
        current: 1.0,
        max: 100_000.0,
    };
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 366155, Some(h.caster), true)
        .unwrap();
    h.advance(45);
    let events = &h
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let tick = events.iter().find(|event| {
        event.kind == CombatLogKind::Heal
            && event.periodic
            && event.spell_id == Some(366155)
            && event.target == Some(h.caster.to_bits())
    });
    let tick = tick.expect("Reversion must heal at its first native periodic tick");
    assert!((42..=43).contains(&tick.amount), "{tick:?}");
    assert_eq!(tick.overflow, 0);
}

#[test]
#[ignore = "checks native Protection Word of Glory rank 2"]
fn protection_word_of_glory_rank_two_scales_self_healing_with_missing_health() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::components::PowerType;

    let cast = |rank_two: bool| {
        let spells = if rank_two {
            vec![85673, 315921]
        } else {
            vec![85673]
        };
        let mut h = crate::spell_cast::test_harness::harness_with_plugins(
            Vec::new(),
            &spells,
            2,
            false,
            |app| {
                app.add_plugins(server_spell_scripts::SpellScriptsPlugin);
            },
        )
        .unwrap();
        h.app.init_resource::<ObservedCombatEvents>();
        h.app.world_mut().entity_mut(h.caster).insert((
            shared::components::Level(90),
            shared::components::ActiveSpec(66),
        ));
        h.advance(3);
        if rank_two {
            assert!(crate::auras::has_aura(h.app.world(), h.caster, 315921));
        }
        *h.app.world_mut().get_mut::<Health>(h.caster).unwrap() = Health {
            current: 50_000.0,
            max: 100_000.0,
        };
        h.set_power(PowerType::HolyPower, 3.0);
        crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 85673, Some(h.caster), false)
            .unwrap();
        h.advance(4);
        let events = &h.app.world().resource::<ObservedCombatEvents>().0;
        let heal = events
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(85673)
                    && event.target == Some(h.caster.to_bits())
            })
            .expect("Word of Glory must heal");
        heal.amount
    };
    let base = cast(false);
    let ranked = cast(true);
    let ratio = ranked as f32 / base as f32;
    assert!(
        (2.35..=2.65).contains(&ratio),
        "rank 2 at 50% health: base={base}, ranked={ranked}, ratio={ratio}"
    );
}

#[test]
#[ignore = "explicit seven-spec core-heal coefficient ledger qualification"]
fn healer_benchmark_core_direct_heal_amounts_match_db2_tc_ledger() {
    // DB2 SpellEffect, DifficultyID0. TC a352b1fa Unit.cpp7345–7456:
    // unmodified heal = base + healing power × BonusCoefficient (+ AP term).
    // These rows all have base0/AP0; Variance0.05 means ±2.5%, not ±5%.
    let cases: [(u32, u8, u32, f32, f32); 21] = [
        (65, 2, 19750, 315.6, 0.05),
        (65, 2, 82326, 3250.0, 0.05),
        (65, 2, 25914, 1117.0, 0.05),
        (65, 2, 85673, 415.8, 0.05),
        // 70448 SpellEffect 86331: Light of Dawn direct heal.
        (65, 2, 85222, 506.736, 0.0),
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
        // 70448 SpellEffect 898900: Verdant Embrace healing child.
        (1468, 13, 361195, 1719.0, 0.0),
        // 70448 SpellEffect 889395: Emerald Blossom healing child.
        (1468, 13, 355916, 323.0, 0.05),
        // 70448 SpellEffect 899173: Fluttering Seedlings healing child.
        (1468, 13, 361361, 260.0, 0.0),
        // 70448 SpellEffect 910753: Spiritbloom healing child.
        (1468, 13, 367230, 878.0, 0.05),
        (1468, 13, 364343, 240.0, 0.0),
    ];
    let mut mismatches = Vec::new();
    for (spec, class, spell, expected, variance) in cases {
        assert!(
            expected > 0.0,
            "spec{spec} spell{spell}: coefficient source pending"
        );
        let spells = match spell {
            18562 => vec![spell, 774],
            47750 => vec![47540, 47758, 47757, 47666, 47750],
            _ => vec![spell],
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
        let target = if spell == 47750 {
            crate::spell_cast::test_harness::spawn_player(&mut h.app, 5, 2.0)
        } else {
            h.caster
        };
        *h.app.world_mut().get_mut::<Health>(target).unwrap() = Health {
            current: 1.0,
            max: 100_000.0,
        };
        if spell == 47750 {
            // DB2 target77 needs the real47540→47757 channel target, not a direct child cast.
            h.cast_and_deliver(47540, Some(target));
            h.advance(50);
        } else {
            crate::spell_cast::creature_cast(
                h.app.world_mut(),
                h.caster,
                spell,
                Some(target),
                true,
            )
            .unwrap();
            h.advance(1);
        }
        let events = &h
            .app
            .world()
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        let Some(heal) = events.iter().find(|event| {
            event.kind == CombatLogKind::Heal
                && !event.periodic
                && event.spell_id == Some(spell)
                && event.target == Some(target.to_bits())
        }) else {
            let error = format!("spec{spec} spell{spell}: native direct heal absent: {events:?}");
            println!("HEAL_COEFFICIENT_MISSING {error}");
            mismatches.push(error);
            continue;
        };
        let delta = variance * 0.5;
        let range =
            (expected * (1.0 - delta)).round() as i32..=(expected * (1.0 + delta)).round() as i32;
        if !range.contains(&heal.amount) || heal.overflow != 0 {
            mismatches.push(format!(
                "spec{spec} spell{spell}: amount{} outside {range:?} or unexpected overheal{}",
                heal.amount, heal.overflow
            ));
        }
        println!(
            "HEAL_COEFFICIENT spec={spec} spell={spell} amount={} expected={expected} range={range:?}",
            heal.amount
        );
    }
    assert!(
        mismatches.is_empty(),
        "native core-heal coefficient mismatches: {mismatches:?}"
    );
}
