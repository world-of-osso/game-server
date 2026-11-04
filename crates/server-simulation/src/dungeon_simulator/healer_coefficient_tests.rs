//! Baseline amount probes, not a legal talent build or healer-policy comparison.
use shared::components::Health;
use shared::protocol::CombatLogKind;

#[test]
#[ignore = "requires native Protection Warrior spell data"]
fn protection_indomitable_heals_actual_paid_rage_only_when_selected() {
    use crate::auras::{AuraApplication, AuraSource};
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness};
    use shared::components::{ActiveSpec, PowerType};
    let mut h = harness(Vec::new(), &[190456, 202095, 316643], 1).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(73));
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 202095,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    for selected in [true, false] {
        if !selected {
            crate::auras::remove_auras(
                h.app.world_mut(),
                h.caster,
                crate::auras::RemoveReason::Unlearned,
                |aura| aura.spell_id == 202095,
            );
        }
        let maximum = h.app.world().get::<Health>(h.caster).unwrap().max;
        h.app
            .world_mut()
            .get_mut::<Health>(h.caster)
            .unwrap()
            .current = maximum * 0.5;
        crate::spell_cast::powers::change_power(
            h.app.world_mut(),
            h.caster,
            PowerType::Rage,
            1000.0,
        );
        let cursor = h.app.world().resource::<ObservedCombatEvents>().0.len();
        crate::spell_cast::scripts::reduce_cooldown(
            h.app.world_mut(),
            h.caster,
            190456,
            f64::INFINITY,
        );
        h.cast_and_deliver(190456, Some(h.caster));
        assert!(
            h.inbox.failed.is_empty(),
            "paid Ignore Pain: {:?}",
            h.inbox.failed
        );
        let heals: Vec<_> = h.app.world().resource::<ObservedCombatEvents>().0[cursor..]
            .iter()
            .filter(|event| event.kind == CombatLogKind::Heal && event.spell_id == Some(316643))
            .collect();
        if selected {
            assert_eq!(heals.len(), 1);
            let expected = maximum * 0.01 * 350.0 / 150.0;
            assert!(
                (heals[0].amount as f32 - expected).abs() <= 2.0,
                "35 Rage must heal 2 1/3 percent, got {} expected {expected}",
                heals[0].amount
            );
        } else {
            assert!(heals.is_empty(), "unselected Indomitable must not heal");
        }
    }
}

#[test]
#[ignore = "requires native Vengeance and Fire damage spell data"]
fn vengeance_revel_shields_fire_damage_with_carry_consumption_and_brand_caps() {
    use crate::auras::{AuraApplication, AuraSource};
    use crate::spell_cast::test_harness::{Harness, ObservedCombatEvents, harness};
    fn fire(h: &mut Harness, amount: f32) -> f32 {
        let spell = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(1216570)
            .unwrap()
            .clone();
        let ctx = crate::spell_cast::effects::EffectCtx {
            caster: h.caster,
            spell: &spell,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        };
        let cursor = h.app.world().resource::<ObservedCombatEvents>().0.len();
        crate::spell_cast::effects::deal_damage(
            h.app.world_mut(),
            &ctx,
            h.npc,
            amount,
            false,
            false,
        );
        h.app.world().resource::<ObservedCombatEvents>().0[cursor..]
            .iter()
            .find(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(1216570))
            .unwrap()
            .amount as f32
    }
    fn drain(h: &mut Harness) -> f32 {
        crate::auras::absorb_damage(h.app.world_mut(), h.caster, Some(h.npc), 1, 1_000_000.0).1
    }
    let mut h = harness(Vec::new(), &[343014, 1265857, 207771, 1216570], 12).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    *h.app.world_mut().get_mut::<Health>(h.caster).unwrap() = Health {
        current: 100_000.0,
        max: 200_000.0,
    };
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 100_000_000.0,
        max: 100_000_000.0,
    };
    assert_eq!(fire(&mut h, 10_000.0) > 0.0, true);
    assert_eq!(drain(&mut h), 0.0, "unselected talent must not shield");
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 343014,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    let first = fire(&mut h, 10_000.0);
    let receipt = crate::combat::replay_post_mitigation_hit(
        h.app.world_mut(),
        h.npc,
        h.caster,
        None,
        1,
        false,
        false,
        250.0,
    )
    .unwrap();
    assert_eq!(receipt.damage_hp, 0.0);
    assert_eq!(receipt.absorbed_hp, 250.0);
    let second = fire(&mut h, 20_000.0);
    assert!((drain(&mut h) - ((first + second) * 0.06 - 250.0)).abs() <= 2.0);
    fire(&mut h, 1_000_000.0);
    assert!(
        (drain(&mut h) - 20_000.0).abs() <= 2.0,
        "normal cap is 10% max health"
    );
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 207771,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    let branded = fire(&mut h, 10_000.0);
    assert!((drain(&mut h) - branded * 0.12).abs() <= 2.0);
    fire(&mut h, 1_000_000.0);
    assert!(
        (drain(&mut h) - 40_000.0).abs() <= 2.0,
        "Brand cap is 20% max health"
    );
}

