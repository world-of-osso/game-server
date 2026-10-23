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
}

fn deliver_hits(world: &mut World) {
    let hits = std::mem::take(&mut world.resource_mut::<PendingHits>().hits);
    for (source, target, hit) in hits {
        let before = world.resource::<super::healer_intake::Capture>().hits.len();
        let result = match hit.kind {
            HitKind::Melee { .. } => crate::combat::replay_healer_hit(world, source, target, &hit),
            HitKind::Spell { .. } => {
                crate::spell_cast::effects::replay_healer_hit(world, source, target, &hit)
            }
        };
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
            hit.tick == 0 || hit.tick > intake.end_tick || !hit.raw.is_finite() || hit.raw < 0.0
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
                app.world_mut()
                    .entity_mut(source)
                    .insert(crate::networking::CombatTarget(Some(target)));
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
    let mut gaps = vec!["RECORDED_FUTURE_DAMAGE_ALERTS_NOT_REPLAYED"];
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
        "metrics":metrics,"gaps":gaps,
        "intake_hits":intake.hits.len(),"native_damage_receipts":incoming_count(app.world(), &members, &actors),
        "source_status":intake.source_status,"duration_ticks":intake.end_tick}))
}
