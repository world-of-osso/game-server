//! Isolated tank decisions under a frozen native incoming-damage recording.
#[path = "protection_rotation.rs"]
pub(super) mod protection_rotation;
use super::healer_intake::{Capture, Hit, HitKind, Intake};
use super::healer_replay::{PendingHits, deliver_hits, freeze_sources};
use bevy::prelude::*;
use serde_json::{Value, json};
use shared::components::{Health, Position};
use shared::protocol::CombatLogKind;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const TICK_SECONDS: f64 = 0.05;
const SPIKE_WINDOW_SECONDS: f64 = 5.0;
const SHIELD_OF_THE_RIGHTEOUS: u32 = 132403;

pub(super) fn source_amount(hit: &Hit) -> Result<f64, String> {
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
    if matches!(
        kind,
        MeleeOutcome::Miss | MeleeOutcome::Dodge | MeleeOutcome::Parry
    ) {
        return Ok(f64::from(hit.raw));
    }
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
    app.world_mut()
        .resource_mut::<PendingHits>()
        .track_delivered_hits = true;
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
    let mut event_cursor = app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    let (mut raw, mut taken, mut self_healing, mut active_seconds, mut spike) =
        (0.0, 0.0, 0.0, 0.0, 0.0_f64);
    let (mut recent, mut recent_sum) = (VecDeque::<(f64, f64)>::new(), 0.0);
    let mut death = None;
    let mut casts = BTreeMap::<u32, usize>::new();
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
        let delivered =
            std::mem::take(&mut app.world_mut().resource_mut::<PendingHits>().delivered_hits);
        for hit in &delivered {
            raw += source_amount(hit)?;
        }
        let world = app.world();
        let recorded = world.resource::<Capture>().hits.len();
        let manual = world.resource::<PendingHits>().manual_raw_receipts;
        if recorded != manual {
            return Err(format!(
                "UNRECORDED_NATIVE_INCOMING_DAMAGE:tick={tick}:observed={recorded}:manual={manual}"
            ));
        }
        elapsed = f64::from(tick) * TICK_SECONDS;
        if crate::auras::has_aura(world, tank, SHIELD_OF_THE_RIGHTEOUS) {
            active_seconds += TICK_SECONDS;
        }
        let events = &world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for event in &events[event_cursor..] {
            if event.source == Some(tank.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && let Some(spell_id) = event.spell_id
            {
                *casts.entry(spell_id).or_default() += 1;
            }
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
        "spike_window_seconds":SPIKE_WINDOW_SECONDS,"largest_spike":spike,"tank_casts":casts}),
    )
}

/// Parse every action row from the pinned MID2 Protection profile.
pub(super) fn protection_program(actions: &Value) -> Result<server::simc_apl::Program, String> {
    let lines = actions.as_array().ok_or("MID2_PROTECTION_ACTIONS_ABSENT")?;
    let mut rows = Vec::with_capacity(lines.len());
    for line in lines {
        let line = line.as_str().ok_or("MID2_PROTECTION_ACTION_NOT_TEXT")?;
        let (head, action) = line
            .split_once("+=")
            .or_else(|| line.split_once('='))
            .ok_or_else(|| format!("invalid MID2 Protection action: {line}"))?;
        let list = if head == "actions" {
            "default"
        } else {
            head.strip_prefix("actions.")
                .ok_or_else(|| format!("invalid MID2 Protection list: {head}"))?
        };
        rows.push((list, action.strip_prefix('/').unwrap_or(action)));
    }
    server::simc_apl::Program::parse(&rows)
}

/// Run SimC's pre-outcome dummy pressure against the actual MID2 parity player.
pub(super) fn replay_simc(
    spec_id: u32,
    path: &std::path::Path,
    duration_seconds: u32,
    seed: u64,
) -> Result<Value, String> {
    use crate::simc_tank_timeline;
    let canonical =
        std::path::Path::new("/home/osso/Projects/world-of-osso/game-server/data/world.db")
            .canonicalize()
            .map_err(|error| format!("canonical world database: {error}"))?;
    let selected = std::env::var_os("DPS_PARITY_WORLD_DB").ok_or("DPS_PARITY_WORLD_DB absent")?;
    if std::path::Path::new(&selected)
        .canonicalize()
        .map_err(|error| error.to_string())?
        != canonical
        || crate::test_data::world_db_path()
            .canonicalize()
            .map_err(|error| error.to_string())?
            != canonical
    {
        return Err("SIMC_REPLAY_REQUIRES_CANONICAL_WORLD_DB".into());
    }
    if duration_seconds != 300 {
        return Err("NATIVE_LITERAL_ENCOUNTER_REQUIRES_300_SECONDS".into());
    }
    let hits = simc_tank_timeline::read_trace(path, "Tank_Dummy_Dungeon")?;
    let _rng = crate::random::seed_scope(seed);
    let run = super::dps_parity::prepare_registered_player(spec_id, 1)?;
    let auras = super::dps_parity::tank_mitigation_auras(&run)?;
    replay_registered_run(run, &hits, duration_seconds, auras)
}

