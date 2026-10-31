use super::test_harness::*;
use crate::auras::ActiveAuras;
use bevy::prelude::*;
use shared::components::{ActiveSpec, Health};
use shared::protocol::CombatLogKind;

#[test]
fn dark_indulgence_grants_dark_side_from_real_mind_blast() {
    for selected in [false, true] {
        let mut h = harness(Vec::new(), &[8092, 372972, 198069], 5).unwrap();
        let caster = h.caster;
        let target = h.npc;
        h.app.world_mut().entity_mut(caster).insert((
            ActiveSpec(256),
            crate::class_progression::KnownSpells::fixture(if selected {
                vec![8092, 372972]
            } else {
                vec![8092]
            }),
        ));
        *h.app.world_mut().get_mut::<Health>(target).unwrap() = Health {
            current: 100_000.0,
            max: 100_000.0,
        };
        h.cast_and_deliver(8092, Some(target));
        h.advance(50);
        assert_eq!(
            crate::auras::has_aura(h.app.world(), caster, 198069),
            selected
        );
    }
}

#[test]
fn twist_of_fate_triggers_from_preimpact_health_for_real_spells() {
    for (spell, healing) in [(585, false), (2061, true)] {
        for (selected, health_pct, expected) in [
            (true, 34.0, true),
            (true, 35.0, false),
            (false, 34.0, false),
        ] {
            let mut h = harness(Vec::new(), &[585, 2061, 390972, 390978], 5).unwrap();
            let caster = h.caster;
            let target = if healing {
                spawn_player(&mut h.app, 5, 2.0)
            } else {
                h.npc
            };
            h.app.world_mut().entity_mut(caster).insert((
                ActiveSpec(256),
                crate::class_progression::KnownSpells::fixture(if selected {
                    vec![585, 2061, 390972]
                } else {
                    vec![585, 2061]
                }),
            ));
            *h.app.world_mut().get_mut::<Health>(target).unwrap() = Health {
                current: health_pct * 1000.0,
                max: 100_000.0,
            };
            h.cast_and_deliver(spell, Some(target));
            h.advance(50);
            assert_eq!(
                crate::auras::has_aura(h.app.world(), caster, 390978),
                expected,
                "spell {spell}, selected {selected}, health {health_pct}%"
            );
        }
    }
}

#[test]
fn twist_of_fate_increases_real_priest_damage_and_healing() {
    for (spell, kind) in [(585, CombatLogKind::Damage), (2061, CombatLogKind::Heal)] {
        let mut amounts = Vec::new();
        for active in [false, true] {
            let _seed = crate::random::seed_scope(70448);
            let mut h = harness(Vec::new(), &[585, 2061, 390972, 390978], 5).unwrap();
            h.app
                .init_resource::<super::test_harness::ObservedCombatEvents>();
            let caster = h.caster;
            let target = if spell == 2061 {
                spawn_player(&mut h.app, 5, 2.0)
            } else {
                h.npc
            };
            *h.app.world_mut().get_mut::<Health>(target).unwrap() = Health {
                current: 80_000.0,
                max: 100_000.0,
            };
            if active {
                crate::auras::apply_aura(
                    h.app.world_mut(),
                    &crate::auras::AuraApplication {
                        caster,
                        target: caster,
                        spell_id: 390978,
                        effect_mask: u32::MAX,
                        source: crate::auras::AuraSource::Cast,
                    },
                );
            }
            h.cast_and_deliver(spell, Some(target));
            h.advance(50);
            let event = h
                .app
                .world()
                .resource::<super::test_harness::ObservedCombatEvents>()
                .0
                .iter()
                .find(|event| {
                    event.kind == kind
                        && event.spell_id == Some(spell)
                        && event.target == Some(target.to_bits())
                })
                .unwrap();
            amounts.push(event.amount);
        }
        assert!(
            (amounts[1] as f32 - amounts[0] as f32 * 1.05).abs() <= 2.0,
            "spell {spell}: {amounts:?}"
        );
    }
}

#[test]
fn discipline_penance_extends_only_its_casters_real_pain() {
    for selected in [false, true] {
        let mut h = harness(
            Vec::new(),
            &[589, 47540, 47758, 47757, 47666, 47750, 390686],
            5,
        )
        .unwrap();
        let caster = h.caster;
        let enemy = h.npc;
        h.app.world_mut().entity_mut(caster).insert((
            ActiveSpec(256),
            crate::class_progression::KnownSpells::fixture(if selected {
                vec![589, 47540, 390686]
            } else {
                vec![589, 47540]
            }),
        ));
        *h.app.world_mut().get_mut::<Health>(enemy).unwrap() = Health {
            current: 100_000.0,
            max: 100_000.0,
        };
        h.cast_and_deliver(589, Some(enemy));
        h.advance(50);
        let other = spawn_player(&mut h.app, 5, 1.0);
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: other,
                target: enemy,
                spell_id: 589,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        let other_before = h
            .app
            .world()
            .get::<ActiveAuras>(enemy)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 589 && aura.caster == other)
            .unwrap()
            .expires_at;
        let expiry = |h: &Harness| {
            h.app
                .world()
                .get::<ActiveAuras>(enemy)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 589 && aura.caster == caster)
                .unwrap()
                .expires_at
                .unwrap()
        };
        let before = expiry(&h);
        h.cast_and_deliver(47540, Some(enemy));
        h.advance(70);
        let extension = expiry(&h) - before;
        let expected = if selected { 4.5 } else { 0.0 };
        assert!(
            (extension - expected).abs() < 0.001,
            "three actual bolts must extend owned Pain by {expected}s, got {extension}"
        );
        let other_after = h
            .app
            .world()
            .get::<ActiveAuras>(enemy)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == 589 && aura.caster == other)
            .unwrap()
            .expires_at;
        assert_eq!(
            other_after, other_before,
            "another Priest's Pain must not be extended"
        );
    }
}

