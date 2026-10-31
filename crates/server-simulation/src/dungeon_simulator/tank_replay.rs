//! Isolated tank decisions under a frozen native incoming-damage recording.
#[path = "protection_rotation.rs"]
pub(super) mod protection_rotation;
use super::healer_intake::{Capture, Hit, HitKind, Intake};
use super::healer_replay::{PendingHits, deliver_hits, freeze_sources};
use crate::simc_tank_timeline;
use bevy::prelude::*;
use serde_json::{Value, json};
use shared::components::{Health, Position};
use shared::protocol::CombatLogKind;
use std::collections::{BTreeMap, BTreeSet, VecDeque};

const TICK_SECONDS: f64 = 0.05;
const SPIKE_WINDOW_SECONDS: f64 = 5.0;

#[derive(Clone, Copy)]
enum DecisionKind {
    Foreground,
    OffGcd,
}

struct DecisionSchedule {
    at: std::time::Duration,
    kind: DecisionKind,
    gaussian_pair: Option<f64>,
}

impl DecisionSchedule {
    fn new(at: std::time::Duration) -> Self {
        Self {
            at,
            kind: DecisionKind::Foreground,
            gaussian_pair: None,
        }
    }

    fn poll_delay(&mut self) -> std::time::Duration {
        use crate::random::Rng;
        // SimC76f8118 rng.hpp:295–326 uses the polar Box–Muller pair.
        let normal = if let Some(value) = self.gaussian_pair.take() {
            value
        } else {
            let mut rng = crate::random::thread_rng();
            loop {
                let x = 2.0 * rng.r#gen::<f64>() - 1.0;
                let y = 2.0 * rng.r#gen::<f64>() - 1.0;
                let w = x * x + y * y;
                if w > 0.0 && w < 1.0 {
                    let factor = (-2.0 * w.ln() / w).sqrt();
                    self.gaussian_pair = Some(y * factor);
                    break x * factor;
                }
            }
        };
        std::time::Duration::from_secs_f64((0.1 + 0.01 * normal).max(0.000_000_001))
    }

    fn decide(&mut self, run: &mut super::dps_parity::Run) -> Result<(), String> {
        let off_gcd_acted = match self.kind {
            DecisionKind::Foreground => {
                super::dps_parity::drive_literal_tick(run)?;
                false
            }
            DecisionKind::OffGcd => super::dps_parity::drive_literal_off_gcd(run)?,
        };
        let world = run.app.world();
        let now = world.resource::<Time>().elapsed();
        let clock = crate::spell_cast::history_now(world);
        let gcd = world
            .get::<crate::spell_cast::SpellHistory>(run.actor)
            .and_then(|history| history.active_gcd_ready_at(clock));
        let cast = world
            .get::<shared::casting::CastState>(run.actor)
            .map(|cast| (cast.duration - cast.elapsed).max(0.0));
        if gcd.is_some()
            && cast.is_none()
            && (matches!(self.kind, DecisionKind::Foreground) || off_gcd_acted)
        {
            // SimC76f8118 action.cpp303–306 schedules the first off-GCD check immediately.
            self.at = now;
            self.kind = DecisionKind::OffGcd;
            return Ok(());
        }
        let poll = now + self.poll_delay();
        if gcd.is_some() || cast.is_some() {
            let gcd_at = next_gcd_decision_boundary(run, std::time::Duration::MAX);
            let cast_at = cast.map_or(now, |remaining| {
                now + std::time::Duration::from_secs_f32(remaining)
                    + std::time::Duration::from_micros(1)
            });
            let ready = if gcd.is_some() {
                gcd_at.max(cast_at)
            } else {
                cast_at
            };
            if ready <= poll {
                self.at = ready;
                self.kind = DecisionKind::Foreground;
            } else {
                self.at = poll;
                self.kind = DecisionKind::OffGcd;
            }
        } else {
            // SimC76f8118 player.cpp343–351,7130–7133: no ready action retries at 100±10 ms.
            self.at = poll;
            self.kind = DecisionKind::Foreground;
        }
        Ok(())
    }

    fn run_due(&mut self, run: &mut super::dps_parity::Run) -> Result<(), String> {
        for _ in 0..128 {
            if self.at > run.app.world().resource::<Time>().elapsed() {
                return Ok(());
            }
            self.decide(run)?;
        }
        Err("tank replay exceeded 128 simultaneous decision events".into())
    }
}

pub(super) fn next_gcd_decision_boundary(
    run: &super::dps_parity::Run,
    boundary: std::time::Duration,
) -> std::time::Duration {
    let world = run.app.world();
    let now = crate::spell_cast::history_now(world);
    let Some(ready) = world
        .get::<crate::spell_cast::SpellHistory>(run.actor)
        .and_then(|history| history.active_gcd_ready_at(now))
    else {
        return boundary;
    };
    let delay =
        std::time::Duration::from_secs_f64(ready - now) + std::time::Duration::from_nanos(1);
    boundary.min(world.resource::<Time>().elapsed() + delay)
}

#[cfg(test)]
mod decision_schedule_tests {
    use super::*;