#[test]
#[ignore = "requires native Vengeance profile and mobility spell data"]
fn first_in_last_out_infernal_strike_grants_then_linearly_decays_and_expires() {
    use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
    for elapsed_ticks in [8, 161] {
        let mut run =
            super::healer_simc_replay::snapshot_party_with_specs(264, 90, [581, 264, 70, 71, 261]);
        let actor = run.tank;
        let target = run.sources[0];
        super::healer_simc_replay::hold(&mut run, actor);
        crate::auras::apply_aura(
            run.app.world_mut(),
            &AuraApplication {
                caster: actor,
                target: actor,
                spell_id: 1266497,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
        let destination = crate::spell_cast::targets::position(run.app.world(), target).unwrap();
        crate::spell_cast::creature_cast_with_targets(
            run.app.world_mut(),
            actor,
            189110,
            crate::spell_cast::CastTargets {
                unit: target,
                primary: Some(target),
                destination: Some(destination.to_array()),
            },
        )
        .unwrap();
        let initial = run
            .app
            .world()
            .get::<ActiveAuras>(actor)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 1266619)
            .expect("Infernal Strike must earn its shield")
            .effects
            .iter()
            .find(|effect| effect.index == 0)
            .unwrap()
            .amount;
        assert!(initial > 1600.0);
        for _ in 0..elapsed_ticks {
            run.app.update();
        }
        if elapsed_ticks == 8 {
            let remaining = crate::auras::absorb_damage(
                run.app.world_mut(),
                actor,
                Some(target),
                1,
                1_000_000.0,
            )
            .1;
            assert!(
                (initial - remaining - 1600.0).abs() <= 2.0,
                "two authored 200ms ticks must remove 1600 points: initial={initial}, remaining={remaining}"
            );
        } else {
            assert!(
                !crate::auras::has_aura(run.app.world(), actor, 1266619),
                "the authored 8-second duration expires the shield even with points left"
            );
        }
    }
}

#[test]
#[ignore = "requires native Paladin Bulwark of Order data"]
fn bulwark_of_order_earns_owned_shield_from_avengers_damage_with_carry_and_cap() {
    use crate::auras::{AuraApplication, AuraSource};
    use crate::spell_cast::test_harness::{Harness, ObservedCombatEvents, harness};
    fn impact(h: &mut Harness, spell_id: u32, amount: f32) -> f32 {
        let spell = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(spell_id)
            .unwrap()
            .clone();
        let ctx = crate::spell_cast::effects::EffectCtx {
            caster: h.caster,
            spell: &spell,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        };
        let cursor = h.app.world().resource::<ObservedCombatEvents>().0.len();
        crate::spell_cast::effects::deal_damage(
            h.app.world_mut(),
            &ctx,
            h.npc,
            amount,
            false,
            false,
        );
        h.app.world().resource::<ObservedCombatEvents>().0[cursor..]
            .iter()
            .find(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(spell_id))
            .unwrap()
            .amount as f32
    }
    fn drain(h: &mut Harness) -> f32 {
        crate::auras::absorb_damage(h.app.world_mut(), h.caster, Some(h.npc), 1, 1_000_000.0).1
    }
    let mut h = harness(Vec::new(), &[31935, 20271, 209389, 209388], 2).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::ActiveSpec(66));
    *h.app.world_mut().get_mut::<Health>(h.caster).unwrap() = Health {
        current: 100_000.0,
        max: 200_000.0,
    };
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 100_000_000.0,
        max: 100_000_000.0,
    };
    impact(&mut h, 31935, 10_000.0);
    assert_eq!(
        drain(&mut h),
        0.0,
        "unselected Bulwark of Order must not shield"
    );
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 209389,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    let first = impact(&mut h, 31935, 10_000.0);
    let absorbed = crate::combat::replay_post_mitigation_hit(
        h.app.world_mut(),
        h.npc,
        h.caster,
        None,
        1,
        false,
        false,
        500.0,
    )
    .unwrap();
    assert_eq!(absorbed.absorbed_hp, 500.0);
    let second = impact(&mut h, 31935, 20_000.0);
    assert!(
        (drain(&mut h) - ((first + second) * 0.75 - 500.0)).abs() <= 2.0,
        "native 209389 effect0 grants 75% of dealt damage and retains unspent shield"
    );
    impact(&mut h, 20271, 10_000.0);
    assert_eq!(drain(&mut h), 0.0, "Judgment is not Avenger's Shield");
    impact(&mut h, 31935, 1_000_000.0);
    assert!(
        (drain(&mut h) - 100_000.0).abs() <= 2.0,
        "native effect1 caps at 50% max health"
    );
}

