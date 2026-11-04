use std::path::Path;

#[test]
#[ignore = "requires native Restoration Shaman spell data"]
fn water_shield_restores_mana_from_incoming_melee() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, collect_swings, harness};
    use shared::components::{Health, Mana};
    use shared::protocol::CombatLogKind;
    let mut h = harness(Vec::new(), &[52127, 52128], 7).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    *h.app.world_mut().get_mut::<Health>(h.caster).unwrap() = Health {
        current: 100_000.0,
        max: 100_000.0,
    };
    *h.app.world_mut().get_mut::<Mana>(h.caster).unwrap() = Mana {
        current: 50_000.0,
        max: 100_000.0,
    };
    h.cast_and_deliver(52127, Some(h.caster));
    assert!(crate::auras::has_aura(h.app.world(), h.caster, 52127));
    let attacker = h.npc;
    let target = h.caster;
    collect_swings(&mut h, attacker, target, 5);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    assert!(
        events.iter().any(|e| e.kind == CombatLogKind::Energize
            && e.spell_id == Some(52128)
            && e.target == Some(target.to_bits())
            && e.amount == 2000),
        "Water Shield must refund 2% max mana on a melee hit: {events:?}"
    );
}

#[test]
#[ignore = "requires native Restoration Shaman spell data"]
fn resurgence_restores_mana_for_critical_direct_heals() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    use shared::components::{Health, Mana};
    use shared::protocol::CombatLogKind;
    let mut h = harness(Vec::new(), &[61295, 77472, 1064, 16196, 101033], 7).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let ally = spawn_player(&mut h.app, 7, 3.0);
    *h.app.world_mut().get_mut::<Health>(ally).unwrap() = Health {
        current: 1.0,
        max: 100_000.0,
    };
    *h.app.world_mut().get_mut::<Mana>(h.caster).unwrap() = Mana {
        current: 0.0,
        max: 100_000.0,
    };
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 100.0;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 16196,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    for (source, expected) in [(61295, 480), (77472, 800), (1064, 200)] {
        h.app
            .world_mut()
            .resource_mut::<ObservedCombatEvents>()
            .0
            .clear();
        crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, source, Some(ally), true)
            .unwrap();
        h.advance(2);
        let events = &h.app.world().resource::<ObservedCombatEvents>().0;
        assert!(
            events.iter().any(|e| e.spell_id == Some(source)
                && e.kind == CombatLogKind::Heal
                && e.crit
                && !e.periodic),
            "source must critically heal: {events:?}"
        );
        assert!(
            events.iter().any(|e| e.spell_id == Some(101033)
                && e.kind == CombatLogKind::Energize
                && e.target == Some(h.caster.to_bits())
                && e.amount == expected),
            "Resurgence source {source} must refund {expected}: {events:?}"
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn restoration_shaman_holds_chain_lightning_against_one_enemy() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let mut run = super::healer_simc_replay::snapshot_party(264, 90, 1);
    super::healer_simc_replay::engage(&mut run);
    let healer = run.healer;
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(healer)
            .unwrap()
            .contains(188443),
        "the class build knows Chain Lightning"
    );
    let mut damage_casts = 0;
    for _ in 0..300 {
        run.app.update();
    }
    for event in &run.app.world().resource::<ObservedCombatEvents>().0 {
        if event.source == Some(healer.to_bits()) && event.kind == CombatLogKind::CastSuccess {
            assert_ne!(
                event.spell_id,
                Some(188443),
                "no Chain Lightning against one enemy"
            );
            if event.target == Some(run.sources[0].to_bits()) {
                damage_casts += 1;
            }
        }
    }
    assert!(damage_casts > 0, "the healer must reach its damage branch");
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn restoration_shaman_chains_lightning_through_engaged_enemies_only() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use bevy::prelude::Entity;
    use shared::components::Position;
    use shared::protocol::CombatLogKind;
    let mut run = super::healer_simc_replay::snapshot_party(264, 90, 3);
    let healer = run.healer;
    assert!(
        run.app
            .world()
            .get::<crate::class_progression::KnownSpells>(healer)
            .unwrap()
            .contains(188443),
        "the class build knows Chain Lightning"
    );
    // Two dummies without a formation are engaged. The third stands 9 yd from
    // the second, inside one 10 yd jump, but 14 yd from the primary, outside
    // its assistance snapshot: it stays unengaged.
    let idle = run.sources.pop().unwrap();
    let [primary, engaged] = [run.sources[0], run.sources[1]];
    let origin = *run.app.world().get::<Position>(primary).unwrap();
    for (unit, z) in [(engaged, 5.0), (idle, 14.0)] {
        run.app.world_mut().entity_mut(unit).insert(Position {
            z: origin.z + z,
            ..origin
        });
    }
    // Only the healer casts, so no other member's spread engages the neighbour.
    for unit in [run.slots[0], run.slots[2], run.slots[3], run.slots[4]] {
        super::healer_simc_replay::hold(&mut run, unit);
    }
    super::healer_simc_replay::engage(&mut run);
    let hit = |run: &super::healer_simc_replay::ClassParty, target: Entity| {
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .any(|e| {
                e.source == Some(healer.to_bits())
                    && e.target == Some(target.to_bits())
                    && e.spell_id == Some(188443)
                    && e.kind == CombatLogKind::Damage
            })
    };
    let leader = run.slots[4];
    for _ in 0..1200 {
        if hit(&run, primary) && hit(&run, engaged) {
            break;
        }
        // A training dummy drops a combat reference 5 s after its last direct
        // hit; the held party no longer hits it, so keep its fight going.
        crate::combat_refs::set_in_combat_with(run.app.world_mut(), leader, engaged);
        run.app.update();
    }
    assert!(
        hit(&run, primary) && hit(&run, engaged),
        "Chain Lightning must reach both engaged enemies"
    );
    assert!(
        !hit(&run, idle),
        "Chain Lightning must not jump to the unengaged neighbour"
    );
    assert!(
        crate::combat_refs::active_opponents(run.app.world(), idle)
            .next()
            .is_none(),
        "the unengaged neighbour must stay out of combat"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot and SimC traces"]
fn restoration_shaman_uses_surging_totem_throughout_party_pulse() {
    let trace = std::env::var("HEALER_TANK_TIMELINE").unwrap();
    let party = std::env::var("HEALER_TANK_PARTY_TRACE").unwrap();
    let report = super::healer_simc_replay::replay_simc_party(
        264,
        90,
        Path::new(&trace),
        Path::new(&party),
        300,
        70448,
    )
    .unwrap();
    let casts = &report["healer_casts"];
    let count = casts["444995"].as_u64().unwrap_or(0);
    assert!(
        count >= 10,
        "Surging Totem is not used across the replay: {report}"
    );
    assert_eq!(
        casts["73920"].as_u64().unwrap_or(0),
        0,
        "selected Surging Totem replaces Healing Rain"
    );
    assert!(
        casts["1064"].as_u64().unwrap_or(0) > 0,
        "policy must use Chain Heal for group injury: {report}"
    );
    println!("SHAMAN_POLICY_REPORT={report}");
}

#[test]
#[ignore = "requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn follower_leaves_ghost_wolf_to_cast_riptide() {
    use server_follower_apl::literal_rotation::execution::{Execution, execute};
    let mut party = super::healer_simc_replay::snapshot_party(264, 90, 1);
    let healer = party.healer;
    let target = party.tank;
    crate::spell_cast::cast_triggered(party.app.world_mut(), healer, 2645, Some(healer), 0);
    assert!(crate::auras::has_aura(party.app.world(), healer, 2645));
    let spell = party
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(61295)
        .unwrap()
        .clone();
    assert!(matches!(
        crate::spell_cast::read_native_cast_start(party.app.world_mut(), healer, &spell, target),
        Err(crate::spell_cast::CastFailure::NotShapeshift)
    ));
    crate::spell_cast::read_follower_cast_after_unshift(
        party.app.world_mut(),
        healer,
        &spell,
        target,
    )
    .unwrap();
    execute(
        party.app.world_mut(),
        healer,
        Execution::Spell(61295, target, Some(target)),
    )
    .unwrap();
    assert!(!crate::auras::has_aura(party.app.world(), healer, 2645));
    assert!(crate::auras::has_aura(party.app.world(), target, 61295));
}