    #[test]
    #[ignore = "uses the selected MID2 Protection profile"]
    fn scheduled_decision_waits_before_casting() {
        use crate::spell_cast::test_harness::ObservedCombatEvents;
        use shared::protocol::CombatLogKind;
        let _seed = crate::random::seed_scope(70448);
        let mut run = super::super::dps_parity::prepare_registered_player(66, 1).unwrap();
        super::super::dps_parity::engage(&mut run);
        for _ in 0..40 {
            run.app.update();
        }
        let actor = run.actor;
        let now = run.app.world().resource::<Time>().elapsed();
        let mut schedule = DecisionSchedule::new(now + std::time::Duration::from_millis(100));
        let count = |run: &super::super::dps_parity::Run| {
            run.app
                .world()
                .resource::<ObservedCombatEvents>()
                .0
                .iter()
                .filter(|event| {
                    event.source == Some(actor.to_bits())
                        && event.kind == CombatLogKind::CastSuccess
                })
                .count()
        };
        let before = count(&run);
        schedule.run_due(&mut run).unwrap();
        assert_eq!(
            count(&run),
            before,
            "no foreground cast before its decision event"
        );
        run.app
            .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(
                std::time::Duration::from_millis(100),
            ));
        run.app.update();
        let before_due = count(&run);
        schedule.run_due(&mut run).unwrap();
        assert!(
            count(&run) > before_due,
            "the due foreground decision casts an ability"
        );
    }

    #[test]
    fn poll_intervals_follow_seeded_gaussian() {
        let sample = || {
            let _seed = crate::random::seed_scope(70448);
            let mut schedule = DecisionSchedule::new(std::time::Duration::ZERO);
            (0..10_000)
                .map(|_| schedule.poll_delay().as_secs_f64() * 1000.0)
                .collect::<Vec<_>>()
        };
        let values = sample();
        assert_eq!(values, sample());
        let mean = values.iter().sum::<f64>() / values.len() as f64;
        let deviation =
            (values.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / values.len() as f64).sqrt();
        assert!((99.5..100.5).contains(&mean), "mean poll interval {mean}");
        assert!(
            (9.5..10.5).contains(&deviation),
            "poll interval deviation {deviation}"
        );
    }
}

pub(super) fn sampled_blocked_pct(
    world: &World,
    source: Entity,
    tank: Entity,
) -> Result<f32, String> {
    let attacker = crate::level_scaling::UnitScaling::of(world, source);
    let defender = crate::level_scaling::UnitScaling::of(world, tank);
    let constant = attacker
        .armor_constant(
            &defender,
            world.resource::<crate::level_scaling::ArmorConstants>(),
        )
        .ok_or("DUMMY_ARMOR_CONSTANT_ABSENT")?;
    let stats = world
        .get::<crate::stats::CharacterStats>(tank)
        .ok_or("TANK_STATS_ABSENT")?;
    Ok(stats
        .defense
        .blocked_pct_with_auras(constant, world.get::<crate::auras::ActiveAuras>(tank)))
}
const SHIELD_OF_THE_RIGHTEOUS: u32 = 132403;
// SimC Tank_Dummy_Dungeon: native dungeon difficulty 23, MID2 season 120.
const TANK_DUNGEON_DIFFICULTY: u32 = 23;
const TANK_DUNGEON_TUNING: u32 = 3025;
const TANK_DUNGEON_SEASON: u32 = 120;

pub(super) fn bulwark_observations(world: &World, tank: Entity) -> (usize, usize, u32) {
    let events = &world
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0;
    let mut starts = 0;
    let mut refreshes = 0;
    for event in events.iter().filter(|event| {
        event.source == Some(tank.to_bits())
            && event.target == Some(tank.to_bits())
            && event.spell_id == Some(432496)
    }) {
        match event.kind {
            CombatLogKind::AuraApplied => starts += 1,
            CombatLogKind::AuraRefreshed => refreshes += 1,
            _ => {}
        }
    }
    let procs = world
        .get::<server::test_support::DivineInspirationProcs>(tank)
        .map_or(0, |count| count.0);
    (starts, refreshes, procs)
}

pub(super) fn resource_ledger_entry(
    event: &shared::protocol::CombatLogEvent,
    tank: Entity,
    time: f64,
) -> Option<Value> {
    if event.target != Some(tank.to_bits()) || event.kind != CombatLogKind::Energize {
        return None;
    }
    let spell = event.spell_id?;
    Some(
        json!({"time_seconds":time,"spell_id":spell,"gained":event.amount-event.overflow,"wasted":event.overflow}),
    )
}

