//! Frozen source pressure, real native follower decisions and recipient mitigation.
use super::healer_intake::{Hit, HitKind, Intake};
use bevy::prelude::*;
use serde_json::{Value, json};
use shared::components::{Health, Position};
use std::collections::BTreeMap;

#[derive(Resource, Default)]
struct PendingHits {
    hits: Vec<(Entity, Entity, Hit)>,
    error: Option<String>,
    manual_raw_receipts: usize,
    replayed_hits: usize,
    non_unit_hits: usize,
}

fn deliver_hits(world: &mut World) {
    let hits = std::mem::take(&mut world.resource_mut::<PendingHits>().hits);
    for (source, target, hit) in hits {
        let before = world.resource::<super::healer_intake::Capture>().hits.len();
        let scale = crate::level_scaling::damage_to_hp_scale(world, source, target);
        if !scale.is_finite() || scale <= 0.0 {
            world.resource_mut::<PendingHits>().error =
                Some("NATIVE_REPLAY_LEVEL_SCALING_FACTOR_INVALID".into());
            return;
        }
        world
            .resource_mut::<super::healer_intake::Capture>()
            .active_hit_scale = Some((target, scale));
        {
            let mut pending = world.resource_mut::<PendingHits>();
            pending.replayed_hits += 1;
            pending.non_unit_hits += usize::from(scale != 1.0);
        }
        let result = match hit.kind {
            HitKind::Melee { .. } => crate::combat::replay_healer_hit(world, source, target, &hit),
            HitKind::Spell { .. } => {
                crate::spell_cast::effects::replay_healer_hit(world, source, target, &hit)
            }
        };
        world
            .resource_mut::<super::healer_intake::Capture>()
            .active_hit_scale = None;
        let added = world.resource::<super::healer_intake::Capture>().hits.len() - before;
        world.resource_mut::<PendingHits>().manual_raw_receipts += added;
        if let Err(error) = result {
            world.resource_mut::<PendingHits>().error = Some(error);
            return;
        }
    }
}

fn freeze_sources(world: &mut World, actors: &BTreeMap<u32, Entity>) {
    for &source in actors.values() {
        // The recorded clock, not a second autonomous dungeon, owns enemy actions.
        // Keep native identity/scaling/armor/auras and all recipient damage handlers.
        world
            .entity_mut(source)
            .remove::<(
                crate::creature_ai::CreatureState,
                crate::creature_aggro::AggroRange,
                crate::scripted_ai::CreatureAiState,
                crate::combat::AutoAttack,
                crate::networking::CombatTarget,
            )>()
            .insert(crate::combat::Unkillable);
    }
}

fn place_party(world: &mut World, members: &[Entity], positions: &[Option<[f32; 3]>]) {
    for (&unit, position) in members.iter().zip(positions) {
        if let Some([x, y, z]) = *position
            && world.get_entity(unit).is_ok()
        {
            world.entity_mut(unit).insert(Position { x, y, z });
        }
    }
}

fn incoming_count(world: &World, members: &[Entity], actors: &BTreeMap<u32, Entity>) -> usize {
    world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .iter()
        .filter(|event| {
            event.kind == shared::protocol::CombatLogKind::Damage
                && event
                    .target
                    .is_some_and(|bits| members.iter().any(|unit| unit.to_bits() == bits))
                && event
                    .source
                    .is_some_and(|bits| actors.values().any(|unit| unit.to_bits() == bits))
        })
        .count()
}

pub(super) fn recorded_combat_seconds(intake: &Intake) -> f64 {
    let mut frame = 0;
    let mut ticks = 0_u32;
    for tick in 1..=intake.end_tick {
        while frame + 1 < intake.frames.len() && intake.frames[frame + 1].tick <= tick {
            frame += 1;
        }
        ticks += u32::from(intake.frames[frame].in_combat);
    }
    f64::from(ticks) * 0.05
}