#[test]
#[ignore = "requires native Restoration Shaman spell data"]
fn earthliving_imbue_allows_riptide_to_apply_its_hot() {
    use crate::spell_cast::{
        CastTargets, cast_for_player_with_targets,
        test_harness::{ObservedCombatEvents, harness, spawn_player},
    };
    let mut h = harness(Vec::new(), &[382021, 382022, 382024, 61295], 7).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let ally = spawn_player(&mut h.app, 7, 3.0);
    cast_for_player_with_targets(
        h.app.world_mut(),
        h.caster,
        382021,
        CastTargets {
            unit: h.caster,
            primary: Some(h.caster),
            destination: None,
        },
    )
    .unwrap();
    h.advance(50);
    assert!(
        crate::auras::has_aura(h.app.world(), h.caster, 382022),
        "imbue must apply its native buff"
    );
    for _ in 0..100 {
        crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 61295, Some(ally), true)
            .unwrap();
        h.advance(2);
        if crate::auras::has_aura(h.app.world(), ally, 382024) {
            return;
        }
    }
    panic!("Earthliving did not proc in 100 Riptides");
}

#[test]
#[ignore = "requires native Restoration Shaman spell data"]
fn critical_riptide_can_trigger_ancestral_awakening() {
    use crate::spell_cast::test_harness::{ObservedCombatEvents, harness, spawn_player};
    use shared::components::Health;
    use shared::protocol::CombatLogKind;
    let mut h = harness(Vec::new(), &[61295, 382309, 382311], 7).unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let target = spawn_player(&mut h.app, 7, 3.0);
    let lowest = spawn_player(&mut h.app, 7, 5.0);
    *h.app.world_mut().get_mut::<Health>(target).unwrap() = Health {
        current: 500_000.0,
        max: 1_000_000.0,
    };
    *h.app.world_mut().get_mut::<Health>(lowest).unwrap() = Health {
        current: 1.0,
        max: 1_000_000.0,
    };
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(h.caster)
        .unwrap()
        .crit_pct = 100.0;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 382309,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    for _ in 0..80 {
        h.app
            .world_mut()
            .resource_mut::<ObservedCombatEvents>()
            .0
            .clear();
        crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 61295, Some(target), true)
            .unwrap();
        h.advance(2);
        let events = &h.app.world().resource::<ObservedCombatEvents>().0;
        if let Some(heal) = events
            .iter()
            .find(|e| e.kind == CombatLogKind::Heal && e.spell_id == Some(382311))
        {
            let source = events
                .iter()
                .find(|e| e.kind == CombatLogKind::Heal && e.spell_id == Some(61295) && !e.periodic)
                .unwrap();
            assert!(source.crit);
            assert_eq!(heal.target, Some(lowest.to_bits()));
            assert!(!heal.crit);
            assert!((heal.amount as f32 - source.amount as f32 * 0.15).abs() <= 1.0);
            return;
        }
    }
    panic!("Ancestral Awakening did not proc in 80 critical Riptides");
}