pub(super) fn replay_registered_run(
    mut run: super::dps_parity::Run,
    hits: &[crate::simc_tank_timeline::Hit],
    duration_seconds: u32,
    mitigation_auras: &[u32],
) -> Result<Value, String> {
    use crate::simc_tank_timeline::Kind;
    let tank = run.actor;
    let source = run.targets[0];
    let spec = run
        .app
        .world()
        .get::<shared::components::ActiveSpec>(tank)
        .ok_or("NATIVE_PLAYER_SPEC_ABSENT")?
        .0;
    let level = run
        .app
        .world()
        .get::<shared::components::Level>(tank)
        .ok_or("NATIVE_PLAYER_LEVEL_ABSENT")?
        .0;
    if level != 90 {
        return Err(format!("MID2_PLAYER_LEVEL_MISMATCH:{level}"));
    }
    super::dps_parity::engage(&mut run);
    freeze_sources(run.app.world_mut(), &[(0, source)].into());
    run.app.insert_resource(Capture {
        members: vec![tank],
        sources: [(source, (0, 0))].into(),
        ..Default::default()
    });
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), tank, source);
    let mut event_cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    let (mut raw, mut taken, mut self_healing, mut active_seconds, mut spike) =
        (0.0, 0.0, 0.0, 0.0, 0.0_f64);
    let (mut recent, mut recent_sum) = (VecDeque::<(f64, f64)>::new(), 0.0);
    let (mut next, mut elapsed, mut death) = (0, 0.0, None);
    let mut casts = BTreeMap::<u32, usize>::new();
    for tick in 1..=duration_seconds * 20 {
        elapsed = f64::from(tick) * TICK_SECONDS;
        while let Some(hit) = hits
            .get(next)
            .filter(|hit| hit.time_ms <= u64::from(tick) * 50)
        {
            if !run
                .app
                .world()
                .get::<Health>(tank)
                .is_some_and(|health| health.current > 0.0)
            {
                break;
            }
            let amount = hit.amount as f32;
            if !amount.is_finite() {
                return Err("SIMC_DUMMY_AMOUNT_OUT_OF_RANGE".into());
            }
            let scale = crate::level_scaling::damage_to_hp_scale(run.app.world(), source, tank);
            run.app
                .world_mut()
                .resource_mut::<Capture>()
                .active_hit_scale = Some((tank, scale));
            match hit.kind {
                Kind::Bleed => {
                    raw += f64::from(amount);
                    crate::spell_cast::effects::replay_simc_dummy_bleed(
                        run.app.world_mut(),
                        source,
                        tank,
                        amount,
                    );
                }
                Kind::Swing | Kind::Strike => {
                    let (outcome, level_difference) = crate::combat::roll_simc_dummy_outcome(
                        run.app.world(),
                        source,
                        tank,
                        hit.kind == Kind::Strike,
                    )?;
                    let recorded = Hit {
                        tick,
                        source_guid: 0,
                        pack: 0,
                        target_slot: 0,
                        school: 1,
                        raw: amount,
                        level_scaling_factor: scale,
                        kind: HitKind::Melee {
                            hand: 0,
                            outcome: format!("{outcome:?}"),
                            level_difference,
                            critical_multiplier: 1.0,
                            weapon_speed: 2.0,
                        },
                    };
                    raw += source_amount(&recorded)?;
                    crate::combat::replay_healer_hit(run.app.world_mut(), source, tank, &recorded)?;
                }
            }
            run.app
                .world_mut()
                .resource_mut::<Capture>()
                .active_hit_scale = None;
            next += 1;
        }
        super::dps_parity::drive_literal_tick(&mut run)?;
        run.app.update();
        let world = run.app.world();
        if mitigation_auras
            .iter()
            .any(|&aura| crate::auras::has_aura(world, tank, aura))
        {
            active_seconds += TICK_SECONDS;
        }
        let events = &world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for event in &events[event_cursor..] {
            if event.source == Some(tank.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && let Some(spell_id) = event.spell_id
            {
                *casts.entry(spell_id).or_default() += 1;
            }
            if event.target != Some(tank.to_bits()) {
                continue;
            }
            if event.kind == CombatLogKind::Damage && event.source == Some(source.to_bits()) {
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
    }
    let absorbed = run
        .app
        .world()
        .resource::<Capture>()
        .consumed_absorbs
        .get(&tank)
        .map_or(0.0, |usage| usage.total);
    Ok(
        json!({"spec":spec,"level":level,"reference_tier":"MID2","timeline":"simc_tank_dummy",
        "rotation_status":"registered_literal_simc","duration_seconds":elapsed,"incoming_events":next,"tank_casts":casts,
        "raw_damage":raw,"damage_taken":taken,"mitigation_pct":(raw>0.0).then(||100.0*(raw-taken)/raw),
        "self_heal_per_second":(elapsed>0.0).then(||self_healing/elapsed),
        "absorb_per_second":(elapsed>0.0).then(||absorbed/elapsed),"time_to_death":death,
        "active_mitigation_auras":mitigation_auras,"active_mitigation_uptime":(elapsed>0.0).then(||active_seconds/elapsed),
        "spike_window_seconds":SPIKE_WINDOW_SECONDS,"largest_spike":spike}),
    )
}