#[test]
#[ignore = "requires native Golden Hour, Echo, and Reversion spell data"]
fn golden_hour_echo_heals_echo_holder_at_echo_percentage() {
    use crate::auras::{AuraApplication, AuraSource};
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    let mut h = harness(Vec::new(), &[378196, 378213, 364343, 366155], 13).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let primary = spawn_player(&mut h.app, 13, 1.0);
    let echo = spawn_player(&mut h.app, 13, 2.0);
    for target in [primary, echo] {
        *h.app.world_mut().get_mut::<Health>(target).unwrap() = Health {
            current: 100_000.0,
            max: 100_000.0,
        };
        crate::combat::replay_post_mitigation_hit(
            h.app.world_mut(),
            h.npc,
            target,
            None,
            1,
            false,
            false,
            10_000.0,
        )
        .unwrap();
    }
    for (target, spell) in [(h.caster, 378196), (echo, 364343)] {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &AuraApplication {
                caster: h.caster,
                target,
                spell_id: spell,
                effect_mask: u32::MAX,
                source: AuraSource::Cast,
            },
        );
    }
    h.cast_and_deliver(366155, Some(primary));
    h.advance(10);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events.iter().any(|e| e.kind == CombatLogKind::Heal
            && e.spell_id == Some(378213)
            && e.target == Some(echo.to_bits())
            && e.amount == 1050),
        "{events:?}"
    );
}

#[test]
#[ignore = "requires native Golden Hour and Merithra spell data"]
fn golden_hour_also_triggers_on_upgraded_merithra() {
    use crate::auras::{AuraApplication, AuraSource};
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    let mut h = harness(Vec::new(), &[378196, 378213, 1256581], 13).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let target = spawn_player(&mut h.app, 13, 2.0);
    *h.app.world_mut().get_mut::<Health>(target).unwrap() = Health {
        current: 100_000.0,
        max: 100_000.0,
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 378196,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    crate::combat::replay_post_mitigation_hit(
        h.app.world_mut(),
        h.npc,
        target,
        None,
        1,
        false,
        false,
        10_000.0,
    )
    .unwrap();
    h.cast_and_deliver(1256581, Some(target));
    h.advance(10);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events.iter().any(|e| e.kind == CombatLogKind::Heal
            && e.spell_id == Some(378213)
            && e.target == Some(target.to_bits())
            && e.amount == 1500),
        "{events:?}"
    );
}