#[test]
#[ignore = "requires native Restoration Shaman spell data"]
fn healing_rain_triggers_tidewaters_on_owned_riptides() {
    use crate::spell_cast::{
        CastTargets, cast_for_player_with_targets,
        test_harness::{ObservedCombatEvents, harness, spawn_player},
    };
    use shared::components::{Health, Mana};
    use shared::protocol::CombatLogKind;
    let mut h = harness(
        Vec::new(),
        &[73920, 73921, 444995, 61295, 462424, 462425],
        7,
    )
    .unwrap();
    h.app.init_resource::<ObservedCombatEvents>();
    let first = spawn_player(&mut h.app, 7, 3.0);
    let second = spawn_player(&mut h.app, 7, 5.0);
    let other = spawn_player(&mut h.app, 7, 6.0);
    for unit in [first, second, other] {
        *h.app.world_mut().get_mut::<Health>(unit).unwrap() = Health {
            current: 1.0,
            max: 100_000.0,
        };
    }
    *h.app.world_mut().get_mut::<Mana>(h.caster).unwrap() = Mana {
        current: 250_000.0,
        max: 250_000.0,
    };
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: 462424,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    for unit in [first, second] {
        crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, 61295, Some(unit), true)
            .unwrap();
    }
    h.advance(2);
    cast_for_player_with_targets(
        h.app.world_mut(),
        h.caster,
        73920,
        CastTargets {
            unit: h.caster,
            primary: Some(h.caster),
            destination: Some([3.0, 0.0, 0.0]),
        },
    )
    .unwrap();
    h.advance(100);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    for unit in [first, second] {
        assert!(
            events.iter().any(|e| e.kind == CombatLogKind::Heal
                && e.spell_id == Some(462425)
                && e.target == Some(unit.to_bits())
                && e.amount > 0),
            "{events:?}"
        );
    }
    assert!(
        !events
            .iter()
            .any(|e| e.spell_id == Some(462425) && e.target == Some(other.to_bits()))
    );
    h.app
        .world_mut()
        .resource_mut::<ObservedCombatEvents>()
        .0
        .clear();
    cast_for_player_with_targets(
        h.app.world_mut(),
        h.caster,
        444995,
        CastTargets {
            unit: h.caster,
            primary: Some(h.caster),
            destination: Some([3.0, 0.0, 0.0]),
        },
    )
    .unwrap();
    h.advance(100);
    let events = &h.app.world().resource::<ObservedCombatEvents>().0;
    for unit in [first, second] {
        assert!(
            events.iter().any(|e| e.kind == CombatLogKind::Heal
                && e.spell_id == Some(462425)
                && e.target == Some(unit.to_bits())
                && e.amount > 0),
            "Surging Totem must trigger Tidewaters: {events:?}"
        );
    }
}