#[test]
fn discipline_evangelism_cast_triggers_scaled_radiance_and_marks_allies() {
    let mut primary_heals = Vec::new();
    for root in [194509, 472433] {
        let _seed = crate::random::seed_scope(70448);
        let mut h = harness(Vec::new(), &[194509, 472433, 194384, 81749], 5).unwrap();
        h.app
            .init_resource::<super::test_harness::ObservedCombatEvents>();
        let caster = h.caster;
        h.app.world_mut().entity_mut(caster).insert(ActiveSpec(256));
        h.app
            .world_mut()
            .get_mut::<server::stats::CharacterStats>(caster)
            .unwrap()
            .crit_pct = 0.0;
        let primary = spawn_player(&mut h.app, 5, 2.0);
        *h.app.world_mut().get_mut::<Health>(primary).unwrap() = Health {
            current: 100.0,
            max: 10_000.0,
        };
        let ally = spawn_player(&mut h.app, 5, 3.0);
        *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
            current: 100.0,
            max: 10_000.0,
        };
        h.cast_and_deliver(root, Some(primary));
        h.advance(80);
        assert!(
            h.inbox.failed.is_empty(),
            "real cast failed: {:?}",
            h.inbox.failed
        );
        let events = &h
            .app
            .world()
            .resource::<super::test_harness::ObservedCombatEvents>()
            .0;
        let heal = events
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(194509)
                    && event.target == Some(primary.to_bits())
            })
            .expect("primary receives real Radiance heal");
        assert!(crate::auras::has_aura(h.app.world(), primary, 194384));
        assert!(crate::auras::has_aura(h.app.world(), ally, 194384));
        primary_heals.push(heal.amount);
    }
    let ratio = primary_heals[1] as f32 / primary_heals[0] as f32;
    assert!(
        (1.3..1.7).contains(&ratio),
        "Evangelism Radiance uses native 150%: {primary_heals:?}"
    );
}

#[test]
fn discipline_dark_side_is_consumed_and_snapshotted_for_all_penance_bolts() {
    let mut damage_sets = Vec::new();
    for empowered in [false, true] {
        let _seed = crate::random::seed_scope(70448);
        let mut h = harness(Vec::new(), &[47540, 47758, 47757, 47666, 47750, 198069], 5).unwrap();
        h.app
            .init_resource::<super::test_harness::ObservedCombatEvents>();
        let caster = h.caster;
        h.app.world_mut().entity_mut(caster).insert(ActiveSpec(256));
        h.app
            .world_mut()
            .get_mut::<server::stats::CharacterStats>(caster)
            .unwrap()
            .crit_pct = 0.0;
        if empowered {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster,
                    target: caster,
                    spell_id: 198069,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        h.cast_and_deliver(47540, Some(h.npc));
        h.advance(70);
        assert!(!crate::auras::has_aura(h.app.world(), caster, 198069));
        let events = &h
            .app
            .world()
            .resource::<super::test_harness::ObservedCombatEvents>()
            .0;
        let damage: Vec<_> = events
            .iter()
            .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(47666))
            .map(|event| event.amount)
            .collect();
        assert_eq!(damage.len(), 3);
        damage_sets.push(damage);
    }
    for (base, empowered) in damage_sets[0].iter().zip(&damage_sets[1]) {
        let ratio = *empowered as f32 / *base as f32;
        assert!(
            (1.2..1.4).contains(&ratio),
            "Dark Side must persist through each real bolt: {damage_sets:?}"
        );
    }
}

#[test]
fn discipline_prompt_prognosis_amplifies_only_the_third_penance_bolt() {
    let mut h = harness(Vec::new(), &[47540, 47758, 47757, 47666, 47750, 1246799], 5).unwrap();
    h.app
        .init_resource::<super::test_harness::ObservedCombatEvents>();
    let caster = h.caster;
    h.app.world_mut().entity_mut(caster).insert((
        ActiveSpec(256),
        crate::class_progression::KnownSpells::fixture(vec![47540, 1246799]),
    ));
    h.app
        .world_mut()
        .get_mut::<server::stats::CharacterStats>(caster)
        .unwrap()
        .crit_pct = 0.0;
    h.cast_and_deliver(47540, Some(h.npc));
    h.advance(70);
    let events = &h
        .app
        .world()
        .resource::<super::test_harness::ObservedCombatEvents>()
        .0;
    let damage: Vec<_> = events
        .iter()
        .filter(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(47666))
        .collect();
    assert_eq!(damage.len(), 3);
    assert!(
        damage[1].amount < damage[0].amount * 2,
        "the second bolt is unmodified: {damage:?}"
    );
    assert!(
        damage[2].amount > damage[0].amount * 2,
        "the third bolt gets the native 125% bonus: {damage:?}"
    );
}