#[test]
#[ignore = "requires native Golden Hour and Reversion spell data"]
fn golden_hour_heals_recent_damage_including_absorbs() {
    use crate::auras::{AuraApplication, AuraSource};
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    let mut h = harness(Vec::new(), &[17, 378196, 378213, 366155], 13).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let target = spawn_player(&mut h.app, 13, 2.0);
    *h.app.world_mut().get_mut::<Health>(target).unwrap() = Health {
        current: 100_000.0,
        max: 100_000.0,
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 378196,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    crate::combat::replay_post_mitigation_hit(
        h.app.world_mut(),
        h.npc,
        target,
        None,
        1,
        false,
        false,
        10_000.0,
    )
    .unwrap();
    h.advance(130);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target,
            spell_id: 17,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    let receipt = crate::combat::replay_post_mitigation_hit(
        h.app.world_mut(),
        h.npc,
        target,
        None,
        1,
        false,
        false,
        10_000.0,
    )
    .unwrap();
    assert!(receipt.absorbed_hp > 0.0);
    h.cast_and_deliver(366155, Some(target));
    h.advance(10);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    let heals: Vec<_> = events
        .iter()
        .filter(|e| {
            e.kind == CombatLogKind::Heal
                && e.spell_id == Some(378213)
                && e.target == Some(target.to_bits())
        })
        .collect();
    assert_eq!(heals.len(), 1, "{events:?}");
    assert!((heals[0].amount - 1500).abs() <= 1, "{heals:?}");
}

#[test]
#[ignore = "requires native Essence Burst and Twin Flame spell data"]
fn preservation_twin_flame_requires_consuming_essence_burst() {
    use crate::auras::{AuraApplication, AuraSource};
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    let mut h = harness(Vec::new(), &[1265979, 1265991, 369299, 355913, 355916], 13).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let target = spawn_player(&mut h.app, 13, 2.0);
    *h.app.world_mut().get_mut::<Health>(target).unwrap() = Health {
        current: 1.0,
        max: 100_000.0,
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 1265979,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    h.cast_and_deliver(355913, Some(target));
    h.advance(50);
    assert!(
        !h.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|e| e.spell_id == Some(1265991))
    );
    h.advance(700);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 369299,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    h.app.world_mut().get_mut::<Health>(target).unwrap().current = 1.0;
    h.cast_and_deliver(355913, Some(target));
    h.advance(50);
    assert!(
        !crate::auras::has_aura(h.app.world(), h.caster, 369299),
        "failed casts: {:?}",
        h.inbox.failed
    );
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events.iter().any(|e| e.kind == CombatLogKind::Heal
            && e.spell_id == Some(1265991)
            && e.target == Some(target.to_bits())
            && e.amount > 0),
        "{events:?}"
    );
}

#[test]
#[ignore = "requires native Essence Burst and Preservation 12.1 set spell data"]
fn preservation_set_2pc_sends_living_flame_when_essence_burst_is_consumed() {
    use crate::auras::{AuraApplication, AuraSource};
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    let mut h = harness(
        Vec::new(),
        &[1296641, 369299, 355913, 355916, 361469, 361509],
        13,
    )
    .unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let full = spawn_player(&mut h.app, 13, 2.0);
    let injured = spawn_player(&mut h.app, 13, 4.0);
    *h.app.world_mut().get_mut::<Health>(injured).unwrap() = Health {
        current: 1.0,
        max: 100_000.0,
    };
    let cast_blossom = |h: &mut crate::spell_cast::test_harness::Harness, auras: &[u32]| {
        for &spell_id in auras {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &AuraApplication {
                    caster: h.caster,
                    target: h.caster,
                    spell_id,
                    effect_mask: u32::MAX,
                    source: AuraSource::Cast,
                },
            );
        }
        let cursor = h.app.world().resource::<ObservedCombatEvents>().0.len();
        h.cast_and_deliver(355913, Some(full));
        h.advance(50);
        let heals: Vec<_> = h.app.world().resource::<ObservedCombatEvents>().0[cursor..]
            .iter()
            .filter(|e| e.kind == CombatLogKind::Heal && e.spell_id == Some(361509))
            .map(|e| e.target)
            .collect();
        h.advance(700);
        heals
    };
    // Without consuming Essence Burst, the set bonus sends nothing.
    assert_eq!(cast_blossom(&mut h, &[1296641]), Vec::new());
    // Consuming it sends Living Flame to the nearby injured ally because the target is full.
    assert_eq!(
        cast_blossom(&mut h, &[369299]),
        vec![Some(injured.to_bits())],
        "failed casts: {:?}",
        h.inbox.failed
    );
}