#[test]
#[ignore = "requires native Restoration Shaman spell data"]
fn restoration_two_piece_heals_immediately_and_preserves_existing_rain() {
    use crate::auras::{AuraApplication, AuraSource, RemoveReason};
    use crate::spell_cast::{
        CastTargets, cast_for_player_with_targets,
        test_harness::{ObservedCombatEvents, harness, spawn_player},
    };
    use bevy::prelude::Entity;
    use shared::components::{Health, Mana};
    use shared::protocol::CombatLogKind;

    for source in [77472, 1064] {
        for equipped in [false, true] {
            let mut h = harness(Vec::new(), &[73920, 73921, 77472, 1064, 1296629], 7).unwrap();
            h.app.init_resource::<ObservedCombatEvents>();
            let known = h
                .app
                .world()
                .get::<crate::class_progression::KnownSpells>(h.caster)
                .unwrap()
                .iter()
                .filter(|&id| id != 1296629)
                .collect();
            h.app
                .world_mut()
                .entity_mut(h.caster)
                .insert(crate::class_progression::KnownSpells::fixture(known));
            crate::auras::remove_auras(
                h.app.world_mut(),
                h.caster,
                RemoveReason::Consumed,
                |aura| aura.spell_id == 1296629,
            );
            let old_target = spawn_player(&mut h.app, 7, -20.0);
            let new_target = spawn_player(&mut h.app, 7, 20.0);
            for target in [old_target, new_target] {
                *h.app.world_mut().get_mut::<Health>(target).unwrap() = Health {
                    current: 1.0,
                    max: 100_000.0,
                };
            }
            *h.app.world_mut().get_mut::<Mana>(h.caster).unwrap() = Mana {
                current: 250_000.0,
                max: 250_000.0,
            };
            if equipped {
                crate::auras::apply_aura(
                    h.app.world_mut(),
                    &AuraApplication {
                        caster: h.caster,
                        target: h.caster,
                        spell_id: 1296629,
                        effect_mask: u32::MAX,
                        source: AuraSource::Cast,
                    },
                );
            }
            let has_rain_heal = |h: &crate::spell_cast::test_harness::Harness, target: Entity| {
                h.app
                    .world()
                    .resource::<ObservedCombatEvents>()
                    .0
                    .iter()
                    .any(|event| {
                        event.kind == CombatLogKind::Heal
                            && event.spell_id == Some(73921)
                            && event.target == Some(target.to_bits())
                    })
            };
            let _seed = crate::random::seed_scope(u64::from(source));
            let mut proc_seen = false;
            for _ in 0..if equipped { 16 } else { 1 } {
                h.advance(200);
                for target in [old_target, new_target] {
                    h.app.world_mut().get_mut::<Health>(target).unwrap().current = 1.0;
                }
                h.app.world_mut().get_mut::<Mana>(h.caster).unwrap().current = 250_000.0;
                cast_for_player_with_targets(
                    h.app.world_mut(),
                    h.caster,
                    73920,
                    CastTargets {
                        unit: h.caster,
                        primary: Some(h.caster),
                        destination: Some([-20.0, 0.0, 0.0]),
                    },
                )
                .unwrap();
                h.advance(50);
                h.app
                    .world_mut()
                    .resource_mut::<ObservedCombatEvents>()
                    .0
                    .clear();
                crate::spell_cast::creature_cast(
                    h.app.world_mut(),
                    h.caster,
                    source,
                    Some(new_target),
                    true,
                )
                .unwrap();
                let immediate = has_rain_heal(&h, new_target);
                h.app
                    .world_mut()
                    .resource_mut::<ObservedCombatEvents>()
                    .0
                    .clear();
                h.advance(50);
                assert!(
                    has_rain_heal(&h, old_target),
                    "the original rain must continue"
                );
                if immediate {
                    assert!(equipped, "tier rain without the set");
                    assert!(has_rain_heal(&h, new_target), "tier rain must pulse");
                    h.advance(130);
                    h.app
                        .world_mut()
                        .resource_mut::<ObservedCombatEvents>()
                        .0
                        .clear();
                    h.advance(45);
                    assert!(!has_rain_heal(&h, new_target), "tier rain must expire");
                    proc_seen = true;
                    break;
                }
            }
            assert_eq!(proc_seen, equipped, "native RPPM tier rain: {source}");
        }
    }
}