#[test]
fn discipline_penance_twinsight_heals_and_holy_ray_increases_later_bolts() {
    let mut h = harness(
        Vec::new(),
        &[47540, 47758, 47757, 47666, 47750, 372969, 1235193, 440742],
        5,
    )
    .unwrap();
    h.app
        .init_resource::<super::test_harness::ObservedCombatEvents>();
    let caster = h.caster;
    h.app.world_mut().entity_mut(caster).insert((
        ActiveSpec(256),
        crate::class_progression::KnownSpells::fixture(vec![47540, 372969, 440742]),
    ));
    h.app
        .world_mut()
        .get_mut::<server::stats::CharacterStats>(caster)
        .unwrap()
        .crit_pct = 0.0;
    *h.app.world_mut().get_mut::<Health>(caster).unwrap() = Health {
        current: 500.0,
        max: 10_000.0,
    };
    let ally = spawn_player(&mut h.app, 5, 1.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 500.0,
        max: 10_000.0,
    };
    h.cast_and_deliver(47540, Some(h.npc));
    h.advance(70);
    let events = &h
        .app
        .world()
        .resource::<super::test_harness::ObservedCombatEvents>()
        .0;
    let damage: Vec<_> = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.source == Some(caster.to_bits())
                && event.spell_id == Some(47666)
        })
        .collect();
    let heals: Vec<_> = events
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Heal
                && event.source == Some(caster.to_bits())
                && event.spell_id == Some(47750)
        })
        .collect();
    assert_eq!(damage.len(), 3, "real channel damage: {events:?}");
    assert_eq!(heals.len(), 3, "Twinsight heals once per bolt: {events:?}");
    assert!(heals.iter().all(
        |event| event.target == Some(caster.to_bits()) || event.target == Some(ally.to_bits())
    ));
    assert!(
        damage[2].amount > damage[0].amount,
        "Holy Ray increases subsequent real bolts: {damage:?}"
    );
}

#[test]
fn shield_discipline_returns_mana_when_real_shields_are_depleted() {
    use shared::components::PowerType;
    for (spell, talent) in [(17, true), (1253593, true), (17, false)] {
        let mut h = harness(Vec::new(), &[17, 1253593, 197045, 47755, 194384, 81749], 5).unwrap();
        let caster = h.caster;
        h.app.world_mut().entity_mut(caster).insert(ActiveSpec(256));
        if talent {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster,
                    target: caster,
                    spell_id: 197045,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        let ally = spawn_player(&mut h.app, 5, 1.0);
        super::cast_triggered(h.app.world_mut(), caster, spell, Some(ally), 1);
        assert!(
            crate::auras::has_aura(h.app.world(), ally, spell),
            "real shield {spell} applied"
        );
        super::powers::change_power(h.app.world_mut(), caster, PowerType::Mana, -500.0);
        let before = super::powers::power_value(h.app.world(), caster, PowerType::Mana).unwrap();
        let (_, absorbed) =
            crate::auras::absorb_damage(h.app.world_mut(), ally, None, 1, 1_000_000.0);
        assert!(absorbed > 0.0 && !crate::auras::has_aura(h.app.world(), ally, spell));
        let after = super::powers::power_value(h.app.world(), caster, PowerType::Mana).unwrap();
        let expected = if talent {
            (before.1 * 0.005).trunc()
        } else {
            0.0
        };
        assert_eq!(
            after.0 - before.0,
            expected,
            "shield {spell}, talent={talent}"
        );
    }
}

#[test]
fn discipline_flash_heal_applies_atonement_even_at_full_health() {
    let mut h = harness(Vec::new(), &[2061, 194384, 81749, 94472], 5).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    let ally = spawn_player(&mut h.app, 5, 1.0);
    h.cast_and_deliver(2061, Some(ally));
    h.advance(80);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(ally)
            .unwrap()
            .iter()
            .any(|a| { a.spell_id == 194384 && a.caster == h.caster })
    );
}

#[test]
fn shadow_mend_override_heals_and_applies_atonement() {
    let mut h = harness(
        Vec::new(),
        &[2061, 186263, 1252217, 194384, 81749, 94472],
        5,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    let ally = spawn_player(&mut h.app, 5, 1.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 500.0,
        max: 10_000.0,
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 1252217,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.cast_and_deliver(2061, Some(ally));
    h.advance(80);
    assert!(
        h.inbox.failed.is_empty(),
        "override cast failed: {:?}",
        h.inbox.failed
    );
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(186263)
                && event.target == Some(ally.to_bits())
                && event.amount > 0),
        "Flash Heal must resolve as Shadow Mend: {:?}",
        h.inbox.log
    );
    assert!(
        crate::auras::has_aura(h.app.world(), ally, 194384),
        "Shadow Mend applies Atonement"
    );
}