#[test]
#[ignore = "requires native Essence Burst and Preservation 12.1 set spell data"]
fn preservation_set_4pc_grants_essence_burst_only_from_verdant_embrace() {
    use crate::spell_cast::test_harness::{harness, spawn_player};
    for (spell, expected) in [
        (360995, true),
        (355913, false),
        (361469, false),
        (366155, false),
    ] {
        let spells = [
            1296642, 369299, 360995, 361195, 355913, 355916, 361469, 361509, 366155,
        ];
        let mut h = harness(Vec::new(), &spells, 13).unwrap();
        let target = spawn_player(&mut h.app, 13, 2.0);
        h.app.world_mut().get_mut::<Health>(target).unwrap().current = 1.0;
        h.advance(100);
        h.cast_and_deliver(spell, Some(target));
        h.advance(100);
        assert_eq!(
            crate::auras::has_aura(h.app.world(), h.caster, 369299),
            expected,
            "spell {spell}, failed casts: {:?}",
            h.inbox.failed
        );
    }
}

/// Share of `trials` foreground casts of `spell` (empowered to rank 1 when `empower`, always
/// critical when `crit`) after which the caster holds Preservation Essence Burst.
fn preservation_essence_burst_share(talents: &[u32], spell: u32, empower: bool, crit: bool) -> f64 {
    use crate::spell_cast::test_harness::{harness, spawn_player};
    const TRIALS: usize = 200;
    let _seed = crate::random::seed_scope(1468);
    let mut spells = vec![369299, spell, 361509, 355941, 359115];
    spells.extend_from_slice(talents);
    let mut h = harness(Vec::new(), &spells, 13).unwrap();
    let target = spawn_player(&mut h.app, 13, 2.0);
    if crit {
        h.app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(h.caster)
            .unwrap()
            .crit_pct = 100.0;
    }
    let mut gains = 0;
    for _ in 0..TRIALS {
        h.app.world_mut().get_mut::<Health>(target).unwrap().current = 1.0;
        h.app
            .world_mut()
            .get_mut::<shared::components::Mana>(h.caster)
            .unwrap()
            .current = 1e9;
        crate::auras::remove_auras(
            h.app.world_mut(),
            h.caster,
            crate::auras::RemoveReason::Cancelled,
            |aura| aura.spell_id == 369299,
        );
        crate::spell_cast::scripts::reduce_cooldown(
            h.app.world_mut(),
            h.caster,
            spell,
            f64::INFINITY,
        );
        if empower {
            let targets = crate::spell_cast::CastTargets {
                unit: h.caster,
                primary: None,
                destination: None,
            };
            crate::spell_cast::creature_cast_empowered_with_targets(
                h.app.world_mut(),
                h.caster,
                spell,
                targets,
                1,
            )
            .unwrap();
        } else {
            h.cast_and_deliver(spell, Some(target));
        }
        h.advance(80);
        gains += usize::from(crate::auras::has_aura(h.app.world(), h.caster, 369299));
    }
    assert!(
        h.inbox.failed.is_empty(),
        "failed casts: {:?}",
        h.inbox.failed
    );
    gains as f64 / TRIALS as f64
}