#[test]
#[ignore = "requires native Restoration Shaman spell data"]
fn restoration_four_piece_shields_an_ally_inside_each_healing_rain() {
    use crate::spell_cast::{
        CastTargets, cast_for_player_with_targets,
        test_harness::{harness, spawn_player},
    };
    use shared::components::{Health, Mana};

    for spell in [73920, 444995] {
        for equipped in [false, true] {
            let mut h = harness(Vec::new(), &[73920, 73921, 444995, 1296630, 1300642], 7).unwrap();
            h.app
                .insert_resource(crate::test_data::world_data().clone());
            // The harness learns every supplied spell, including passive set bonuses.
            let known = h
                .app
                .world()
                .get::<crate::class_progression::KnownSpells>(h.caster)
                .unwrap()
                .iter()
                .filter(|&id| id != 1296630)
                .collect();
            h.app
                .world_mut()
                .entity_mut(h.caster)
                .insert(crate::class_progression::KnownSpells::fixture(known));
            crate::auras::remove_auras(
                h.app.world_mut(),
                h.caster,
                crate::auras::RemoveReason::Consumed,
                |aura| aura.spell_id == 1296630,
            );
            h.app
                .world_mut()
                .entity_mut(h.caster)
                .insert(shared::components::ActiveSpec(264));
            let inside = spawn_player(&mut h.app, 7, 20.0);
            let outside = spawn_player(&mut h.app, 7, 40.0);
            *h.app.world_mut().get_mut::<Health>(inside).unwrap() = Health {
                current: 1.0,
                max: 100_000.0,
            };
            *h.app.world_mut().get_mut::<Mana>(h.caster).unwrap() = Mana {
                current: 250_000.0,
                max: 250_000.0,
            };
            if equipped {
                crate::auras::apply_aura(
                    h.app.world_mut(),
                    &crate::auras::AuraApplication {
                        caster: h.caster,
                        target: h.caster,
                        spell_id: 1296630,
                        effect_mask: u32::MAX,
                        source: crate::auras::AuraSource::Cast,
                    },
                );
            }
            cast_for_player_with_targets(
                h.app.world_mut(),
                h.caster,
                spell,
                CastTargets {
                    unit: h.caster,
                    primary: Some(h.caster),
                    destination: Some([20.0, 0.0, 0.0]),
                },
            )
            .unwrap();
            h.advance(110);
            assert_eq!(
                crate::auras::has_aura(h.app.world(), inside, 1300642),
                equipped,
                "shield requires the equipped set and an actual rain pulse: {spell}"
            );
            assert!(!crate::auras::has_aura(h.app.world(), outside, 1300642));
        }
    }
}