#[test]
fn divine_aegis_accumulates_critical_flash_heals_and_absorbs_damage() {
    let mut h = harness(
        Vec::new(),
        &[2061, 47515, 47753, 194384, 81749, 94472, 133],
        5,
    )
    .unwrap();
    h.app
        .init_resource::<super::test_harness::ObservedCombatEvents>();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 100.0;
    let ally = spawn_player(&mut h.app, 5, 1.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 1_000.0,
        max: 100_000.0,
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 47515,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    for _ in 0..2 {
        h.cast_and_deliver(2061, Some(ally));
        h.advance(80);
    }
    let heals: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(2061)
                && event.target == Some(ally.to_bits())
        })
        .collect();
    assert_eq!(heals.len(), 2, "two real Flash Heals: {:?}", h.inbox.log);
    assert!(heals.iter().all(|event| event.crit));
    let expected = heals
        .iter()
        .map(|event| (event.amount - event.overflow).max(0) as f32 * 0.30)
        .sum::<f32>()
        .min(30_000.0);
    assert!(expected > 0.0);
    let fire = world_db().unwrap().spell_store().get(133).unwrap().clone();
    super::effects::deal_damage(
        h.app.world_mut(),
        &super::effects::EffectCtx {
            caster: h.npc,
            spell: &fire,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        },
        ally,
        expected + 100.0,
        false,
        false,
    );
    h.advance(2);
    let hit = h
        .app
        .world()
        .resource::<super::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .rev()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(133)
                && event.target == Some(ally.to_bits())
        })
        .unwrap_or_else(|| {
            panic!(
                "actual shield consumption: {:?}",
                h.app
                    .world()
                    .resource::<super::test_harness::ObservedCombatEvents>()
                    .0
            )
        });
    assert!(
        (hit.absorbed as f32 - expected).abs() <= 2.0,
        "Aegis must accumulate 30% of effective critical heals: expected {expected}, got {hit:?}"
    );
}

#[test]
fn divine_aegis_does_not_trigger_from_critical_atonement() {
    let mut h = harness(
        Vec::new(),
        &[585, 17, 194384, 81749, 94472, 47515, 47753],
        5,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 100.0;
    let ally = spawn_player(&mut h.app, 5, 1.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 1_000.0,
        max: 10_000.0,
    };
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 100_000.0,
        max: 100_000.0,
    };
    for (target, spell) in [(h.caster, 47515), (ally, 194384)] {
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.caster,
                target,
                spell_id: spell,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    h.cast_and_deliver(585, Some(h.npc));
    h.advance(70);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(94472)
                && event.crit
                && event.target == Some(ally.to_bits())),
        "real critical Atonement must occur"
    );
    assert!(
        !crate::auras::has_aura(h.app.world(), ally, 47753),
        "Atonement cannot trigger Divine Aegis"
    );
}

#[test]
fn master_the_darkness_penance_draws_one_proc_per_four_casts() {
    let mut h = harness(
        Vec::new(),
        &[47540, 47758, 47666, 1253590, 1253591, 1253593],
        5,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 100_000.0,
        max: 100_000.0,
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 1253590,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let mut procs = 0;
    for _ in 0..4 {
        h.cast_and_deliver(47540, Some(h.npc));
        h.advance(250);
        if crate::auras::has_aura(h.app.world(), h.caster, 1253591) {
            procs += 1;
            crate::auras::remove_auras(
                h.app.world_mut(),
                h.caster,
                crate::auras::RemoveReason::Consumed,
                |aura| aura.spell_id == 1253591,
            );
        }
    }
    assert!(
        h.inbox.failed.is_empty(),
        "all four real Penance casts must succeed: {:?}",
        h.inbox.failed
    );
    assert_eq!(procs, 1, "one success is drawn from each four-card deck");
}

#[test]
fn master_the_darkness_mind_blast_earns_void_shield_and_absorb_discharges() {
    let mut h = harness(
        Vec::new(),
        &[
            17, 8092, 133, 194384, 81749, 94472, 1253591, 1253593, 1253827, 1253828,
        ],
        5,
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    let ally_one = spawn_player(&mut h.app, 5, 1.0);
    let ally_two = spawn_player(&mut h.app, 5, 2.0);
    let marked = spawn_player(&mut h.app, 5, 3.0);
    let ally_three = spawn_player(&mut h.app, 5, 4.0);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: marked,
            spell_id: 194384,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 1253827,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.cast_and_deliver(8092, Some(h.npc));
    h.advance(80);
    assert!(
        crate::auras::has_aura(h.app.world(), h.caster, 1253591),
        "Mind Blast must earn the native override"
    );
    h.cast_and_deliver(17, Some(h.caster));
    h.advance(5);
    assert!(
        crate::auras::has_aura(h.app.world(), h.caster, 1253593),
        "Shield must resolve to Void Shield"
    );
    let protected: Vec<_> = [ally_one, ally_two, ally_three]
        .into_iter()
        .filter(|&ally| crate::auras::has_aura(h.app.world(), ally, 1253593))
        .collect();
    assert_eq!(
        protected.len(),
        2,
        "Void Shield protects the primary and two unmarked allies"
    );
    assert!(
        !crate::auras::has_aura(h.app.world(), marked, 1253593),
        "unmarked allies take priority"
    );
    for ally in protected {
        assert!(
            crate::auras::has_aura(h.app.world(), ally, 194384),
            "Void Shield must apply Atonement"
        );
    }
    assert!(
        !crate::auras::has_aura(h.app.world(), h.caster, 1253591),
        "Void Shield consumes its override"
    );
    let fire = world_db().unwrap().spell_store().get(133).unwrap().clone();
    h.inbox.log.clear();
    super::effects::deal_damage(
        h.app.world_mut(),
        &super::effects::EffectCtx {
            caster: h.npc,
            spell: &fire,
            depth: 0,
            dest: None,
            chain_multiplier: 1.0,
        },
        h.caster,
        200.0,
        false,
        false,
    );
    h.advance(5);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage
                && event.spell_id == Some(1253828)
                && event.target == Some(h.npc.to_bits())
                && event.amount > 0),
        "actual absorbed damage must discharge at its attacker: {:?}",
        h.inbox.log
    );
}

