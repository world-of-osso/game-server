//! Isolated tank decisions under a frozen native incoming-damage recording.
use super::healer_intake::{Capture, Hit, HitKind, Intake};
use super::healer_replay::{PendingHits, deliver_hits, freeze_sources};
use bevy::prelude::*;
use serde_json::{Value, json};
use shared::components::{Health, Position};
use shared::protocol::CombatLogKind;
use std::collections::{BTreeSet, VecDeque};

const TICK_SECONDS: f64 = 0.05;
const SPIKE_WINDOW_SECONDS: f64 = 5.0;
const SHIELD_OF_THE_RIGHTEOUS: u32 = 132403;

fn source_amount(hit: &Hit) -> Result<f64, String> {
    let HitKind::Melee {
        ref outcome,
        level_difference,
        critical_multiplier,
        ..
    } = hit.kind
    else {
        return Ok(f64::from(hit.raw));
    };
    use crate::melee::MeleeOutcome;
    let kind = match outcome.as_str() {
        "Hit" => MeleeOutcome::Hit,
        "Crit" => MeleeOutcome::Crit,
        "Glancing" => MeleeOutcome::Glancing,
        "Block" => MeleeOutcome::Block,
        "Miss" => MeleeOutcome::Miss,
        "Dodge" => MeleeOutcome::Dodge,
        "Parry" => MeleeOutcome::Parry,
        _ => return Err(format!("RECORDED_MELEE_OUTCOME_UNSUPPORTED:{outcome}")),
    };
    Ok(f64::from(
        crate::melee::outcome_damage(hit.raw, kind, level_difference, 0.0).0 * critical_multiplier,
    ))
}

pub(super) fn replay(intake: &Intake) -> Result<Value, String> {
    if intake.schema != 1
        || intake.map_id != 34
        || intake.difficulty_id != 1
        || !intake.gaps.is_empty()
        || intake.frames.is_empty()
        || intake
            .hits
            .windows(2)
            .any(|pair| pair[0].tick > pair[1].tick)
    {
        return Err("INVALID_RECORDED_TANK_INTAKE".into());
    }
    let _rng = crate::random::seed_scope(intake.seed);
    let mut app = super::runtime::build_app();
    let graph = app.world().resource::<super::Space>().0.clone();
    let copy = super::runtime::create_copy(&mut app);
    let tank = crate::followers::simulator::spawn_parity_attacker(
        app.world_mut(),
        66,
        intake.level,
        copy,
        crate::followers::simulator::ParityActor::Follower,
    )?;
    let actors = super::runtime::spawn_population(&mut app, copy);
    super::healer_intake::apply_source_snapshots(app.world_mut(), &intake.sources, &actors)?;
    freeze_sources(app.world_mut(), &actors);
    let sources = graph
        .packs
        .iter()
        .flat_map(|pack| {
            pack.spawns
                .iter()
                .map(|spawn| (actors[&spawn.guid], (spawn.guid, pack.id)))
        })
        .collect();
    app.insert_resource(Capture {
        members: vec![tank],
        sources,
        ..Default::default()
    });
    app.init_resource::<PendingHits>();
    app.add_systems(
        FixedUpdate,
        deliver_hits
            .before(crate::creature_aggro::update_victim_system)
            .before(crate::followers::update_ai),
    );
    let source_entities: BTreeSet<_> = actors.values().map(|entity| entity.to_bits()).collect();
    let mut next_hit = 0;
    let mut frame = 0;
    let mut active_sources = BTreeSet::new();
    let mut raw_cursor = 0;
    let mut event_cursor = app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    let (mut raw, mut taken, mut self_healing, mut active_seconds, mut spike) =
        (0.0, 0.0, 0.0, 0.0, 0.0_f64);
    let (mut recent, mut recent_sum) = (VecDeque::<(f64, f64)>::new(), 0.0);
    let mut death = None;
    let mut elapsed = 0.0;
    for tick in 1..=intake.end_tick {
        while frame + 1 < intake.frames.len() && intake.frames[frame + 1].tick < tick {
            frame += 1;
        }
        let position = intake.frames[frame]
            .positions
            .first()
            .and_then(|position| *position)
            .ok_or("RECORDED_TANK_POSITION_ABSENT")?;
        app.world_mut().entity_mut(tank).insert(Position {
            x: position[0],
            y: position[1],
            z: position[2],
        });
        if !intake.frames[frame].in_combat {
            crate::combat::combat_stop(app.world_mut(), tank);
            for source in std::mem::take(&mut active_sources) {
                crate::combat::combat_stop(app.world_mut(), source);
            }
        }
        while let Some(hit) = intake.hits.get(next_hit).filter(|hit| hit.tick == tick) {
            if hit.target_slot == 0 {
                let source = *actors
                    .get(&hit.source_guid)
                    .ok_or("RECORDED_SOURCE_ABSENT")?;
                active_sources.insert(source);
                crate::combat_refs::set_in_combat_with(app.world_mut(), tank, source);
                app.world_mut().resource_mut::<PendingHits>().hits.push((
                    source,
                    tank,
                    hit.clone(),
                ));
            }
            next_hit += 1;
        }
        app.update();
        if let Some(error) = app.world_mut().resource_mut::<PendingHits>().error.take() {
            return Err(error);
        }
        let world = app.world();
        let recorded = &world.resource::<Capture>().hits;
        for hit in &recorded[raw_cursor..] {
            raw += source_amount(hit)?;
        }
        raw_cursor = recorded.len();
        elapsed = f64::from(tick) * TICK_SECONDS;
        if crate::auras::has_aura(world, tank, SHIELD_OF_THE_RIGHTEOUS) {
            active_seconds += TICK_SECONDS;
        }
        let events = &world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for event in &events[event_cursor..] {
            if event.target != Some(tank.to_bits()) {
                continue;
            }
            if event.kind == CombatLogKind::Damage
                && event
                    .source
                    .is_some_and(|source| source_entities.contains(&source))
            {
                let amount = f64::from(event.amount.max(0));
                taken += amount;
                recent.push_back((elapsed, amount));
                recent_sum += amount;
                while recent
                    .front()
                    .is_some_and(|(time, _)| *time <= elapsed - SPIKE_WINDOW_SECONDS)
                {
                    recent_sum -= recent.pop_front().unwrap().1;
                }
                spike = spike.max(recent_sum);
            }
            if event.kind == CombatLogKind::Heal && event.source == Some(tank.to_bits()) {
                self_healing += f64::from((event.amount - event.overflow.max(0)).max(0));
            }
        }
        event_cursor = events.len();
        if world
            .get::<Health>(tank)
            .is_none_or(|health| health.current <= 0.0)
        {
            death = Some(elapsed);
            break;
        }
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
    let absorbed = app
        .world()
        .resource::<Capture>()
        .consumed_absorbs
        .get(&tank)
        .map_or(0.0, |usage| usage.total);
    Ok(
        json!({"spec":66,"source_status":intake.source_status,"duration_seconds":elapsed,
        "raw_damage":raw,"damage_taken":taken,"mitigation_pct":(raw > 0.0).then(|| 100.0 * (raw - taken) / raw),
        "self_heal_per_second":(elapsed > 0.0).then(|| self_healing / elapsed),
        "absorb_per_second":(elapsed > 0.0).then(|| absorbed / elapsed),"time_to_death":death,
        "active_mitigation_spell":SHIELD_OF_THE_RIGHTEOUS,"active_mitigation_uptime":(elapsed > 0.0).then(|| active_seconds / elapsed),
        "spike_window_seconds":SPIKE_WINDOW_SECONDS,"largest_spike":spike}),
    )
}