fn level_scaling_report(world: &World, healer: Entity, recorded: &[Hit]) -> Value {
    let usage = world
        .resource::<super::healer_intake::Capture>()
        .consumed_absorbs
        .get(&healer)
        .copied()
        .unwrap_or_default();
    let pending = world.resource::<PendingHits>();
    let non_unit = super::healer_intake::non_unit_hits(recorded);
    json!({"recorded_hits":recorded.len(),"recorded_unit_hits":recorded.len()-non_unit,
        "recorded_non_unit_hits":non_unit,"replayed_hits":pending.replayed_hits,"replayed_non_unit_hits":pending.non_unit_hits,
        "absorbed_points_on_non_unit_hits":usage.non_unit,"absorbed_points_unknown_scale":usage.unknown_scale,
        "absorbed_points_non_unit_share":(usage.total>0.0).then(||usage.non_unit/usage.total),
        "gap_moot_for_intake":!recorded.is_empty() && non_unit==0 && pending.non_unit_hits==0 && usage.unknown_scale==0.0,
        "factor_definition":"native damage multiplier / victim health multiplier, before integer rounding",
        "absorb_convention":"native consumed shield points, not normalized prevented HP"})
}

#[test]
#[ignore = "RED: non-unit consumed-point attribution missing"]
fn healer_benchmark_counts_nonunit_hit_absorbed_point_share() {
    let _rng = crate::random::seed_scope(1);
    let mut app = super::runtime::build_app();
    let copy = super::runtime::create_copy(&mut app);
    let actors = super::runtime::spawn_population(&mut app, copy);
    let healer = crate::spell_cast::test_harness::spawn_player(&mut app, 5, 0.0);
    app.world_mut()
        .entity_mut(healer)
        .insert((shared::components::UnitLevel(10), copy));
    let (&guid, &source) = actors
        .iter()
        .find(|&(_, &unit)| {
            let factor = crate::level_scaling::damage_to_hp_scale(app.world(), unit, healer);
            factor.is_finite()
                && factor > 0.0
                && factor != 1.0
                && crate::spell_cast::targets::relation_matches(
                    app.world(),
                    unit,
                    healer,
                    crate::spell_cast::targets::Relation::Enemy,
                )
        })
        .expect("real Stockade10..30 curve against level10 fixture");
    let graph = &app.world().resource::<super::Space>().0;
    let pack = graph
        .packs
        .iter()
        .find(|pack| pack.spawns.iter().any(|spawn| spawn.guid == guid))
        .unwrap()
        .id;
    app.insert_resource(super::healer_intake::Capture {
        members: vec![healer],
        sources: [(source, (guid, pack))].into(),
        ..Default::default()
    });
    crate::auras::apply_aura(
        app.world_mut(),
        &crate::auras::AuraApplication {
            caster: healer,
            target: healer,
            spell_id: 17,
            effect_mask: 1,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let attacker = crate::level_scaling::UnitScaling::of(app.world(), source);
    let victim = crate::level_scaling::UnitScaling::of(app.world(), healer);
    let difference =
        i32::from(victim.level_against(&attacker)) - i32::from(attacker.level_against(&victim));
    super::healer_intake::record(
        app.world_mut(),
        source,
        healer,
        1,
        50.0,
        HitKind::Melee {
            hand: 0,
            outcome: "Hit".into(),
            level_difference: difference,
            critical_multiplier: 1.0,
            weapon_speed: crate::combat::AUTO_ATTACK_INTERVAL,
        },
    );
    let recorded = app
        .world()
        .resource::<super::healer_intake::Capture>()
        .hits
        .clone();
    // Persisted intake enters a fresh observation context, not the recording hit's tag.
    app.insert_resource(super::healer_intake::Capture {
        members: vec![healer],
        sources: [(source, (guid, pack))].into(),
        ..Default::default()
    });
    app.insert_resource(PendingHits {
        hits: vec![(source, healer, recorded[0].clone())],
        ..Default::default()
    });
    deliver_hits(app.world_mut());
    assert!(app.world().resource::<PendingHits>().error.is_none());
    let summary = level_scaling_report(app.world(), healer, &recorded);
    assert_eq!(summary["recorded_non_unit_hits"], 1);
    assert_eq!(summary["replayed_non_unit_hits"], 1);
    assert_eq!(summary["absorbed_points_on_non_unit_hits"], 50.0);
    assert_eq!(summary["absorbed_points_non_unit_share"], 1.0);
    assert_eq!(summary["gap_moot_for_intake"], false);
}

pub(super) fn replay(intake: &Intake, spec: u32) -> Result<Value, String> {
    if intake.schema != 1 || intake.map_id != 34 || intake.difficulty_id != 1 {
        return Err("INTAKE_SCHEMA_OR_STOCKADE_NORMAL_COPY_MISMATCH".into());
    }
    if !intake.gaps.is_empty() {
        return Err(format!("INCOMPLETE_RAW_INTAKE:{:?}", intake.gaps));
    }
    if intake.frames.is_empty()
        || intake
            .hits
            .windows(2)
            .any(|pair| pair[0].tick > pair[1].tick)
        || intake.hits.iter().any(|hit| {
            hit.tick == 0
                || hit.tick > intake.end_tick
                || !hit.raw.is_finite()
                || hit.raw < 0.0
                || !hit.level_scaling_factor.is_finite()
                || hit.level_scaling_factor <= 0.0
        })
    {
        return Err("INVALID_INTAKE_TIMELINE".into());
    }
    let _rng = crate::random::seed_scope(intake.seed);
    let mut app = super::runtime::build_app();
    let graph = app.world().resource::<super::Space>().0.clone();
    if graph.map_id != intake.map_id || graph.difficulty_id() != intake.difficulty_id {
        return Err("NATIVE_COPY_DIFFERS_FROM_RECORDED_INTAKE".into());
    }
    let copy = super::runtime::create_copy(&mut app);
    let party = crate::followers::simulator::spawn_party_with_healer(
        app.world_mut(),
        copy,
        Position {
            x: graph.entrance[0],
            y: graph.entrance_height,
            z: graph.entrance[1],
        },
        intake.level,
        spec,
    )?;
    let actors = super::runtime::spawn_population(&mut app, copy);
    let members: Vec<_> = party.members.iter().copied().chain([party.owner]).collect();
    if intake
        .frames
        .iter()
        .any(|frame| frame.positions.len() != members.len())
        || intake
            .hits
            .iter()
            .any(|hit| hit.target_slot >= members.len() || !actors.contains_key(&hit.source_guid))
    {
        return Err("RECORDED_PARTY_SLOT_OR_NATIVE_SOURCE_ABSENT".into());
    }
    if intake.hits.iter().any(|hit| !intake.sources.contains_key(&hit.source_guid)
        || matches!(hit.kind, HitKind::Spell { bonus_source_guid: Some(guid), .. } if !intake.sources.contains_key(&guid))) {
        return Err("RECORDED_SOURCE_SCALING_METADATA_ABSENT".into());
    }
    super::healer_intake::apply_source_snapshots(app.world_mut(), &intake.sources, &actors)?;
    freeze_sources(app.world_mut(), &actors);
    let mut recorder = super::healer_metrics::start(app.world_mut(), &graph, &party, &actors);
    app.init_resource::<PendingHits>();
    app.add_systems(
        FixedUpdate,
        deliver_hits
            .before(crate::creature_aggro::update_victim_system)
            .before(crate::followers::update_ai),
    );
    let mut next_hit = 0;
    let mut frame = 0;
    let mut active_sources = std::collections::BTreeSet::new();
    for tick in 1..=intake.end_tick {
        while frame + 1 < intake.frames.len() && intake.frames[frame + 1].tick < tick {
            frame += 1;
        }
        place_party(app.world_mut(), &members, &intake.frames[frame].positions);
        if !intake.frames[frame].in_combat {
            for &unit in members.iter().chain(active_sources.iter()) {
                if app.world().get_entity(unit).is_ok() {
                    crate::combat::combat_stop(app.world_mut(), unit);
                }
            }
            active_sources.clear();
        }
        while let Some(hit) = intake.hits.get(next_hit).filter(|hit| hit.tick == tick) {
            let source = actors[&hit.source_guid];
            let target = members[hit.target_slot];
            if app
                .world()
                .get::<Health>(target)
                .is_some_and(|health| health.current > 0.0)
            {
                active_sources.insert(source);
                // CombatTarget re-arms native creature auto-attacks independently of AI.
                // Combat refs admit companion decisions without starting a second hit clock.
                crate::combat_refs::set_in_combat_with(app.world_mut(), party.owner, source);
                app.world_mut().resource_mut::<PendingHits>().hits.push((
                    source,
                    target,
                    hit.clone(),
                ));
            }
            next_hit += 1;
        }
        app.update();
        if let Some(error) = app.world_mut().resource_mut::<PendingHits>().error.take() {
            return Err(error);
        }
        if !app.world().resource::<PendingHits>().hits.is_empty() {
            return Err("REPLAY_FIXED_TICK_NOT_EXECUTED".into());
        }
        let capture = app.world().resource::<super::healer_intake::Capture>();
        let expected = app.world().resource::<PendingHits>().manual_raw_receipts;
        if capture.hits.len() != expected {
            return Err(format!(
                "UNRECORDED_NATIVE_INCOMING_DAMAGE:tick={tick}:observed={}:manual={expected}:recent={:?}",
                capture.hits.len(),
                capture.hits.iter().rev().take(8).collect::<Vec<_>>()
            ));
        }
        recorder.observe_frame(app.world());
        for &(_, guid) in intake.source_deaths.iter().filter(|(at, _)| *at == tick) {
            if let Some(&source) = actors.get(&guid) {
                crate::combat::combat_stop(app.world_mut(), source);
                app.world_mut()
                    .entity_mut(source)
                    .remove::<crate::networking::CombatTarget>();
                if let Some(mut health) = app.world_mut().get_mut::<Health>(source) {
                    health.current = 0.0;
                }
                active_sources.remove(&source);
            }
        }
    }
    let observations = recorder.report(app.world());
    let mut metrics = observations["dungeon_healing"].clone();
    let recorded_combat = recorded_combat_seconds(intake);
    metrics["native_combat_seconds"] = metrics["combat_seconds"].clone();
    metrics["recorded_combat_seconds"] = json!(recorded_combat);
    metrics["effective_hps"] = json!(
        (recorded_combat > 0.0)
            .then(|| metrics["effective_healing"].as_f64().unwrap() / recorded_combat)
    );
    metrics["effective_throughput_hps"] = json!((recorded_combat > 0.0).then(|| {
        metrics["effective_healing_plus_absorbed_damage"]
            .as_f64()
            .unwrap()
            / recorded_combat
    }));
    metrics["throughput_denominator"] = json!(
        "frozen intake recorded party-combat window; identical for all specs including early wipes"
    );
    let scaling = level_scaling_report(app.world(), party.members[1], &intake.hits);
    let mut gaps = vec![
        "RECORDED_FUTURE_DAMAGE_ALERTS_NOT_REPLAYED",
        "RECORDED_SOURCE_GEOMETRY_NOT_REPLAYED",
    ];
    if scaling["gap_moot_for_intake"] == false {
        gaps.push("ABSORB_POINTS_VS_PREVENTED_HP_LEVEL_SCALING");
    }
    if intake.hits.iter().any(|hit| hit.school & !1 != 0) {
        gaps.push("NATIVE_PARTIAL_SPELL_RESISTANCE_UNIMPLEMENTED");
    }
    // Any autonomous source damage would invalidate identical-input qualification.
    let observed_raw = app
        .world()
        .resource::<super::healer_intake::Capture>()
        .hits
        .len();
    let expected_raw = app.world().resource::<PendingHits>().manual_raw_receipts;
    if observed_raw != expected_raw {
        return Err("UNRECORDED_NATIVE_INCOMING_DAMAGE".into());
    }
    Ok(json!({"spec":spec,"status":"native","seed":intake.seed,
        "metrics":metrics,"gaps":gaps,"level_scaling":scaling,
        "intake_hits":intake.hits.len(),"native_damage_receipts":incoming_count(app.world(), &members, &actors),
        "source_status":intake.source_status,"duration_ticks":intake.end_tick}))
}