#[test]
fn void_shield_without_attacker_never_discharges_at_idle_enemy() {
    use shared::components::CombatStatus;
    let mut h = harness(Vec::new(), &[1253593, 1253827, 1253828], 5).unwrap();
    let caster = h.caster;
    let enemy = h.npc;
    h.app
        .world_mut()
        .entity_mut(enemy)
        .insert(CombatStatus(false));
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target: caster,
            spell_id: 1253827,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let before = h.app.world().get::<Health>(enemy).unwrap().current;
    crate::spell_cast::script_registry::after_absorb_consumed(
        h.app.world_mut(),
        caster,
        caster,
        None,
        1253593,
        200.0,
    );
    h.advance(5);
    assert_eq!(h.app.world().get::<Health>(enemy).unwrap().current, before);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(1253828))
    );

    h.app.world_mut().entity_mut(enemy).insert((
        CombatStatus(true),
        shared::components::Position {
            x: 50_001.0,
            y: 0.0,
            z: 0.0,
        },
    ));
    crate::spell_cast::script_registry::after_absorb_consumed(
        h.app.world_mut(),
        caster,
        caster,
        None,
        1253593,
        200.0,
    );
    h.advance(5);
    assert_eq!(h.app.world().get::<Health>(enemy).unwrap().current, before);
    h.app
        .world_mut()
        .entity_mut(enemy)
        .insert(shared::components::Position {
            x: 3.0,
            y: 0.0,
            z: 0.0,
        });
    crate::spell_cast::script_registry::after_absorb_consumed(
        h.app.world_mut(),
        caster,
        caster,
        None,
        1253593,
        200.0,
    );
    h.advance(5);
    assert!(h.app.world().get::<Health>(enemy).unwrap().current < before);
}

#[test]
fn atonement_transfers_critical_smite_before_absorb_but_not_fireball() {
    let mut h = harness(Vec::new(), &[17, 194384, 81749, 94472, 585, 133], 5).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 100.0;
    let ally = spawn_player(&mut h.app, 5, 1.0);
    {
        let mut health = h.app.world_mut().get_mut::<Health>(ally).unwrap();
        health.max = 10_000.0;
        health.current = 5_000.0;
    }
    {
        let mut health = h.app.world_mut().get_mut::<Health>(h.npc).unwrap();
        health.max = 100_000.0;
        health.current = 100_000.0;
    }
    h.cast_and_deliver(17, Some(ally));
    h.advance(35);
    super::cast_triggered_with_effect0_amount(h.app.world_mut(), h.npc, 17, Some(h.npc), 1, 10.0);
    h.advance(2);
    h.inbox.log.clear();
    h.cast_and_deliver(585, Some(h.npc));
    h.advance(60);
    let smite = h
        .inbox
        .log
        .iter()
        .find(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(585))
        .expect("real Smite damage");
    let transfer = h
        .inbox
        .log
        .iter()
        .find(|e| {
            e.kind == CombatLogKind::Heal
                && e.spell_id == Some(94472)
                && e.target == Some(ally.to_bits())
        })
        .expect("Atonement transfer");
    assert!(
        smite.crit && transfer.crit,
        "critical outcome must be carried: {smite:?} {transfer:?}"
    );
    assert!(
        smite.absorbed > 0 && smite.amount > 0,
        "fixture requires partial absorb: {smite:?}"
    );
    let expected = ((smite.amount + smite.absorbed) as f32 * 0.46).round() as i32;
    assert!(
        (transfer.amount - expected).abs() <= 1,
        "transfer uses pre-absorb damage: {smite:?} {transfer:?}"
    );
    h.advance(35);
    h.inbox.log.clear();
    h.cast_and_deliver(133, Some(h.npc));
    h.advance(80);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.kind == CombatLogKind::Damage && e.spell_id == Some(133)),
        "real Fireball damage required"
    );
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|e| e.kind == CombatLogKind::Heal && e.spell_id == Some(94472)),
        "Fireball is not a SimC Atonement trigger"
    );
}