#[test]
fn zero_capacity_charge_category_keeps_native_spell_cooldown() {
    use crate::spell_cast::test_harness::{Inbox, harness};
    // Native Dark Soul: Instability has category1447 with zero base charges
    // and its own 120-second recovery.
    let mut h = harness(Vec::new(), &[113858], 9).unwrap();
    h.cast_and_deliver(113858, None);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    h.advance(31);
    h.inbox = Inbox::default();
    h.cast_and_deliver(113858, None);
    assert_eq!(
        h.inbox.failed[0].reason,
        shared::spell_data::CastFailReason::OnCooldown
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn restoration_shaman_replay_native_mana_regeneration() {
    let mut run = super::healer_simc_replay::snapshot_party(264, 90, 1);
    let healer = run.healer;
    let known = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(healer)
        .unwrap();
    println!(
        "RESTO_SHAMAN_FLAME_SHOCK wrapper={} payload={}",
        known.contains(470411),
        known.contains(188389)
    );
    let mana = run
        .app
        .world_mut()
        .get_mut::<shared::components::Mana>(healer)
        .unwrap();
    mana.into_inner().current = 200_000.0;
    for _ in 0..20 {
        run.app.update();
    }
    let rate = run
        .app
        .world()
        .get::<crate::spell_cast::powers::PowerPools>(healer)
        .unwrap()
        .mana_regen_per_sec;
    assert!(
        crate::auras::has_aura(run.app.world(), healer, 52127),
        "the idle follower maintains Water Shield"
    );
    // Native 52127 effect1 adds 714.149 mana per five seconds at level 90.
    assert!((rate - 2142.8298).abs() < 0.01, "actual mana regen: {rate}");
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn restoration_shaman_healing_stream_summon_applies_native_pulse() {
    let mut run = super::healer_simc_replay::snapshot_party(264, 90, 1);
    let healer = run.healer;
    let tank = run.tank;
    let mut health = run
        .app
        .world_mut()
        .get_mut::<shared::components::Health>(tank)
        .unwrap();
    health.current = health.max * 0.5;
    drop(health);
    super::dps_parity::literal_rotation::execution::execute(
        run.app.world_mut(),
        healer,
        super::dps_parity::literal_rotation::execution::Execution::Spell(5394, healer, Some(tank)),
    )
    .unwrap();
    for _ in 0..50 {
        run.app.update();
    }
    let world = run.app.world_mut();
    let actors: Vec<_> = world
        .query::<(
            bevy::prelude::Entity,
            &shared::components::Npc,
            &crate::spell_cast::summon::SummonedCreature,
        )>()
        .iter(world)
        .filter(|(_, npc, summon)| npc.template_id == 3527 && summon.owner == healer)
        .map(|(entity, _, _)| entity)
        .collect();
    assert!(
        !actors.is_empty(),
        "actual Healing Stream summon must exist"
    );
    assert!(
        actors
            .iter()
            .any(|actor| crate::auras::has_aura(world, *actor, 5672)),
        "totem must apply its native periodic aura: {actors:?}"
    );
    let events = &world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    assert!(
        events.iter().any(|event| event.spell_id == Some(52042)
            && event.amount > 0
            && event.target == Some(tank.to_bits())),
        "native pulse must heal the injured tank; store={}, totem faction={:?}, tank faction={:?}, events={:?}",
        world
            .resource::<crate::spell_info::SpellStore>()
            .get(52042)
            .is_some(),
        world.get::<shared::components::UnitFactionTemplate>(actors[0]),
        world.get::<shared::components::UnitFactionTemplate>(tank),
        events
            .iter()
            .filter(|event| matches!(event.spell_id, Some(5672 | 52042)))
            .collect::<Vec<_>>()
    );
    assert!(
        events
            .iter()
            .filter(|event| event.spell_id == Some(52042))
            .all(|event| event.target == Some(tank.to_bits())),
        "the only injured party member must receive each pulse"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot and SimC traces"]
fn restoration_shaman_heals_the_simc_dummy_party() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::healer_simc_replay::replay_simc(264, 90, Path::new(&trace), 300, 70448).unwrap();
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert!(effective > 0.0 && raw >= effective, "{report}");
    let spells = metrics["spell_healing"].as_object().unwrap();
    assert!(!spells.is_empty());
    let spell_total: f64 = spells
        .values()
        .map(|spell| spell["effective_hps"].as_f64().unwrap())
        .sum();
    assert!((spell_total - effective).abs() < 0.000001);
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
    assert!(metrics["tank_dtps"].as_f64().unwrap() > 0.0, "{report}");
    assert!(
        metrics["tank_health_min"].as_f64().unwrap() >= 0.0,
        "{report}"
    );
    assert!(
        metrics["mana_min"].as_f64().unwrap()
            <= metrics["mana_curve"][0]["current"].as_f64().unwrap(),
        "{report}"
    );
    assert!(
        report["healer_casts"]["974"].as_u64().unwrap_or(0) > 0,
        "Earth Shield must be cast: {report}"
    );
    let streams = report["healer_casts"]["5394"].as_u64().unwrap_or(0);
    assert!(
        (1..=22).contains(&streams),
        "native 30s Healing Stream charges must bound casts: {report}"
    );
    assert!(
        metrics["spell_healing"]["52042"]["effective_hps"]
            .as_f64()
            .unwrap_or(0.0)
            > 0.0,
        "real Healing Stream Totem must heal the injured party: {report}"
    );
    assert!(
        report["healer_casts"]["77472"].as_u64().unwrap_or(0) > 0,
        "Healing Wave must be available between totem charges: {report}"
    );
    let casts = report["healer_casts"].as_object().unwrap();
    let total: u64 = casts.values().filter_map(|count| count.as_u64()).sum();
    assert!(total > 100, "healer must remain active: {report}");
    assert!(
        casts.get("470411").and_then(|v| v.as_u64()).unwrap_or(0) > 0,
        "Flame Shock should fill healthy periods: {report}"
    );
    assert!(
        casts.get("51505").and_then(|v| v.as_u64()).unwrap_or(0) > 0,
        "Lava Burst should fill healthy periods: {report}"
    );
    println!("RESTORATION_SHAMAN_REPORT={report}");
}