pub(super) fn apply_replay_bloodlust(world: &mut World, tank: Entity) -> Result<(), String> {
    crate::auras::apply_aura(
        world,
        &crate::auras::AuraApplication {
            caster: tank,
            target: tank,
            spell_id: 2825,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    server::test_support::recompute_fixture_stats(world)
}

fn self_periodic_damage(event: &shared::protocol::CombatLogEvent, tank: Entity) -> bool {
    event.kind == CombatLogKind::Damage && event.periodic && event.source == Some(tank.to_bits())
}

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
        "CritBlock" => MeleeOutcome::CritBlock,
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
    let mut self_periodic_taken = 0.0;
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
            let self_periodic = self_periodic_damage(event, tank);
            if event.kind == CombatLogKind::Damage
                && (self_periodic
                    || event
                        .source
                        .is_some_and(|source| source_entities.contains(&source)))
            {
                let amount = f64::from(event.amount.max(0));
                taken += amount;
                if self_periodic {
                    self_periodic_taken += amount;
                }
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
        "raw_damage":raw,"damage_taken":taken,"self_periodic_damage_taken":self_periodic_taken,
        "mitigation_pct":(raw > 0.0).then(|| 100.0 * (raw - taken) / raw),
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
    replay_simc_with_healing(spec_id, path, duration_seconds, seed, false)
}

pub(super) fn replay_simc_with_healing(
    spec_id: u32,
    path: &std::path::Path,
    duration_seconds: u32,
    seed: u64,
    external_healing: bool,
) -> Result<Value, String> {
    replay_simc_with_pressure(spec_id, path, duration_seconds, seed, external_healing, 1.0)
}

pub(super) fn replay_simc_with_pressure(
    spec_id: u32,
    path: &std::path::Path,
    duration_seconds: u32,
    seed: u64,
    external_healing: bool,
    pressure_scale: f64,
) -> Result<Value, String> {
    replay_simc_with_targets(
        spec_id,
        path,
        1,
        duration_seconds,
        seed,
        external_healing,
        pressure_scale,
    )
}

pub(super) fn replay_simc_with_targets(
    spec_id: u32,
    path: &std::path::Path,
    target_count: usize,
    duration_seconds: u32,
    seed: u64,
    external_healing: bool,
    pressure_scale: f64,
) -> Result<Value, String> {
    use crate::simc_tank_timeline;
    if !matches!(target_count, 1 | 3) {
        return Err(format!("SIMC_TANK_TARGET_COUNT_UNSUPPORTED:{target_count}"));
    }
    if !pressure_scale.is_finite() || pressure_scale <= 0.0 {
        return Err("SIMC_PRESSURE_SCALE_MUST_BE_POSITIVE_AND_FINITE".into());
    }
    let selected = std::env::var_os("DPS_PARITY_WORLD_DB").ok_or("DPS_PARITY_WORLD_DB absent")?;
    let selected = std::path::Path::new(&selected)
        .canonicalize()
        .map_err(|error| format!("selected world database: {error}"))?;
    let startup = crate::test_data::world_db_path()
        .canonicalize()
        .map_err(|error| format!("startup world database: {error}"))?;
    if selected != startup {
        return Err("SIMC_REPLAY_WORLD_DB_MISMATCH".into());
    }
    if duration_seconds != 300 {
        return Err("NATIVE_LITERAL_ENCOUNTER_REQUIRES_300_SECONDS".into());
    }
    let scenario = simc_tank_timeline::read_scenario(path)?;
    let racial = simc_tank_timeline::read_touch_of_elune(path, &scenario.player)?;
    let mut hits = simc_tank_timeline::read_trace(path, &scenario.dummy)?;
    scale_pressure(&mut hits, pressure_scale);
    let (bloodlust_ms, raid_buffs) = if external_healing {
        (
            simc_tank_timeline::read_bloodlust_time(path, &scenario.player)?,
            simc_tank_timeline::read_raid_buffs(path, &scenario.player)?,
        )
    } else {
        (None, Vec::new())
    };
    let dummy_debuffs = if external_healing {
        simc_tank_timeline::read_dummy_debuffs(path, &scenario.dummy)?
    } else {
        Vec::new()
    };
    let dummy_bleeding = if external_healing {
        simc_tank_timeline::read_dummy_bleeding(path, &scenario.dummy)?
    } else {
        false
    };
    let _rng = crate::random::seed_scope(seed);
    let mut run =
        super::dps_parity::prepare_registered_player_with(spec_id, target_count, |run| {
            apply_simc_touch_of_elune(run, racial)?;
            apply_simc_raid_buffs(run, &raid_buffs)?;
            if racial.is_some() && raid_buffs.is_empty() {
                run.app.update();
            }
            apply_simc_dummy_debuffs(run, &dummy_debuffs)
        })?;
    apply_simc_starting_holy_power(&mut run);
    if dummy_bleeding {
        let target = run.targets[0];
        run.app
            .world_mut()
            .entity_mut(target)
            .insert(crate::simc_apl_native::FixtureBleeding);
    }
    let auras = super::dps_parity::tank_mitigation_auras(&run)?;
    let mut report = replay_registered_run_with_healing(
        run,
        &hits,
        duration_seconds,
        auras,
        external_healing,
        bloodlust_ms,
        &raid_buffs,
        &dummy_debuffs,
        dummy_bleeding,
        &scenario,
    )?;
    report["pressure_scale"] = json!(pressure_scale);
    Ok(report)
}

pub(super) fn scale_pressure(hits: &mut [simc_tank_timeline::Hit], scale: f64) {
    for hit in hits {
        hit.amount *= scale;
    }
}

pub(super) fn apply_simc_starting_holy_power(run: &mut super::dps_parity::Run) {
    apply_simc_starting_holy_power_to(run.app.world_mut(), run.actor);
}

pub(super) fn apply_simc_starting_holy_power_to(world: &mut World, actor: Entity) {
    // SimC76f8118 sc_paladin.cpp:4969–4977: Blessed Hammer supplies one at pull.
    let selected = world
        .get::<crate::class_progression::TraitLoadout>(actor)
        .is_some_and(|traits| {
            traits
                .active_talent_spells()
                .any(|spell| spell.spell_id == 204019)
        });
    if selected {
        let mut pools = world
            .get_mut::<crate::spell_cast::powers::PowerPools>(actor)
            .expect("paladin has power pools");
        let holy_power = pools
            .get_mut(shared::components::PowerType::HolyPower)
            .expect("paladin has holy power");
        holy_power.current = (holy_power.current + 1.0).min(1.0);
    }
}

pub(super) fn replay_registered_run(
    run: super::dps_parity::Run,
    hits: &[crate::simc_tank_timeline::Hit],
    duration_seconds: u32,
    mitigation_auras: &[u32],
) -> Result<Value, String> {
    replay_registered_run_with_healing(
        run,
        hits,
        duration_seconds,
        mitigation_auras,
        false,
        None,
        &[],
        &[],
        false,
        &crate::simc_tank_timeline::Scenario {
            dummy: "Tank_Dummy_Dungeon".into(),
            player: String::new(),
            target_level: None,
            armor_constant: 4531.03,
            heals: Vec::new(),
        },
    )
}

pub(super) fn apply_simc_touch_of_elune(
    run: &mut super::dps_parity::Run,
    racial: Option<u32>,
) -> Result<(), String> {
    let is_night_elf = crate::auras::has_aura(run.app.world(), run.actor, 154748);
    if is_night_elf != racial.is_some() {
        return Err("SIMC_TOUCH_OF_ELUNE_RACE_OR_STATE_MISMATCH".into());
    }
    if let Some(spell_id) = racial {
        let actor = run.actor;
        let world = run.app.world_mut();
        crate::auras::apply_aura(
            world,
            &crate::auras::AuraApplication {
                caster: actor,
                target: actor,
                spell_id,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
        if !crate::auras::has_aura(world, actor, spell_id) {
            return Err(format!("SIMC_TOUCH_OF_ELUNE_NOT_APPLIED:{spell_id}"));
        }
    }
    Ok(())
}

pub(super) fn apply_simc_raid_buffs(
    run: &mut super::dps_parity::Run,
    buffs: &[simc_tank_timeline::RaidBuff],
) -> Result<(), String> {
    let tank = run.actor;
    apply_simc_raid_buffs_to(run, tank, buffs)
}

pub(super) fn apply_simc_raid_buffs_to(
    run: &mut super::dps_parity::Run,
    tank: Entity,
    buffs: &[simc_tank_timeline::RaidBuff],
) -> Result<(), String> {
    for buff in buffs {
        let world = run.app.world_mut();
        if world
            .resource::<crate::spell_info::SpellStore>()
            .get(buff.spell_id)
            .is_none()
        {
            return Err(format!(
                "SIMC_RAID_BUFF_NOT_IN_CATALOG:{}:{}",
                buff.name, buff.spell_id
            ));
        }
        if !crate::auras::has_aura(world, tank, buff.spell_id) {
            crate::auras::apply_aura(
                world,
                &crate::auras::AuraApplication {
                    caster: tank,
                    target: tank,
                    spell_id: buff.spell_id,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        if !crate::auras::has_aura(world, tank, buff.spell_id) {
            return Err(format!(
                "SIMC_RAID_BUFF_NOT_APPLIED:{}:{}",
                buff.name, buff.spell_id
            ));
        }
    }
    if !buffs.is_empty() {
        // Mastery-dependent aura amounts refresh after the first stat recompute.
        run.app.update();
        run.app.update();
    }
    Ok(())
}

pub(super) fn apply_simc_dummy_debuffs(
    run: &mut super::dps_parity::Run,
    debuffs: &[simc_tank_timeline::RaidBuff],
) -> Result<(), String> {
    let target = run.targets[0];
    for debuff in debuffs {
        let world = run.app.world_mut();
        if world
            .resource::<crate::spell_info::SpellStore>()
            .get(debuff.spell_id)
            .is_none()
        {
            return Err(format!(
                "SIMC_DUMMY_DEBUFF_NOT_IN_CATALOG:{}:{}",
                debuff.name, debuff.spell_id
            ));
        }
        let application = crate::auras::AuraApplication {
            caster: run.actor,
            target,
            spell_id: debuff.spell_id,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        };
        if debuff.spell_id == 115804 {
            // The SimC override remains active for the encounter, unlike a normal cast.
            let mut spell = world
                .resource::<crate::spell_info::SpellStore>()
                .get(debuff.spell_id)
                .unwrap()
                .clone();
            let duration = spell
                .misc
                .as_mut()
                .and_then(|misc| misc.duration.as_mut())
                .ok_or("SIMC_MORTAL_WOUNDS_DURATION_ABSENT")?;
            duration.duration_ms = -1;
            duration.max_duration_ms = -1;
            crate::auras::apply_aura_with_spell(world, &application, &spell);
        } else {
            crate::auras::apply_aura(world, &application);
        }
        if !crate::auras::has_aura(world, target, debuff.spell_id) {
            return Err(format!(
                "SIMC_DUMMY_DEBUFF_NOT_APPLIED:{}:{}",
                debuff.name, debuff.spell_id
            ));
        }
    }
    if !debuffs.is_empty() {
        run.app.update();
        run.app.update();
    }
    Ok(())
}

pub(super) fn configure_trace_attacker(
    world: &mut World,
    source: Entity,
    player_level: u8,
    scenario: &simc_tank_timeline::Scenario,
) -> Result<(), String> {
    // SimC76f8118 sc_enemy.cpp1214–1220 sets the dungeon dummy two levels above its tank.
    let attacker_level = scenario.target_level.unwrap_or(player_level + 2);
    let base = world
        .resource::<crate::level_scaling::ArmorConstants>()
        .at(attacker_level)
        .ok_or("SIMC_DUMMY_ARMOR_CONSTANT_ABSENT")?;
    world
        .entity_mut(source)
        .insert(shared::components::UnitLevel(attacker_level))
        .remove::<crate::level_scaling::ArmorTuning>()
        .remove::<shared::level_scaling::LevelScaling>()
        .remove::<crate::level_scaling::ScaledStats>();
    if (base - scenario.armor_constant).abs() > 0.01 {
        let tuning = world
            .resource::<crate::world_data::WorldData>()
            .expected_stats
            .armor_tuning(TANK_DUNGEON_TUNING, TANK_DUNGEON_SEASON)?;
        world.entity_mut(source).insert(tuning);
        let attacker = crate::level_scaling::UnitScaling::of(world, source);
        let target = crate::level_scaling::UnitScaling::of(world, source);
        let actual = attacker
            .armor_constant(
                &target,
                world.resource::<crate::level_scaling::ArmorConstants>(),
            )
            .ok_or("SIMC_DUMMY_ARMOR_CONSTANT_ABSENT")?;
        if (actual - scenario.armor_constant).abs() > 0.01 {
            return Err(format!(
                "SIMC_DUMMY_ARMOR_CONSTANT_MISMATCH: trace={}, native={actual}",
                scenario.armor_constant
            ));
        }
    }
    Ok(())
}

fn replay_registered_run_with_healing(
    mut run: super::dps_parity::Run,
    hits: &[crate::simc_tank_timeline::Hit],
    duration_seconds: u32,
    mitigation_auras: &[u32],
    external_healing: bool,
    bloodlust_ms: Option<u64>,
    raid_buffs: &[simc_tank_timeline::RaidBuff],
    dummy_debuffs: &[simc_tank_timeline::RaidBuff],
    dummy_bleeding: bool,
    scenario: &simc_tank_timeline::Scenario,
) -> Result<Value, String> {
    use crate::simc_tank_timeline::Kind;
    let tank = run.actor;
    let source = run.targets[0];
    let initial_counters = super::dps_parity::tank_resource_counters(&run)?;
    let target_debuffs = super::dps_parity::tank_target_debuff_auras(&run)?;
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
    configure_trace_attacker(run.app.world_mut(), source, level, scenario)?;
    if spec == 73 {
        // SimC76f8118 sc_warrior.cpp:3432 assumes the first Charge enters combat.
        // Start outside native Charge's minimum range and let native movement close it.
        let target = *run
            .app
            .world()
            .get::<Position>(source)
            .ok_or("DUMMY_POSITION_ABSENT")?;
        run.app.world_mut().entity_mut(tank).insert(Position {
            x: target.x - 9.0,
            y: target.y,
            z: target.z,
        });
    }
    let started_at = run.app.world().resource::<Time>().elapsed();
    super::dps_parity::install_fixture_lifetimes(&mut run, started_at);
    super::dps_parity::engage(&mut run);
    freeze_sources(run.app.world_mut(), &[(0, source)].into());
    run.app.insert_resource(Capture {
        members: vec![tank],
        sources: [(source, (0, 0))].into(),
        ..Default::default()
    });
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), tank, source);
    run.app
        .world_mut()
        .entity_mut(tank)
        .insert(crate::spell_cast::ObservedResourcePayments::default());
    let mut event_cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    let mut precombat_casts = BTreeMap::<u32, usize>::new();
    for event in &run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0[..event_cursor]
    {
        if event.source == Some(tank.to_bits())
            && event.kind == CombatLogKind::CastSuccess
            && let Some(spell) = event.spell_id
        {
            *precombat_casts.entry(spell).or_default() += 1;
        }
    }
    let (mut raw, mut taken, mut self_healing, mut active_seconds, mut spike) =
        (0.0, 0.0, 0.0, 0.0, 0.0_f64);
    let mut self_periodic_taken = 0.0;
    let (mut recent, mut recent_sum) = (VecDeque::<(f64, f64)>::new(), 0.0);
    let (mut next, mut elapsed, mut death) = (0, 0.0, None);
    let mut next_heal = 0;
    let mut casts = BTreeMap::<u32, usize>::new();
    let mut healing_by_spell = BTreeMap::<u32, f64>::new();
    let mut raw_healing_by_spell = BTreeMap::<u32, f64>::new();
    let mut healing_events_by_spell = BTreeMap::<u32, usize>::new();
    let mut raw_heal_health_fraction_by_spell = BTreeMap::<u32, f64>::new();
    let mut periodic_ticks_by_spell = BTreeMap::<u32, usize>::new();
    let mut outgoing_damage_by_spell = BTreeMap::<u32, i64>::new();
    let mut outgoing_damage_hits_by_spell = BTreeMap::<u32, u64>::new();
    let mut energize_by_spell = BTreeMap::<u32, (i64, i64)>::new();
    let mut energize_timeline = Vec::new();
    let mut cast_timeline = Vec::new();
    let mut damage_taken_by_kind = BTreeMap::<&str, f64>::new();
    let mut outcomes = BTreeMap::<String, usize>::new();
    let mut outcome_sequence = Vec::<Value>::new();
    let mut incoming_before_absorbs = BTreeMap::<&str, f64>::new();
    let mut incoming_by_outcome = BTreeMap::<String, f64>::new();
    let mut defense_totals = [0.0_f64; 6];
    let mut defense_samples = 0_u64;
    let mut dusk_samples = 0_u64;
    let mut dusk_reduction_sum = 0.0_f64;
    let mut block_sources = BTreeMap::<u32, f64>::new();
    let mut aura_seconds = BTreeMap::<u32, f64>::new();
    let mut debuff_seconds: BTreeMap<u32, f64> =
        target_debuffs.iter().map(|&id| (id, 0.0)).collect();
    let mut external_healing_effective = 0.0;
    let mut engagement_samples = vec![super::dps_parity::capture_engagement(&run)];
    let mut retired = Vec::new();
    let mut bloodlust_applied = false;
    let mut decisions = DecisionSchedule::new(started_at);
    for tick in 1..=duration_seconds * 20 {
        elapsed = f64::from(tick) * TICK_SECONDS;
        if !bloodlust_applied && bloodlust_ms.is_some_and(|time| time <= u64::from(tick - 1) * 50) {
            apply_replay_bloodlust(run.app.world_mut(), tank)?;
            bloodlust_applied = true;
        }
        while external_healing
            && scenario
                .heals
                .get(next_heal)
                .is_some_and(|(time, _)| *time <= u64::from(tick) * 50)
        {
            let (time, raw_heal) = scenario.heals[next_heal];
            next_heal += 1;
            if time == 0 {
                continue;
            }
            let effective = {
                let mut health = run
                    .app
                    .world_mut()
                    .get_mut::<Health>(tank)
                    .ok_or("TANK_HEALTH_ABSENT")?;
                if health.current > 0.0 {
                    let amount = raw_heal.min((health.max - health.current).max(0.0));
                    health.current += amount;
                    Some(amount)
                } else {
                    None
                }
            };
            if let Some(amount) = effective {
                external_healing_effective += f64::from(amount);
                crate::spell_cast::script_registry::heal_received(
                    run.app.world_mut(),
                    tank,
                    0,
                    raw_heal,
                    amount,
                    false,
                );
            }
        }
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
            let _attack = crate::random::attack_scope(next as u64);
            let amount = hit.amount as f32;
            if !amount.is_finite() {
                return Err("SIMC_DUMMY_AMOUNT_OUT_OF_RANGE".into());
            }
            let world = run.app.world();
            let maximum_dusk = world
                .get::<crate::auras::ActiveAuras>(tank)
                .and_then(|auras| auras.iter().find(|aura| aura.spell_id == 1241945))
                .and_then(|aura| aura.effects.iter().find(|effect| effect.index == 0))
                .map_or(0.0, |effect| f64::from(effect.amount) / 100.0);
            dusk_reduction_sum += maximum_dusk;
            dusk_samples += 1;
            let scale = crate::level_scaling::damage_to_hp_scale(run.app.world(), source, tank);
            let before_events = run
                .app
                .world()
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0
                .len();
            let before_absorbed = run
                .app
                .world()
                .resource::<Capture>()
                .consumed_absorbs
                .get(&tank)
                .map_or(0.0, |usage| usage.total);
            run.app
                .world_mut()
                .resource_mut::<Capture>()
                .active_hit_scale = Some((tank, scale));
            let mut outcome_key = None;
            match hit.kind {
                Kind::BleedApplication => {
                    crate::spell_cast::effects::replay_simc_dummy_bleed_application(
                        run.app.world_mut(),
                        source,
                        tank,
                    );
                }
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
                    let table = hit.melee_table.ok_or("SIMC_DUMMY_MELEE_TABLE_ABSENT")?;
                    let (outcome, level_difference) = crate::combat::roll_simc_dummy_outcome(
                        run.app.world(),
                        source,
                        tank,
                        hit.kind == Kind::Strike,
                        table.miss,
                        table.crit,
                    )?;
                    let key = format!(
                        "{}:{outcome:?}",
                        if hit.kind == Kind::Swing {
                            "swing"
                        } else {
                            "strike"
                        }
                    );
                    *outcomes.entry(key.clone()).or_default() += 1;
                    outcome_key = Some(key);
                    let stats = run
                        .app
                        .world()
                        .get::<crate::stats::CharacterStats>(tank)
                        .ok_or("TANK_STATS_ABSENT")?;
                    let world = run.app.world();
                    let blocked_pct = sampled_blocked_pct(world, source, tank)?;
                    let damage_mult = run
                        .app
                        .world()
                        .get::<crate::auras::AuraStatMods>(tank)
                        .map_or(1.0, |mods| mods.damage_taken(1));
                    for (total, value) in defense_totals.iter_mut().zip([
                        stats.defense.parry,
                        stats.defense.block,
                        stats.armor,
                        blocked_pct,
                        stats.versatility_pct / 2.0,
                        damage_mult,
                    ]) {
                        *total += f64::from(value);
                    }
                    defense_samples += 1;
                    outcome_sequence.push(json!({
                        "outcome": format!("{outcome:?}"),
                        "parry_pct": stats.defense.parry,
                        "dodge_pct": stats.defense.dodge,
                        "block_pct": stats.defense.block,
                    }));
                    if let Some(auras) = world.get::<crate::auras::ActiveAuras>(tank) {
                        for aura in auras.iter() {
                            for effect in
                                aura.effects.iter().filter(|effect| effect.aura_type == 51)
                            {
                                *block_sources.entry(aura.spell_id).or_default() +=
                                    f64::from(effect.amount * aura.stack_multiplier());
                            }
                        }
                    }
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
                    let source_damage = source_amount(&recorded)?;
                    raw += source_damage;
                    if hit.kind == Kind::Strike {
                        crate::spell_cast::effects::replay_simc_dummy_strike(
                            run.app.world_mut(),
                            source,
                            tank,
                            source_damage as f32,
                            matches!(
                                outcome,
                                crate::melee::MeleeOutcome::Crit
                                    | crate::melee::MeleeOutcome::CritBlock
                            ),
                            matches!(
                                outcome,
                                crate::melee::MeleeOutcome::Block
                                    | crate::melee::MeleeOutcome::CritBlock
                            ),
                        );
                    } else {
                        crate::combat::replay_healer_hit(
                            run.app.world_mut(),
                            source,
                            tank,
                            &recorded,
                        )?;
                    }
                }
            }
            let world = run.app.world();
            let after_absorbed = world
                .resource::<Capture>()
                .consumed_absorbs
                .get(&tank)
                .map_or(0.0, |usage| usage.total);
            let damage: f64 = world
                .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
                .0[before_events..]
                .iter()
                .filter(|event| {
                    event.source == Some(source.to_bits())
                        && event.target == Some(tank.to_bits())
                        && event.kind == CombatLogKind::Damage
                })
                .map(|event| f64::from(event.amount.max(0)))
                .sum();
            let kind = match hit.kind {
                Kind::Swing => "swing",
                Kind::Strike => "strike",
                Kind::BleedApplication => "bleed_application",
                Kind::Bleed => "bleed",
            };
            let pre_absorb = damage + after_absorbed - before_absorbed;
            *incoming_before_absorbs.entry(kind).or_default() += pre_absorb;
            if let Some(key) = outcome_key {
                *incoming_by_outcome.entry(key).or_default() += pre_absorb;
            }
            run.app
                .world_mut()
                .resource_mut::<Capture>()
                .active_hit_scale = None;
            next += 1;
        }
        if run
            .app
            .world()
            .get::<Health>(tank)
            .is_none_or(|health| health.current <= 0.0)
        {
            death = Some(elapsed);
            break;
        }
        decisions
            .run_due(&mut run)
            .map_err(|error| format!("native APL tick {tick}: {error}"))?;
        let boundary = started_at + std::time::Duration::from_millis(u64::from(tick) * 50);
        while run.app.world().resource::<Time>().elapsed() < boundary {
            let next = super::dps_parity::next_fixture_boundary(&run, boundary.min(decisions.at));
            super::dps_parity::advance_fixture_step(&mut run, next)?;
            super::dps_parity::apply_fixture_add_events(&mut run, &mut retired, started_at)?;
            decisions.run_due(&mut run)?;
        }
        if matches!(tick, 1 | 20 | 5400 | 6000) {
            engagement_samples.push(super::dps_parity::capture_engagement(&run));
        }
        let world = run.app.world();
        if mitigation_auras
            .iter()
            .any(|&aura| crate::auras::has_aura(world, tank, aura))
        {
            active_seconds += TICK_SECONDS;
        }
        if let Some(auras) = world.get::<crate::auras::ActiveAuras>(source) {
            for (&spell, seconds) in &mut debuff_seconds {
                if auras
                    .iter()
                    .any(|aura| aura.spell_id == spell && aura.caster == tank)
                {
                    *seconds += TICK_SECONDS;
                }
            }
        }
        let active = world
            .get::<crate::auras::ActiveAuras>(tank)
            .into_iter()
            .flat_map(|auras| auras.iter())
            .filter(|aura| aura.caster == tank && !aura.passive)
            .map(|aura| aura.spell_id)
            .collect::<BTreeSet<_>>();
        for spell in active {
            *aura_seconds.entry(spell).or_default() += TICK_SECONDS;
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
                cast_timeline.push(json!({"time_seconds":elapsed,"spell_id":spell_id}));
            }
            if event.target == Some(source.to_bits())
                && event.kind == CombatLogKind::Damage
                && event
                    .source
                    .and_then(Entity::try_from_bits)
                    .is_some_and(|actor| {
                        actor == tank
                            || world
                                .get::<crate::spell_cast::summon::SummonedCreature>(actor)
                                .is_some_and(|summon| summon.owner == tank)
                    })
            {
                let spell_id = event.spell_id.unwrap_or(0);
                *outgoing_damage_by_spell.entry(spell_id).or_default() +=
                    i64::from((event.amount - event.overflow.max(0)).max(0));
                *outgoing_damage_hits_by_spell.entry(spell_id).or_default() += 1;
            }
            if event.source == Some(tank.to_bits())
                && event.kind == CombatLogKind::Damage
                && event.periodic
                && let Some(spell_id) = event.spell_id
            {
                *periodic_ticks_by_spell.entry(spell_id).or_default() += 1;
            }
            if event.target == Some(tank.to_bits())
                && event.kind == CombatLogKind::Energize
                && let Some(spell_id) = event.spell_id
            {
                let totals = energize_by_spell.entry(spell_id).or_default();
                totals.0 += i64::from(event.amount);
                totals.1 += i64::from(event.amount - event.overflow);
                energize_timeline.push(
                    resource_ledger_entry(event, tank, elapsed).expect("target energize has spell"),
                );
            }
            if event.target != Some(tank.to_bits()) {
                continue;
            }
            let self_periodic = self_periodic_damage(event, tank);
            if event.kind == CombatLogKind::Damage
                && (event.source == Some(source.to_bits()) || self_periodic)
            {
                let amount = f64::from(event.amount.max(0));
                taken += amount;
                let kind = if self_periodic {
                    "self_periodic"
                } else if event.periodic {
                    "bleed"
                } else if event.spell_id.is_some() {
                    "strike"
                } else {
                    "swing"
                };
                *damage_taken_by_kind.entry(kind).or_default() += amount;
                if self_periodic {
                    self_periodic_taken += amount;
                }
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
                let effective = f64::from((event.amount - event.overflow.max(0)).max(0));
                self_healing += effective;
                if let Some(spell) = event.spell_id {
                    *healing_by_spell.entry(spell).or_default() += effective;
                    *raw_healing_by_spell.entry(spell).or_default() +=
                        f64::from(event.amount.max(0));
                    *healing_events_by_spell.entry(spell).or_default() += 1;
                    let max_health = world.get::<Health>(tank).expect("tank health").max;
                    *raw_heal_health_fraction_by_spell.entry(spell).or_default() +=
                        f64::from(event.amount.max(0)) / f64::from(max_health);
                }
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
    let final_counters = super::dps_parity::tank_resource_counters(&run)?;
    let mut counters = BTreeMap::new();
    for (name, start) in initial_counters {
        let end = final_counters
            .get(name)
            .ok_or_else(|| format!("TANK_COUNTER_DISAPPEARED:{name}"))?;
        let delta = end
            .checked_sub(start)
            .ok_or_else(|| format!("TANK_COUNTER_DECREASED:{name}"))?;
        counters.insert(name, delta);
    }
    if counters.len() != final_counters.len() {
        return Err("TANK_COUNTER_SET_CHANGED".into());
    }
    let target_debuff_uptime: BTreeMap<u32, f64> = debuff_seconds
        .into_iter()
        .map(|(spell, seconds)| (spell, (seconds / elapsed).min(1.0)))
        .collect();
    let capture = run.app.world().resource::<Capture>();
    let absorbs_by_spell: BTreeMap<u32, f64> = capture
        .consumed_absorbs_by_spell
        .iter()
        .filter_map(|(&(caster, spell), &amount)| (caster == tank).then_some((spell, amount)))
        .collect();
    let aura_uptime: BTreeMap<u32, f64> = aura_seconds
        .into_iter()
        .map(|(spell, seconds)| (spell, (seconds / elapsed).min(1.0)))
        .collect();
    let defense_mean: BTreeMap<_, _> = [
        "parry_pct",
        "block_pct",
        "armor",
        "blocked_pct",
        "versatility_dr_pct",
        "aura_damage_multiplier",
    ]
    .into_iter()
    .zip(defense_totals.map(|value| value / defense_samples.max(1) as f64))
    .collect();
    let mut resource_spent_by_spell = BTreeMap::<u32, BTreeMap<String, f64>>::new();
    for &(spell, pool, amount) in &run
        .app
        .world()
        .get::<crate::spell_cast::ObservedResourcePayments>(tank)
        .unwrap()
        .payments
    {
        let resource = match pool {
            crate::spell_cast::CostPool::Power(power) => format!("{power:?}").to_lowercase(),
            crate::spell_cast::CostPool::Health => "health".to_owned(),
        };
        *resource_spent_by_spell
            .entry(spell)
            .or_default()
            .entry(resource)
            .or_default() += f64::from(amount);
    }
    let mut report = json!({"spec":spec,"level":level,"reference_tier":"MID2","timeline":"simc_tank_dummy",
        "external_healing":external_healing,"external_healing_effective":external_healing_effective,
        "bloodlust_start_seconds":bloodlust_ms.filter(|_| bloodlust_applied).map(|ms| ms as f64 / 1000.0),
        "raid_buffs":raid_buffs.iter().map(|buff| json!({"name":buff.name,"spell_id":buff.spell_id})).collect::<Vec<_>>(),
        "dummy_debuffs":dummy_debuffs.iter().map(|buff| json!({"name":buff.name,"spell_id":buff.spell_id})).collect::<Vec<_>>(),
        "dummy_bleeding":dummy_bleeding,
        "self_healing_by_spell":healing_by_spell,"raw_self_healing_by_spell":raw_healing_by_spell,
        "periodic_damage_ticks_by_spell":periodic_ticks_by_spell,
        "energize_by_spell":energize_by_spell.iter().map(|(spell, (attempted, effective))| (spell, json!({"attempted": attempted, "effective": effective}))).collect::<BTreeMap<_, _>>(),
        "absorbs_by_spell":absorbs_by_spell,"aura_uptime":aura_uptime,
        "resource_counters":counters,"target_debuff_uptime":target_debuff_uptime,
        "rotation_status":"registered_literal_simc","duration_seconds":elapsed,"incoming_events":next,"tank_casts":casts,
        "raw_damage":raw,"damage_taken":taken,"damage_taken_by_kind":damage_taken_by_kind,"incoming_before_absorbs":incoming_before_absorbs,"incoming_outcomes":outcomes,"incoming_outcome_sequence":outcome_sequence,"incoming_by_outcome":incoming_by_outcome,
        "defense_mean":defense_mean,
        "dusk_mean_reduction": (dusk_samples > 0).then(|| dusk_reduction_sum / dusk_samples as f64),
        "block_sources_mean":block_sources.iter().map(|(spell, amount)| (spell, amount / defense_samples.max(1) as f64)).collect::<BTreeMap<_,_>>(),
        "self_periodic_damage_taken":self_periodic_taken,
        "mitigation_pct":(raw>0.0).then(||100.0*(raw-taken)/raw),
        "self_heal_per_second":(elapsed>0.0).then(||self_healing/elapsed),
        "absorb_per_second":(elapsed>0.0).then(||absorbed/elapsed),"time_to_death":death,
        "active_mitigation_auras":mitigation_auras,"active_mitigation_uptime":(elapsed>0.0).then(||active_seconds/elapsed),
        "spike_window_seconds":SPIKE_WINDOW_SECONDS,"largest_spike":spike});
    report["self_healing_events_by_spell"] = json!(healing_events_by_spell);
    report["raw_heal_health_fraction_by_spell"] = json!(raw_heal_health_fraction_by_spell);
    let world = run.app.world();
    let attacker = crate::level_scaling::UnitScaling::of(world, source);
    let defender = crate::level_scaling::UnitScaling::of(world, tank);
    let armor_constant = attacker
        .armor_constant(
            &defender,
            world.resource::<crate::level_scaling::ArmorConstants>(),
        )
        .ok_or("SIMC_DUMMY_ARMOR_CONSTANT_ABSENT")?;
    let dungeon = scenario.target_level.is_none();
    report["tank_dummy_scenario"] = json!({"dummy":scenario.dummy,
        "difficulty":dungeon.then_some(TANK_DUNGEON_DIFFICULTY),
        "content_tuning":dungeon.then_some(TANK_DUNGEON_TUNING),
        "season":dungeon.then_some(TANK_DUNGEON_SEASON),
        "attacker_level":attacker.level_against(&defender),"armor_constant":armor_constant});
    report["combat"] = super::dps_parity::combat_report(&run, elapsed);
    report["actor_kind"] = json!("player");
    report["target_waves"] = json!(run.target_waves);
    report["engagement_samples"] = json!(engagement_samples);
    let (bulwark_starts, bulwark_refreshes, divine_inspiration_procs) =
        bulwark_observations(world, tank);
    report["holy_bulwark_applications"] =
        json!({"starts": bulwark_starts, "refreshes": bulwark_refreshes});
    report["divine_inspiration_procs"] = json!(divine_inspiration_procs);
    report["energize_timeline"] = json!(energize_timeline);
    report["cast_timeline"] = json!(cast_timeline);
    report["precombat_casts"] = json!(precombat_casts);
    report["outgoing_damage_by_spell"] = json!(outgoing_damage_by_spell);
    report["outgoing_damage_hits_by_spell"] = json!(outgoing_damage_hits_by_spell);
    report["resource_spent_by_spell"] = json!(resource_spent_by_spell);
    report["melee_rage"] = json!(
        capture
            .melee_rage
            .iter()
            .filter_map(|(&(unit, source), &amount)| (unit == tank).then_some((source, amount)))
            .collect::<BTreeMap<_, _>>()
    );
    Ok(report)
}