#[test]
#[ignore = "real class summons require the prepared native creature database"]
fn discipline_death_summons_shadowfiend_only_in_execute_range() {
    use super::test_harness::{ObservedCombatEvents, prepare_class_world};
    let _seed = crate::random::seed_scope(70448);
    let prepared = prepare_class_world();
    let mut h = harness(
        Vec::new(),
        &[32379, 34433, 1280172, 81749, 94472, 137032],
        5,
    )
    .unwrap();
    h.app.insert_resource(prepared.world_data());
    let conn = rusqlite::Connection::open_with_flags(
        &world_db().unwrap().powers_db,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(super::summon::load_summon_properties(&conn).unwrap());
    h.app.init_resource::<ObservedCombatEvents>();
    h.app
        .init_resource::<super::test_harness::FiendAtonementReceipts>();
    h.app.world_mut().entity_mut(h.caster).insert((
        ActiveSpec(256),
        crate::class_progression::KnownSpells::fixture(vec![32379, 34433]),
        server::networking::CombatTarget(Some(h.npc)),
    ));
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 137032,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    for current in [900_000.0, 100_000.0] {
        for _ in 0..12 {
            *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
                current,
                max: 1_000_000.0,
            };
            let before = h
                .app
                .world()
                .resource::<ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.kind == CombatLogKind::Summon && event.spell_id == Some(1280172)
                })
                .count();
            super::cast_triggered(h.app.world_mut(), h.caster, 32379, Some(h.npc), 1);
            h.advance(1);
            let after = h
                .app
                .world()
                .resource::<ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.kind == CombatLogKind::Summon && event.spell_id == Some(1280172)
                })
                .count();
            assert!(
                after - before <= 1,
                "each Death cast may proc at most one fiend"
            );
        }
        let summons = h
            .app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| event.kind == CombatLogKind::Summon && event.spell_id == Some(1280172))
            .count();
        if current > 200_000.0 {
            assert_eq!(summons, 0, "no execute-range proc above 20%");
        } else {
            let rolls = &h
                .app
                .world()
                .resource::<super::test_harness::FiendAtonementReceipts>()
                .2;
            assert!(
                rolls.iter().all(|(_, _, _, chance)| *chance == 100.0),
                "native Discipline passive raises the 40% base chance to 100%"
            );
            assert_eq!(
                summons,
                rolls.len(),
                "every eligible hit summons exactly one fiend"
            );
        }
    }
}

#[test]
#[ignore = "real class summons require the prepared native creature database"]
fn discipline_fiend_landed_swings_restore_owner_mana() {
    use super::test_harness::{ObservedCombatEvents, prepare_class_world};
    use shared::components::PowerType;
    let prepared = prepare_class_world();
    for (summon, talent, mana_spell, percentage) in [
        (1280172, 34433, 343727, 0.005),
        (200174, 200174, 123051, 0.002),
    ] {
        let mut h = harness(Vec::new(), &[summon, talent, mana_spell], 5).unwrap();
        h.app.insert_resource(prepared.world_data());
        let conn = rusqlite::Connection::open_with_flags(
            &world_db().unwrap().powers_db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        h.app
            .insert_resource(super::summon::load_summon_properties(&conn).unwrap());
        h.app.init_resource::<ObservedCombatEvents>();
        h.app.world_mut().entity_mut(h.caster).insert((
            ActiveSpec(256),
            crate::class_progression::KnownSpells::fixture(vec![talent, summon]),
            server::networking::CombatTarget(Some(h.npc)),
        ));
        super::powers::change_power(h.app.world_mut(), h.caster, PowerType::Mana, -500.0);
        let maximum = super::powers::power_value(h.app.world(), h.caster, PowerType::Mana)
            .unwrap()
            .1;
        h.cast_and_deliver(summon, Some(h.npc));
        h.advance(60);
        let events = &h.app.world().resource::<ObservedCombatEvents>().0;
        let gains: Vec<_> = events
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Energize
                    && event.spell_id == Some(mana_spell)
                    && event.target == Some(h.caster.to_bits())
            })
            .collect();
        assert!(
            !gains.is_empty(),
            "summon {summon} must energize its owner: {events:?}; failed={:?}, go={:?}",
            h.inbox.failed,
            h.inbox.go
        );
        assert!(
            gains
                .iter()
                .all(|event| event.amount == (maximum * percentage) as i32),
            "unexpected fiend mana amounts: {gains:?}"
        );
    }
}

