use std::path::Path;

#[test]
#[ignore = "requires the selected real Discipline gear and canonical world database"]
fn discipline_soulcoiler_real_item_channels_five_fractional_barriers() {
    use super::super::dps_parity::literal_rotation::{
        Driver, equipment::Equipment, execution::Prepared,
    };
    use crate::simc_apl_native::Bindings;
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use server::simc_apl::Program;
    use shared::protocol::CombatLogKind;
    use std::collections::BTreeMap;

    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let tank = run.actor;
    let healer = super::super::dps_parity::spawn_second_reference_player(&mut run, 256).unwrap();
    let party = crate::followers::simulator::spawn_party_for_reference_healer(
        run.app.world_mut(),
        tank,
        healer,
    )
    .unwrap();
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
        &run.targets,
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
    let mut members = party.members.clone();
    members.push(tank);
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
        caster: run.targets[0],
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
#[ignore = "requires selected Discipline profile and native 70448 data"]
fn equipped_seastone_procs_intellect_and_reverses_after_six_stacks() {
    use super::super::dps_parity::literal_rotation::execution::{Execution, execute};
    use crate::auras::ActiveAuras;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let healer = super::super::dps_parity::spawn_second_reference_player(&mut run, 256).unwrap();
    let target = run.targets[0];
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
#[ignore = "requires selected Discipline profile and native 70448 data"]
fn discipline_pain_observes_power_of_dark_side_procs() {
    use super::super::dps_parity::literal_rotation::execution::{Execution, execute};
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let _seed = crate::random::seed_scope(70448);
    let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let healer = super::super::dps_parity::spawn_second_reference_player(&mut run, 256).unwrap();
    let target = run.targets[0];
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
#[ignore = "requires selected Discipline profile and native 70448 data"]
fn discipline_smite_and_penance_capture_matched_stat_factors() {
    use super::super::dps_parity::literal_rotation::execution::{Execution, execute};
    use crate::spell_cast::effects::factor_capture::FactorCapture;
    for (root, focus) in [(585, 585), (47540, 47666)] {
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
        let healer =
            super::super::dps_parity::spawn_second_reference_player(&mut run, 256).unwrap();
        let target = run.targets[0];
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
#[ignore = "requires the selected real Discipline profile and canonical world database"]
fn discipline_policy_applies_pain_and_does_not_refresh_early() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let tank = run.actor;
    let healer = super::super::dps_parity::spawn_second_reference_player(&mut run, 256).unwrap();
    let party = crate::followers::simulator::spawn_party_for_reference_healer(
        run.app.world_mut(),
        tank,
        healer,
    )
    .unwrap();
    let mut members = party.members.clone();
    members.push(tank);
    let enemy = run.targets[0];
    run.app
        .world_mut()
        .entity_mut(tank)
        .insert(crate::networking::CombatTarget(Some(enemy)));
    let mut policy = super::prepare(&mut run, healer).unwrap();
    policy.drive(run.app.world_mut(), healer, &members).unwrap();
    for _ in 0..40 {
        run.app.update();
    }
    assert!(
        crate::auras::has_aura(run.app.world(), enemy, 589),
        "Pain must apply to the enemy"
    );
    let casts = |run: &super::super::dps_parity::Run| {
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
    policy.drive(run.app.world_mut(), healer, &members).unwrap();
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
#[ignore = "requires the selected real Discipline profile and canonical world database"]
fn discipline_policy_uses_evangelism_without_injury_gate() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
    let tank = run.actor;
    let healer = super::super::dps_parity::spawn_second_reference_player(&mut run, 256).unwrap();
    let party = crate::followers::simulator::spawn_party_for_reference_healer(
        run.app.world_mut(),
        tank,
        healer,
    )
    .unwrap();
    let mut members = party.members.clone();
    members.push(tank);
    run.app
        .world_mut()
        .entity_mut(tank)
        .insert(crate::networking::CombatTarget(None));
    let mut policy = super::prepare(&mut run, healer).unwrap();
    let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
    policy.drive(run.app.world_mut(), healer, &members).unwrap();
    for _ in 0..30 {
        run.app.update();
    }
    let events = &run.app.world().resource::<ObservedCombatEvents>().0[cursor..];
    assert!(
        events
            .iter()
            .any(|event| event.kind == CombatLogKind::CastSuccess
                && event.source == Some(healer.to_bits())
                && event.spell_id == Some(472433)),
        "Evangelism must not require injured allies: {events:?}"
    );
    assert!(
        members
            .iter()
            .any(|&member| crate::auras::has_aura(run.app.world(), member, 194384)),
        "Evangelism's Radiance must apply Atonement"
    );
    // The selected profile does not learn Radiance; add it to exercise this APL branch.
    let mut spells: Vec<_> = run
        .app
        .world()
        .get::<crate::class_progression::KnownSpells>(healer)
        .unwrap()
        .iter()
        .collect();
    spells.push(194509);
    run.app
        .world_mut()
        .entity_mut(healer)
        .insert(crate::class_progression::KnownSpells::fixture(spells));
    policy.drive(run.app.world_mut(), healer, &members).unwrap();
    for _ in 0..50 {
        run.app.update();
    }
    assert!(
        run.app.world().resource::<ObservedCombatEvents>().0[cursor..]
            .iter()
            .any(|event| event.kind == CombatLogKind::CastSuccess
                && event.source == Some(healer.to_bits())
                && event.spell_id == Some(194509)),
        "Radiance must follow under the Mind Blast cooldown condition: {:?}",
        &run.app.world().resource::<ObservedCombatEvents>().0[cursor..]
    );
}

#[test]
#[ignore = "requires the selected real Discipline profile and canonical world database"]
fn discipline_shield_policy_prefers_atoned_ally_and_uses_void_proc() {
    use crate::spell_cast::test_harness::ObservedCombatEvents;
    use shared::protocol::CombatLogKind;
    for darkness in [false, true] {
        let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
        let tank = run.actor;
        let healer =
            super::super::dps_parity::spawn_second_reference_player(&mut run, 256).unwrap();
        let party = crate::followers::simulator::spawn_party_for_reference_healer(
            run.app.world_mut(),
            tank,
            healer,
        )
        .unwrap();
        let mut members = party.members.clone();
        members.push(tank);
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
        run.app.world_mut().entity_mut(healer).insert(
            crate::class_progression::KnownSpells::fixture(vec![17, 1253593, 390691]),
        );
        let mut policy = super::prepare(&mut run, healer).unwrap();
        let cursor = run.app.world().resource::<ObservedCombatEvents>().0.len();
        policy.drive(run.app.world_mut(), healer, &members).unwrap();
        run.app.update();
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

#[test]
#[ignore = "requires MID2 profiles, canonical world database and SimC trace"]
fn discipline_priest_heals_simc_dummy_party() {
    let trace = std::env::var("HEALER_TANK_SIMC_TRACE").expect("HEALER_TANK_SIMC_TRACE required");
    let report =
        super::super::healer_replay::replay_simc(256, Path::new(&trace), 300, 70448).unwrap();
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
