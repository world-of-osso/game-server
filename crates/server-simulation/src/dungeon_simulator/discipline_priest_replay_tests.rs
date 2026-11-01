use std::path::Path;

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn discipline_soulcoiler_real_item_channels_five_fractional_barriers() {
    use super::dps_parity::literal_rotation::{Driver, equipment::Equipment, execution::Prepared};
    use crate::simc_apl_native::Bindings;
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use server::simc_apl::Program;
    use shared::protocol::CombatLogKind;
    use std::collections::BTreeMap;

    let _seed = crate::random::seed_scope(70448);
    let mut run = super::healer_simc_replay::snapshot_party(256, 90, 1);
    let healer = run.healer;
    super::healer_simc_replay::hold(&mut run, healer);
    super::healer_simc_replay::equip_profile_item(&mut run, healer, 256, 270162);
    let equipment = Equipment::load(run.app.world(), healer, &mut Bindings::default()).unwrap();
    let item = equipment
        .uses()
        .find(|(slot, _)| *slot == "trinket1")
        .expect("equipped Soulcoiler")
        .1;
    assert_eq!(item.item_id, 270162);
    assert_eq!(item.on_use.spell_id, 1291894);
    let prepared = Prepared::read(
        run.app.world_mut(),
        healer,
        &run.sources,
        &equipment,
        std::iter::empty(),
        false,
    )
    .unwrap();
    let mut driver = Driver::new(Program::parse(&[("default", "use_item,slot=trinket1")]).unwrap());
    let (action, execution) = driver
        .select(
            "default",
            &BTreeMap::<String, f64>::new(),
            &prepared,
            &equipment,
        )
        .unwrap()
        .expect("real item is ready");
    driver
        .execute(run.app.world_mut(), healer, action, execution)
        .unwrap();
    for _ in 0..55 {
        run.app.update();
    }
    let members = run.slots.to_vec();
    let shielded: Vec<_> = members
        .into_iter()
        .filter(|unit| crate::auras::has_aura(run.app.world(), *unit, 1307578))
        .collect();
    assert_eq!(
        shielded.len(),
        5,
        "five channel ticks shield distinct allies"
    );
    let target = shielded[0];
    let fire = run
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(133)
        .unwrap()
        .clone();
    let ctx = crate::spell_cast::effects::EffectCtx {
        caster: run.sources[0],
        spell: &fire,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    crate::spell_cast::effects::deal_damage(
        run.app.world_mut(),
        &ctx,
        target,
        1_000.0,
        false,
        false,
    );
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let hit = events
        .iter()
        .rev()
        .find(|event| {
            event.kind == CombatLogKind::Damage
                && event.spell_id == Some(133)
                && event.target == Some(target.to_bits())
        })
        .expect("actual incoming hit");
    assert!(
        (hit.absorbed - hit.amount).abs() <= 1 && hit.absorbed > 0,
        "Soulcoil absorbs half the post-modifier hit: {hit:?}"
    );
}
#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn equipped_seastone_procs_intellect_and_reverses_after_six_stacks() {
    use super::dps_parity::literal_rotation::execution::{Execution, execute};
    use crate::auras::ActiveAuras;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::healer_simc_replay::snapshot_party(256, 90, 1);
    let healer = run.healer;
    let target = run.sources[0];
    super::healer_simc_replay::hold(&mut run, healer);
    super::healer_simc_replay::equip_profile_item(&mut run, healer, 256, 270167);
    let driver = run
        .app
        .world()
        .get::<ActiveAuras>(healer)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 1295058)
        .expect("equipped Seastone driver");
    let expected_amount = driver
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert!(expected_amount > 0.0);
    let mut stacks_seen = Vec::new();
    for _ in 0..20 {
        execute(
            run.app.world_mut(),
            healer,
            Execution::Spell(589, target, Some(target)),
        )
        .unwrap();
        for _ in 0..400 {
            run.app.update();
            if let Some(buff) = run
                .app
                .world()
                .get::<ActiveAuras>(healer)
                .unwrap()
                .iter()
                .find(|aura| aura.spell_id == 1295057)
            {
                let amount = buff
                    .effects
                    .iter()
                    .find(|effect| effect.index == 0)
                    .unwrap()
                    .amount;
                assert!((amount - expected_amount).abs() < 0.001);
                if stacks_seen.last().copied() != Some(buff.stacks) {
                    stacks_seen.push(buff.stacks);
                }
            }
        }
    }
    assert!(
        stacks_seen.contains(&6),
        "must reach six stacks: {stacks_seen:?}"
    );
    assert!(
        stacks_seen.windows(2).any(|pair| pair == [6, 5]),
        "must reverse after reaching six: {stacks_seen:?}"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn discipline_pain_observes_power_of_dark_side_procs() {
    use super::dps_parity::literal_rotation::execution::{Execution, execute};
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::healer_simc_replay::snapshot_party(256, 90, 1);
    let healer = run.healer;
    let target = run.sources[0];
    super::healer_simc_replay::hold(&mut run, healer);
    for _ in 0..10 {
        execute(
            run.app.world_mut(),
            healer,
            Execution::Spell(589, target, Some(target)),
        )
        .unwrap();
        for _ in 0..400 {
            run.app.update();
        }
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0;
    let ticks = events
        .iter()
        .filter(|event| {
            event.spell_id == Some(589) && event.kind == CombatLogKind::Damage && event.periodic
        })
        .count();
    let procs = events
        .iter()
        .filter(|event| {
            event.spell_id == Some(198069)
                && matches!(
                    event.kind,
                    CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
                )
        })
        .count();
    assert!(ticks > 0, "real Pain ticks required");
    assert!(
        procs > 0,
        "selected Power of the Dark Side must proc from real Pain ticks"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn discipline_smite_and_penance_capture_matched_stat_factors() {
    use super::dps_parity::literal_rotation::execution::{Execution, execute};
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    for (root, focus) in [(585, 585), (47540, 47666)] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::healer_simc_replay::snapshot_party(256, 90, 1);
        let healer = run.healer;
        let target = run.sources[0];
        super::healer_simc_replay::hold(&mut run, healer);
        let mut stats = run
            .app
            .world_mut()
            .get_mut::<crate::stats::CharacterStats>(healer)
            .unwrap();
        stats.spell_power = 3567.0;
        stats.versatility_pct = 5.6667;
        stats.crit_pct = 0.0;
        run.app.insert_resource(FactorCapture::new(
            healer,
            target,
            focus,
            "disc_matched",
            32,
        ));
        execute(
            run.app.world_mut(),
            healer,
            Execution::Spell(root, target, Some(target)),
        )
        .unwrap();
        for _ in 0..100 {
            run.app.update();
        }
        let trace = run.app.world().resource::<FactorCapture>().export();
        assert!(
            trace["captured_hits"].as_u64().unwrap_or(0) > 0,
            "real spell {focus}: {trace}"
        );
        println!("DISC_FACTOR_{focus}={trace}");
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn discipline_apl_applies_pain_and_does_not_refresh_early() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let mut run = super::healer_simc_replay::snapshot_party(256, 90, 1);
    super::healer_simc_replay::engage(&mut run);
    let (tank, healer) = (run.tank, run.healer);
    let enemy = run.sources[0];
    run.app
        .world_mut()
        .entity_mut(tank)
        .insert(crate::networking::CombatTarget(Some(enemy)));
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), enemy, 589),
        "Pain must apply to the enemy"
    );
    let casts = |run: &super::healer_simc_replay::ClassParty| {
        run.app
            .world()
            .resource::<ObservedCombatEvents>()
            .0
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::CastSuccess
                    && event.source == Some(healer.to_bits())
                    && event.spell_id == Some(589)
            })
            .count()
    };
    assert_eq!(casts(&run), 1);
    for _ in 0..40 {
        run.app.update();
    }
    assert_eq!(
        casts(&run),
        1,
        "fresh Pain must not be recast before its pandemic window"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn discipline_apl_uses_evangelism_without_injury_gate() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let mut run = super::healer_simc_replay::snapshot_party(256, 90, 1);
    super::healer_simc_replay::engage(&mut run);
    let (tank, healer) = (run.tank, run.healer);
    let members = run.slots.to_vec();
    run.app
        .world_mut()
        .entity_mut(tank)
        .insert(crate::networking::CombatTarget(None));
    let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
    // Engaged, the APL opens with Pain and Mind Blast before its ramp; the
    // uninjured party must still reach Evangelism.
    let cast = |run: &super::healer_simc_replay::ClassParty, spell: u32| {
        run.app.world().resource::<ObservedCombatEvents>().0[cursor..]
            .iter()
            .any(|event| {
                event.kind == CombatLogKind::CastSuccess
                    && event.source == Some(healer.to_bits())
                    && event.spell_id == Some(spell)
            })
    };
    for _ in 0..300 {
        if cast(&run, 472433) {
            break;
        }
        run.app.update();
    }
    assert!(
        cast(&run, 472433),
        "Evangelism must not require injured allies: {:?}",
        &run.app.world().resource::<ObservedCombatEvents>().0[cursor..]
    );
    assert!(
        members
            .iter()
            .any(|&member| crate::auras::has_aura(run.app.world(), member, 194384)),
        "Evangelism's Radiance must apply Atonement"
    );
    // Grant Radiance when the class build does not learn it, to exercise this APL branch.
    let mut spells: Vec<_> = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(healer)
        .unwrap()
        .iter()
        .collect();
    if !spells.contains(&194509) {
        spells.push(194509);
    }
    run.app
        .world_mut()
        .entity_mut(healer)
        .insert(crate::class_progression::KnownSpells::fixture(spells));
    for _ in 0..300 {
        if cast(&run, 194509) {
            break;
        }
        run.app.update();
    }
    assert!(
        cast(&run, 194509),
        "Radiance must follow under the Mind Blast cooldown condition: {:?}",
        &run.app.world().resource::<ObservedCombatEvents>().0[cursor..]
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn discipline_radiance_heals_nearby_full_health_class_party() {
    use crate::spell_cast::targets::{self, Relation};
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use bevy::prelude::Vec3;
    use shared::components::Position;
    use shared::protocol::CombatLogKind;

    let _seed = crate::random::seed_scope(1);
    let mut run = super::healer_simc_replay::snapshot_party(256, 90, 1);
    super::healer_simc_replay::engage(&mut run);
    let (tank, healer) = (run.tank, run.healer);
    super::healer_simc_replay::finish_precombat(&mut run.app, &[tank, healer]).unwrap();
    let position = *run.app.world().get::<Position>(healer).unwrap();
    let center = Vec3::new(position.x, position.y, position.z);
    let world = run.app.world_mut();
    let candidates = targets::area_units(world, healer, center, 40.0, Relation::Ally, 0);
    let relations: Vec<_> = run
        .slots
        .iter()
        .map(|&unit| {
            (
                unit,
                targets::relation_matches(world, healer, unit, Relation::Ally),
            )
        })
        .collect();
    let cursor = world.resource::<ObservedCombatEvents>().0.len();
    assert!(crate::spell_cast::try_cast_triggered(
        world,
        healer,
        194509,
        Some(healer),
        0
    ));
    run.app.update();
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[cursor..];
    let healed: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(healer.to_bits())
                && event.spell_id == Some(194509)
                && event.kind == CombatLogKind::Heal
        })
        .filter_map(|event| event.target)
        .collect();
    for &unit in &run.slots {
        assert!(
            healed.contains(&unit.to_bits()),
            "party member {unit:?} missed: healed={healed:?}, candidates={candidates:?}, relations={relations:?}"
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn discipline_void_shield_secondaries_exclude_party_pets_and_guardians() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use bevy::prelude::Entity;
    use shared::protocol::CombatLogKind;

    let _seed = crate::random::seed_scope(1);
    let mut run = super::healer_simc_replay::snapshot_party(256, 90, 1);
    super::healer_simc_replay::engage(&mut run);
    let (tank, healer) = (run.tank, run.healer);
    super::healer_simc_replay::finish_precombat(&mut run.app, &[tank, healer]).unwrap();
    let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
    for _ in 0..200 {
        run.app.update();
        let events = &run.app.world().resource::<ObservedCombatEvents>().0[cursor..];
        if events
            .iter()
            .filter(|event| {
                event.source == Some(healer.to_bits())
                    && event.spell_id == Some(1253593)
                    && matches!(
                        event.kind,
                        CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
                    )
            })
            .count()
            >= 3
        {
            break;
        }
    }
    let world = run.app.world();
    let events = &world.resource::<ObservedCombatEvents>().0[cursor..];
    let recipients: Vec<_> = events
        .iter()
        .filter(|event| {
            event.source == Some(healer.to_bits())
                && event.spell_id == Some(1253593)
                && matches!(
                    event.kind,
                    CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
                )
        })
        .filter_map(|event| event.target.and_then(Entity::try_from_bits))
        .take(3)
        .collect();
    let companions: Vec<_> = recipients
        .iter()
        .filter(|unit| !run.slots.contains(unit))
        .map(|&unit| {
            (
                unit,
                world
                    .get::<shared::components::Npc>(unit)
                    .map(|npc| npc.name.clone()),
                world
                    .get::<crate::hunter_pet::HunterPet>(unit)
                    .map(|pet| pet.owner),
                world
                    .get::<crate::spell_cast::summon::SummonedCreature>(unit)
                    .map(|summon| (summon.owner, summon.spell_id)),
            )
        })
        .collect();
    assert_eq!(
        recipients.len(),
        3,
        "the real Void Shield cast must select three recipients: {events:?}"
    );
    assert!(
        companions.is_empty(),
        "secondary recipients must be party members, not pets/guardians: {companions:?}"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn discipline_emergency_uses_void_shield_and_radiance() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::components::Health;
    use shared::protocol::CombatLogKind;

    for void_shield in [true, false] {
        let mut run = super::healer_simc_replay::snapshot_party(256, 90, 1);
        super::healer_simc_replay::engage(&mut run);
        let (tank, healer) = (run.tank, run.healer);
        super::healer_simc_replay::finish_precombat(&mut run.app, &[tank, healer]).unwrap();
        let excluded = [33206, 62618, 472433, 47540, 32379, 8092];
        let mut spells: Vec<_> = run
            .app
            .world()
            .get::<crate::class_progression::KnownSpells>(healer)
            .unwrap()
            .iter()
            .filter(|spell| !excluded.contains(spell) && (void_shield || *spell != 17))
            .collect();
        if !void_shield && !spells.contains(&194509) {
            spells.push(194509);
        }
        run.app
            .world_mut()
            .entity_mut(healer)
            .insert(crate::class_progression::KnownSpells::fixture(spells));
        if void_shield {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: healer,
                    target: healer,
                    spell_id: 1253591,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        } else {
            for member in run
                .slots
                .into_iter()
                .filter(|&member| member != tank)
                .take(2)
            {
                let mut health = run.app.world_mut().get_mut::<Health>(member).unwrap();
                health.current = health.max * 0.6;
            }
        }
        let mut health = run.app.world_mut().get_mut::<Health>(tank).unwrap();
        health.current = health.max * 0.18;
        let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
        let expected = if void_shield { 1253593 } else { 194509 };
        for _ in 0..160 {
            run.app.update();
            if run.app.world().resource::<ObservedCombatEvents>().0[cursor..]
                .iter()
                .any(|event| {
                    event.kind == CombatLogKind::CastSuccess
                        && event.source == Some(healer.to_bits())
                        && event.target == Some(tank.to_bits())
                        && event.spell_id == Some(expected)
                })
            {
                break;
            }
        }
        let events = &run.app.world().resource::<ObservedCombatEvents>().0[cursor..];
        assert!(
            events
                .iter()
                .any(|event| event.kind == CombatLogKind::CastSuccess
                    && event.source == Some(healer.to_bits())
                    && event.target == Some(tank.to_bits())
                    && event.spell_id == Some(expected)),
            "critical tank must receive emergency spell {expected}: {events:?}"
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn discipline_emergency_heals_critical_tank_before_damage() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::components::Health;
    use shared::protocol::CombatLogKind;

    let mut run = super::healer_simc_replay::snapshot_party(256, 90, 1);
    super::healer_simc_replay::engage(&mut run);
    let (tank, healer) = (run.tank, run.healer);
    super::healer_simc_replay::finish_precombat(&mut run.app, &[tank, healer]).unwrap();
    let excluded = [17, 33206, 62618, 194509, 472433, 47540, 32379, 8092];
    let spells: Vec<_> = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(healer)
        .unwrap()
        .iter()
        .filter(|spell| !excluded.contains(spell))
        .collect();
    run.app
        .world_mut()
        .entity_mut(healer)
        .insert(crate::class_progression::KnownSpells::fixture(spells));
    for member in run.slots {
        crate::auras::remove_auras(
            run.app.world_mut(),
            member,
            crate::auras::RemoveReason::Consumed,
            |aura| aura.spell_id == 194384 && aura.caster == healer,
        );
    }
    crate::auras::apply_aura(
        run.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: healer,
            target: tank,
            spell_id: 194384,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    crate::auras::remove_auras(
        run.app.world_mut(),
        run.sources[0],
        crate::auras::RemoveReason::Consumed,
        |aura| aura.spell_id == 589 && aura.caster == healer,
    );
    let mut health = run.app.world_mut().get_mut::<Health>(tank).unwrap();
    health.current = health.max * 0.18;
    let before = health.current;
    let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
    for _ in 0..160 {
        run.app.update();
        if run.app.world().resource::<ObservedCombatEvents>().0[cursor..]
            .iter()
            .any(|event| {
                event.kind == CombatLogKind::Heal
                    && event.source == Some(healer.to_bits())
                    && event.target == Some(tank.to_bits())
                    && event.spell_id == Some(186263)
                    && event.amount > event.overflow
            })
        {
            break;
        }
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[cursor..];
    let save = events
        .iter()
        .position(|event| {
            event.kind == CombatLogKind::Heal
                && event.source == Some(healer.to_bits())
                && event.target == Some(tank.to_bits())
                && event.spell_id == Some(186263)
                && event.amount > event.overflow
        })
        .unwrap_or_else(|| panic!("the critical tank must receive Shadow Mend: {events:?}"));
    assert!(run.app.world().get::<Health>(tank).unwrap().current > before);
    assert!(
        !events[..save]
            .iter()
            .any(|event| event.kind == CombatLogKind::CastSuccess
                && event.source == Some(healer.to_bits())
                && matches!(event.spell_id, Some(585 | 589))),
        "healer must not choose damage before the emergency heal: {events:?}"
    );
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn discipline_shield_apl_prefers_atoned_ally_and_uses_void_proc() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    for darkness in [false, true] {
        let mut run = super::healer_simc_replay::snapshot_party(256, 90, 1);
        // The plain shield needs fight_remains within mana.pct*6: the 300 s
        // fixture the decision was tuned on, not a 45-minute dungeon route.
        run.app
            .insert_resource(server_follower_apl::registry::FixedLengthFight { duration: 300.0 });
        super::healer_simc_replay::engage(&mut run);
        let (tank, healer) = (run.tank, run.healer);
        let members = run.slots.to_vec();
        let ally = *members
            .iter()
            .find(|&&member| member != healer && member != tank)
            .unwrap();
        run.app
            .world_mut()
            .entity_mut(tank)
            .insert(crate::networking::CombatTarget(None));
        crate::auras::apply_aura(
            run.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: healer,
                target: ally,
                spell_id: 17,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        assert!(crate::auras::has_aura(run.app.world(), ally, 194384));
        if darkness {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: healer,
                    target: healer,
                    spell_id: 1253591,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        leave_only_shields(&mut run);
        let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
        let shielded = |run: &super::healer_simc_replay::ClassParty| {
            run.app.world().resource::<ObservedCombatEvents>().0[cursor..]
                .iter()
                .any(|event| {
                    event.kind == CombatLogKind::CastSuccess
                        && event.source == Some(healer.to_bits())
                        && matches!(event.spell_id, Some(17 | 1253593))
                })
        };
        for _ in 0..400 {
            if shielded(&run) {
                break;
            }
            run.app.update();
        }
        let events = &run.app.world().resource::<ObservedCombatEvents>().0[cursor..];
        let expected = if darkness { 1253593 } else { 17 };
        assert!(
            events
                .iter()
                .any(|event| event.kind == CombatLogKind::CastSuccess
                    && event.source == Some(healer.to_bits())
                    && event.spell_id == Some(expected)),
            "expected shield {expected} with darkness={darkness}: {events:?}"
        );
        assert!(
            events.iter().any(|event| matches!(
                event.kind,
                CombatLogKind::AuraApplied | CombatLogKind::AuraRefreshed
            ) && event.target == Some(ally.to_bits())
                && event.spell_id == Some(expected)),
            "shield must prefer the already Atoned ally: {events:?}"
        );
    }
}

/// Leave the shields as the healer's only castable actions while the class
/// build keeps its passives (Shield Discipline among them).
fn leave_only_shields(run: &mut super::healer_simc_replay::ClassParty) {
    let healer = run.healer;
    let competing = [
        33206, 62618, 194509, 472433, 47540, 2061, 589, 32379, 8092, 585,
    ];
    let mut known: Vec<_> = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(healer)
        .unwrap()
        .iter()
        .filter(|spell| !competing.contains(spell))
        .collect();
    for spell in [17, 1253593, 390691] {
        if !known.contains(&spell) {
            known.push(spell);
        }
    }
    run.app
        .world_mut()
        .entity_mut(healer)
        .insert(crate::class_progression::KnownSpells::fixture(known));
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot"]
fn discipline_plain_shield_follows_the_replay_fight_length() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::components::Mana;
    use shared::protocol::CombatLogKind;
    // At 10% mana the shield's gate is fight_remains <= 60 s (Shield
    // Discipline's mana.pct*6): a 30 s replay passes it, a 300 s one does not.
    for (duration, expected) in [(300.0, false), (30.0, true)] {
        let mut run = super::healer_simc_replay::snapshot_party(256, 90, 1);
        run.app
            .insert_resource(server_follower_apl::registry::FixedLengthFight { duration });
        super::healer_simc_replay::engage(&mut run);
        leave_only_shields(&mut run);
        let healer = run.healer;
        let shielded = |run: &mut super::healer_simc_replay::ClassParty| {
            let mut mana = run.app.world_mut().get_mut::<Mana>(healer).unwrap();
            mana.current = mana.max * 0.1;
            run.app
                .world()
                .resource::<ObservedCombatEvents>()
                .0
                .iter()
                .any(|event| {
                    event.kind == CombatLogKind::CastSuccess
                        && event.source == Some(healer.to_bits())
                        && event.spell_id == Some(17)
                })
        };
        for _ in 0..200 {
            if shielded(&mut run) {
                break;
            }
            run.app.update();
        }
        assert_eq!(
            shielded(&mut run),
            expected,
            "plain shield in a {duration} s replay"
        );
    }
}

#[test]
#[ignore = "class party: requires the FOLLOWER_DPS_WORLD_DB snapshot and SimC traces"]
fn discipline_priest_heals_simc_dummy_party() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::healer_simc_replay::replay_simc(256, 90, Path::new(&trace), 300, 70448).unwrap();
    let metrics = &report["metrics"];
    let effective = metrics["effective_hps"].as_f64().unwrap();
    let raw = metrics["raw_hps"].as_f64().unwrap();
    assert!(report["incoming_events"].as_u64().unwrap() > 100);
    assert!(effective > 0.0 && raw >= effective, "{report}");
    let casts = report["healer_casts"].as_object().unwrap();
    let total: u64 = casts.values().map(|value| value.as_u64().unwrap()).sum();
    assert!(
        total >= 200,
        "Discipline policy spent too much time idle: {report}"
    );
    assert!(
        casts
            .get("585")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0,
        "Discipline did not cast Smite: {report}"
    );
    assert!(
        metrics["spell_healing"]["94472"]["effective_hps"]
            .as_f64()
            .unwrap_or(0.0)
            > 0.0,
        "{report}"
    );
    assert!(
        casts
            .get("1291894")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0,
        "equipped Soulcoiler must be used by the replay: {report}"
    );
    assert!(
        casts
            .get("186263")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0,
        "selected Shadow Mend must cast while applying Atonement: {report}"
    );
    assert!(
        casts
            .get("32379")
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            > 0,
        "fixed-time execute must enable Shadow Word: Death: {report}"
    );
    assert_eq!(metrics["mana_curve"].as_array().unwrap().len(), 301);
    println!("NATIVE_DISCIPLINE_PRIEST={report}");
}