#[test]
#[ignore = "real class summons require the prepared native creature database"]
fn discipline_fiend_spells_transfer_pet_damage_to_atonement() {
    use super::summon::SummonedCreature;
    use super::test_harness::{ObservedCombatEvents, prepare_class_world};
    let prepared = prepare_class_world();
    for spell in [1280172, 200174, 451235] {
        let mut h = harness(Vec::new(), &[17, 194384, 81749, 94472, spell, 451435], 5).unwrap();
        h.app.insert_resource(prepared.world_data());
        if spell == 451235 {
            assert!(
                h.app
                    .world()
                    .resource::<server::world_data::WorldData>()
                    .templates
                    .contains_key(&224466),
                "prepared Voidwraith template missing"
            );
        }
        let conn = rusqlite::Connection::open_with_flags(
            &world_db().unwrap().powers_db,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        h.app
            .insert_resource(super::summon::load_summon_properties(&conn).unwrap());
        h.app.init_resource::<ObservedCombatEvents>();
        h.app
            .init_resource::<super::test_harness::FiendAtonementReceipts>();
        h.app.world_mut().entity_mut(h.caster).insert((
            ActiveSpec(256),
            server::networking::CombatTarget(Some(h.npc)),
        ));
        h.app
            .world_mut()
            .get_mut::<server::stats::CharacterStats>(h.caster)
            .unwrap()
            .spell_power = 1_000.0;
        *h.app.world_mut().get_mut::<Health>(h.caster).unwrap() = Health {
            current: 1_000.0,
            max: 10_000.0,
        };
        *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
            current: 100_000.0,
            max: 100_000.0,
        };
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.caster,
                target: h.caster,
                spell_id: 194384,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        if spell == 451235 {
            let position = super::targets::position(h.app.world(), h.npc).unwrap();
            let mut native = world_db()
                .unwrap()
                .spell_store()
                .get(spell)
                .unwrap()
                .clone();
            native.cast_destination = Some(position.to_array());
            super::plan_triggered_spell(h.app.world_mut(), h.caster, &native, None, 1)
                .unwrap_or_else(|failure| panic!("Voidwraith destination plan: {failure:?}"));
            assert!(super::launch_triggered_spell(
                h.app.world_mut(),
                h.caster,
                &native,
                None,
                1
            ));
        } else {
            h.cast_and_deliver(spell, Some(h.npc));
        }
        h.advance(2);
        assert!(
            h.inbox.failed.is_empty(),
            "summon {spell} cast failed: {:?}",
            h.inbox.failed
        );
        let summons: Vec<_> = h
            .app
            .world_mut()
            .query::<(Entity, &SummonedCreature)>()
            .iter(h.app.world())
            .map(|(entity, summon)| (entity, summon.owner, summon.spell_id))
            .collect();
        let actor = summons
            .iter()
            .find_map(|(entity, owner, id)| (*owner == h.caster && *id == spell).then_some(*entity))
            .unwrap_or_else(|| panic!("summon {spell} not present immediately: {summons:?}"));
        assert_eq!(
            h.app
                .world()
                .get::<shared::components::UnitLevel>(actor)
                .unwrap()
                .0,
            h.app
                .world()
                .get::<shared::components::Level>(h.caster)
                .unwrap()
                .0,
            "summon {spell} must inherit its owner's level"
        );
        h.advance(60);
        let events = &h.app.world().resource::<ObservedCombatEvents>().0;
        assert!(
            events
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.source == Some(actor.to_bits())
                    && event.target == Some(h.npc.to_bits())
                    && event.amount > 0),
            "summon {spell} must actually damage its target: {events:?}"
        );
        let receipt = h
            .app
            .world()
            .resource::<super::test_harness::FiendAtonementReceipts>()
            .0[&h.caster];
        assert!(
            receipt.0 > 0.0 && receipt.1 > 0.0 && receipt.2 > 0.0,
            "fiend damage and transfer receipts: {receipt:?}"
        );
        if spell == 1280172 || spell == 200174 {
            let fiends = &h
                .app
                .world()
                .resource::<super::test_harness::FiendAtonementReceipts>()
                .1;
            let hits = &fiends
                .iter()
                .find(|fiend| fiend.actor == actor)
                .unwrap()
                .landed_raw;
            assert!(
                hits.iter().any(|hit| *hit > 80.0),
                "fiend melee must inherit owner SP: {hits:?}"
            );
        }
        assert!(
            events.iter().any(|event| event.kind == CombatLogKind::Heal
                && event.source == Some(h.caster.to_bits())
                && event.target == Some(h.caster.to_bits())
                && event.spell_id == Some(94472)
                && event.amount > 0),
            "summon {spell} must transfer Atonement: {events:?}"
        );
    }
}

#[test]
fn atonement_applies_separate_injured_and_full_health_soft_caps() {
    let mut h = harness(Vec::new(), &[194384, 81749, 94472, 585], 5).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 100_000.0,
        max: 100_000.0,
    };
    let allies: Vec<_> = (0..7)
        .map(|index| spawn_player(&mut h.app, 5, index as f32))
        .collect();
    for (index, &ally) in allies.iter().enumerate() {
        *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
            current: if index == 6 { 10_000.0 } else { 1_000.0 },
            max: 10_000.0,
        };
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.caster,
                target: ally,
                spell_id: 194384,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    h.inbox.log.clear();
    h.cast_and_deliver(585, Some(h.npc));
    h.advance(60);
    let damage = h
        .inbox
        .log
        .iter()
        .find(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(585))
        .expect("real Smite damage");
    let base = damage.amount as f32 * 0.46;
    for (index, ally) in allies.into_iter().enumerate() {
        let heal = h
            .inbox
            .log
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(94472)
                    && event.target == Some(ally.to_bits())
            })
            .expect("each marked ally receives Atonement");
        let factor = if index == 6 {
            (5.0_f32 / 7.0).powf(0.75)
        } else {
            (5.0_f32 / 6.0).powf(0.75)
        };
        assert!(
            (heal.amount - (base * factor).round() as i32).abs() <= 1,
            "target {index}: {heal:?}"
        );
        if index == 6 {
            assert_eq!(heal.overflow, heal.amount);
        }
    }
}

#[test]
fn atonement_shadow_word_pain_uses_native_abyssal_reverie() {
    let mut h = harness(Vec::new(), &[17, 194384, 81749, 94472, 589, 373054], 5).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 373054,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let ally = spawn_player(&mut h.app, 5, 1.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 1_000.0,
        max: 10_000.0,
    };
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 100_000.0,
        max: 100_000.0,
    };
    h.cast_and_deliver(17, Some(ally));
    h.advance(35);
    h.inbox.log.clear();
    h.cast_and_deliver(589, Some(h.npc));
    h.advance(120);
    let index = h
        .inbox
        .log
        .iter()
        .position(|event| {
            event.kind == CombatLogKind::Damage && event.spell_id == Some(589) && event.periodic
        })
        .expect("real Pain tick");
    let damage = &h.inbox.log[index];
    let transfer = h.inbox.log[index + 1..]
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(94472)
                && event.target == Some(ally.to_bits())
        })
        .expect("Atonement transfer");
    // 70448 Abyssal Reverie effect 0 adds 10% to Shadow transfers.
    let expected = (damage.amount as f32 * 0.46 * 1.10).round() as i32;
    assert!(
        (transfer.amount - expected).abs() <= 1,
        "native Shadow transfer bonus: {damage:?} {transfer:?}"
    );
}