#[test]
#[ignore = "requires native Preservation Essence Burst, Essence Well and Living Flame data"]
fn preservation_essence_burst_sources_roll_their_native_chances() {
    let share = preservation_essence_burst_share;
    // 369297 effect 0: Living Flame 20% per hit; effect 1: Reversion 15% per cast.
    let living_flame = share(&[369297], 361469, false, false);
    assert!(
        (0.13..0.27).contains(&living_flame),
        "Living Flame {living_flame}"
    );
    let reversion = share(&[369297], 366155, false, false);
    assert!((0.09..0.21).contains(&reversion), "Reversion {reversion}");
    // 445625: a critical Living Flame hit rolls again, 1 - 0.8^2 = 36%; crits alone add nothing.
    let critical = share(&[369297], 361469, false, true);
    assert!(
        (0.13..0.27).contains(&critical),
        "critical Living Flame {critical}"
    );
    let titanic = share(&[369297, 445625], 361469, false, true);
    assert!(
        (0.28..0.44).contains(&titanic),
        "Titanic Precision {titanic}"
    );
    // 1265993 effect 0: 50% per Dream Breath release; nothing without the talent.
    let well = share(&[369297, 1265993], 355936, true, false);
    assert!((0.40..0.60).contains(&well), "Essence Well {well}");
    assert_eq!(share(&[369297], 355936, true, false), 0.0);
}

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

    let cast = |rank_two: bool, self_target: bool| {
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
        let target = if self_target {
            h.caster
        } else {
            crate::spell_cast::test_harness::spawn_player(&mut h.app, 2, 2.0)
        };
        *h.app.world_mut().get_mut::<Health>(target).unwrap() = Health {
            current: 50_000.0,
            max: 100_000.0,
        };
        h.set_power(PowerType::HolyPower, 3.0);
        crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 85673, Some(target), false)
            .unwrap();
        h.advance(4);
        let events = &h.app.world().resource::<ObservedCombatEvents>().0;
        let heal = events
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(85673)
                    && event.target == Some(target.to_bits())
            })
            .expect("Word of Glory must heal");
        heal.amount
    };
    let base = cast(false, true);
    let ranked = cast(true, true);
    let ratio = ranked as f32 / base as f32;
    assert!(
        (2.35..=2.65).contains(&ratio),
        "rank 2 at 50% health: base={base}, ranked={ranked}, ratio={ratio}"
    );
    let ally_base = cast(false, false);
    let ally_ranked = cast(true, false);
    let ally_ratio = ally_ranked as f32 / ally_base as f32;
    assert!(
        (0.93..=1.07).contains(&ally_ratio),
        "rank 2 must not enhance an ally: base={ally_base}, ranked={ally_ranked}"
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

#[test]
#[ignore = "requires native Dream Breath, Consume Flame and Merithra modifier data"]
fn preservation_consume_flame_inherits_source_modifiers_once() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness};
    for (merithra, versatility) in [(false, 0.0), (true, 0.0), (false, 50.0), (true, 50.0)] {
        let mut spells = vec![444088, 355941, 360995, 361195, 445495];
        if merithra {
            spells.push(1256689);
        }
        let mut h = harness(Vec::new(), &spells, 13).unwrap();
        h.app.init_resource::<ObservedCombatEvents>();
        *h.app.world_mut().get_mut::<Health>(h.caster).unwrap() = Health {
            current: 1.0,
            max: 1_000_000.0,
        };
        let mut stats = h
            .app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(h.caster)
            .unwrap();
        stats.crit_pct = 0.0;
        stats.versatility_pct = versatility;
        h.advance(2);
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
        crate::spell_cast::execute_cast(h.app.world_mut(), h.caster, &dream, Some(h.caster))
            .unwrap();
        h.advance(45);
        let tick = h
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .find(|e| e.spell_id == Some(355941) && e.kind == CombatLogKind::Heal && e.periodic)
            .unwrap()
            .amount;
        crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, 360995, Some(h.caster), 1);
        h.advance(3);
        let detonation = h
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .find(|e| e.spell_id == Some(445495) && e.kind == CombatLogKind::Heal)
            .unwrap()
            .amount;
        // 100 SP x 0.5352, with the selected Dream-only 60% and general versatility each once.
        let source_modifier = if merithra { 1.6 } else { 1.0 };
        let expected_tick: f32 = 53.52 * source_modifier * (1.0 + versatility / 100.0);
        let expected_detonation = expected_tick * 2.0 * 2.9; // four consumed seconds / two-second tick
        assert!(
            (tick as f32 - expected_tick).abs() <= 1.0,
            "merithra={merithra} versatility={versatility}: tick={tick} expected={expected_tick}"
        );
        assert!(
            (detonation as f32 - expected_detonation).abs() <= 1.0,
            "merithra={merithra} versatility={versatility}: detonation={detonation} expected={expected_detonation}"
        );
    }
}