#[test]
fn atonement_smite_uses_native_oracle_and_voidheart_bonuses() {
    for (aura, multiplier) in [(1272359, 1.15_f32), (449887, 1.20_f32)] {
        let mut h = harness(Vec::new(), &[17, 194384, 81749, 94472, 585, aura], 5).unwrap();
        h.app
            .world_mut()
            .entity_mut(h.caster)
            .insert(ActiveSpec(256));
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.caster,
                target: h.caster,
                spell_id: aura,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        let ally = spawn_player(&mut h.app, 5, 1.0);
        *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
            current: 1_000.0,
            max: 10_000.0,
        };
        *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
            current: 100_000.0,
            max: 100_000.0,
        };
        h.cast_and_deliver(17, Some(ally));
        h.advance(35);
        h.inbox.log.clear();
        h.cast_and_deliver(585, Some(h.npc));
        h.advance(60);
        let damage = h
            .inbox
            .log
            .iter()
            .find(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(585))
            .expect("real Smite damage");
        let transfer = h
            .inbox
            .log
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(94472)
                    && event.target == Some(ally.to_bits())
            })
            .expect("Atonement transfer");
        let expected = (damage.amount as f32 * 0.46 * multiplier).round() as i32;
        assert!(
            (transfer.amount - expected).abs() <= 1,
            "native talent {aura}: {damage:?} {transfer:?}"
        );
    }
}

#[test]
fn atonement_smite_uses_native_grace_mastery() {
    let mut h = harness(Vec::new(), &[17, 194384, 81749, 94472, 585, 271534], 5).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .mastery_points = 20.0;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 271534,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let ally = spawn_player(&mut h.app, 5, 1.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 1_000.0,
        max: 10_000.0,
    };
    *h.app.world_mut().get_mut::<Health>(h.npc).unwrap() = Health {
        current: 100_000.0,
        max: 100_000.0,
    };
    h.cast_and_deliver(17, Some(ally));
    h.advance(35);
    h.inbox.log.clear();
    h.cast_and_deliver(585, Some(h.npc));
    h.advance(60);
    let smite = h
        .inbox
        .log
        .iter()
        .find(|event| event.kind == CombatLogKind::Damage && event.spell_id == Some(585))
        .expect("real Smite damage");
    let transfer = h
        .inbox
        .log
        .iter()
        .find(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(94472)
                && event.target == Some(ally.to_bits())
        })
        .expect("Atonement transfer");
    // 70448 Grace effect 0 is 1.35% per mastery point; 20 points yield 27%.
    let expected = (smite.amount as f32 * 0.46 * 1.27).round() as i32;
    assert!(
        (transfer.amount - expected).abs() <= 1,
        "native Grace must amplify transfer: {smite:?} {transfer:?}"
    );
}

#[test]
fn shield_absorbs_hit_and_smite_heals_its_atoned_ally() {
    let mut h = harness(Vec::new(), &[17, 139, 194384, 81749, 94472, 585, 133], 5).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(ActiveSpec(256));
    let ally = spawn_player(&mut h.app, 5, 1.0);
    {
        let mut health = h.app.world_mut().get_mut::<Health>(ally).unwrap();
        health.max = 1000.0;
        health.current = 1000.0;
    }
    {
        let mut health = h.app.world_mut().get_mut::<Health>(h.npc).unwrap();
        health.max = 10000.0;
        health.current = 10000.0;
    }
    h.cast_and_deliver(17, Some(ally));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(ally)
            .unwrap()
            .iter()
            .any(|a| a.spell_id == 194384 && a.caster == h.caster)
    );
    let fire = world_db().unwrap().spell_store().get(133).unwrap().clone();
    let ctx = super::effects::EffectCtx {
        caster: h.npc,
        spell: &fire,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    super::effects::deal_damage(h.app.world_mut(), &ctx, ally, 600.0, false, false);
    let after_hit = h.health(ally);
    assert!(
        after_hit > 400.0 && after_hit < 1000.0,
        "shield did not absorb a portion: {after_hit}"
    );
    h.advance(35);
    h.cast_and_deliver(585, Some(h.npc));
    h.advance(60);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        h.health(ally) > after_hit,
        "Smite did not heal the marked ally"
    );
    let damage = h
        .inbox
        .log
        .iter()
        .find(|e| e.spell_id == Some(585) && e.kind == CombatLogKind::Damage)
        .expect("Smite damage")
        .amount;
    let healing = h
        .inbox
        .log
        .iter()
        .find(|e| {
            e.spell_id == Some(94472)
                && e.target == Some(ally.to_bits())
                && e.kind == CombatLogKind::Heal
        })
        .expect("Atonement heal")
        .amount;
    assert_eq!(healing, (damage as f32 * 0.46).round() as i32);
}
