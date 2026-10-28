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
const SHIELD_OF_THE_RIGHTEOUS: u32 = 132403;

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
    let bloodlust_ms = if external_healing {
        let (profile, _) = super::dps_parity::read_reference(spec_id)?;
        let name = profile
            .options
            .get("name")
            .and_then(|value| value.as_str())
            .ok_or("SIMC_PROFILE_NAME_ABSENT")?;
        simc_tank_timeline::read_bloodlust_time(path, name)?
    } else {
        None
    };
    let raid_buffs = if external_healing {
        simc_tank_timeline::read_raid_buffs(path)?
    } else {
        Vec::new()
    };
    let dummy_debuffs = if external_healing {
        simc_tank_timeline::read_dummy_debuffs(path, "Tank_Dummy_Dungeon")?
    } else {
        Vec::new()
    };
    let _rng = crate::random::seed_scope(seed);
    let mut run = super::dps_parity::prepare_registered_player(spec_id, 1)?;
    apply_simc_raid_buffs(&mut run, &raid_buffs)?;
    apply_simc_dummy_debuffs(&mut run, &dummy_debuffs)?;
    let auras = super::dps_parity::tank_mitigation_auras(&run)?;
    replay_registered_run_with_healing(
        run,
        &hits,
        duration_seconds,
        auras,
        external_healing,
        bloodlust_ms,
        &raid_buffs,
        &dummy_debuffs,
    )
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
    )
}

pub(super) fn apply_simc_raid_buffs(
    run: &mut super::dps_parity::Run,
    buffs: &[simc_tank_timeline::RaidBuff],
) -> Result<(), String> {
    let tank = run.actor;
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
        crate::auras::apply_aura(
            world,
            &crate::auras::AuraApplication {
                caster: run.actor,
                target,
                spell_id: debuff.spell_id,
                effect_mask: u32::MAX,
                source: crate::auras::AuraSource::Cast,
            },
        );
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

fn replay_registered_run_with_healing(
    mut run: super::dps_parity::Run,
    hits: &[crate::simc_tank_timeline::Hit],
    duration_seconds: u32,
    mitigation_auras: &[u32],
    external_healing: bool,
    bloodlust_ms: Option<u64>,
    raid_buffs: &[simc_tank_timeline::RaidBuff],
    dummy_debuffs: &[simc_tank_timeline::RaidBuff],
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
    let mut event_cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    let (mut raw, mut taken, mut self_healing, mut active_seconds, mut spike) =
        (0.0, 0.0, 0.0, 0.0, 0.0_f64);
    let mut self_periodic_taken = 0.0;
    let (mut recent, mut recent_sum) = (VecDeque::<(f64, f64)>::new(), 0.0);
    let (mut next, mut elapsed, mut death) = (0, 0.0, None);
    let mut casts = BTreeMap::<u32, usize>::new();
    let mut healing_by_spell = BTreeMap::<u32, f64>::new();
    let mut raw_healing_by_spell = BTreeMap::<u32, f64>::new();
    let mut periodic_ticks_by_spell = BTreeMap::<u32, usize>::new();
    let mut energize_by_spell = BTreeMap::<u32, (i64, i64)>::new();
    let mut damage_taken_by_kind = BTreeMap::<&str, f64>::new();
    let mut outcomes = BTreeMap::<String, usize>::new();
    let mut incoming_before_absorbs = BTreeMap::<&str, f64>::new();
    let mut incoming_by_outcome = BTreeMap::<String, f64>::new();
    let mut defense_totals = [0.0_f64; 6];
    let mut defense_samples = 0_u64;
    let mut aura_seconds = BTreeMap::<u32, f64>::new();
    let mut debuff_seconds: BTreeMap<u32, f64> =
        target_debuffs.iter().map(|&id| (id, 0.0)).collect();
    let mut external_healing_effective = 0.0;
    let mut bloodlust_applied = false;
    for tick in 1..=duration_seconds * 20 {
        elapsed = f64::from(tick) * TICK_SECONDS;
        if !bloodlust_applied && bloodlust_ms.is_some_and(|time| time <= u64::from(tick - 1) * 50) {
            crate::auras::apply_aura(
                run.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: tank,
                    target: tank,
                    spell_id: 2825,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
            bloodlust_applied = true;
        }
        if external_healing && tick % 100 == 0 && tick < duration_seconds * 20 {
            let mut health = run
                .app
                .world_mut()
                .get_mut::<Health>(tank)
                .ok_or("TANK_HEALTH_ABSENT")?;
            if health.current > 0.0 {
                let amount = 75_000.0_f32.min((health.max - health.current).max(0.0));
                health.current += amount;
                external_healing_effective += f64::from(amount);
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
            let amount = hit.amount as f32;
            if !amount.is_finite() {
                return Err("SIMC_DUMMY_AMOUNT_OUT_OF_RANGE".into());
            }
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
                    let source_level = crate::level_scaling::UnitScaling::of(world, source)
                        .level_against(&crate::level_scaling::UnitScaling::of(world, tank));
                    let constant = world
                        .resource::<crate::level_scaling::ArmorConstants>()
                        .at(source_level)
                        .ok_or("DUMMY_ARMOR_CONSTANT_ABSENT")?;
                    let damage_mult = run
                        .app
                        .world()
                        .get::<crate::auras::AuraStatMods>(tank)
                        .map_or(1.0, |mods| mods.damage_taken(1));
                    for (total, value) in defense_totals.iter_mut().zip([
                        stats.defense.parry,
                        stats.defense.block,
                        stats.armor,
                        stats.defense.blocked_pct(constant),
                        stats.versatility_pct / 2.0,
                        damage_mult,
                    ]) {
                        *total += f64::from(value);
                    }
                    defense_samples += 1;
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
                            outcome == crate::melee::MeleeOutcome::Crit,
                            outcome == crate::melee::MeleeOutcome::Block,
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
            .is_some_and(|health| health.current > 0.0)
        {
            super::dps_parity::drive_literal_tick(&mut run)
                .map_err(|error| format!("native APL tick {tick}: {error}"))?;
        }
        let boundary = started_at + std::time::Duration::from_millis(u64::from(tick) * 50);
        super::dps_parity::advance_fixture_step(&mut run, boundary)?;
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
    Ok(
        json!({"spec":spec,"level":level,"reference_tier":"MID2","timeline":"simc_tank_dummy",
        "external_healing":external_healing,"external_healing_effective":external_healing_effective,
        "bloodlust_start_seconds":bloodlust_ms.filter(|_| bloodlust_applied).map(|ms| ms as f64 / 1000.0),
        "raid_buffs":raid_buffs.iter().map(|buff| json!({"name":buff.name,"spell_id":buff.spell_id})).collect::<Vec<_>>(),
        "dummy_debuffs":dummy_debuffs.iter().map(|buff| json!({"name":buff.name,"spell_id":buff.spell_id})).collect::<Vec<_>>(),
        "self_healing_by_spell":healing_by_spell,"raw_self_healing_by_spell":raw_healing_by_spell,
        "periodic_damage_ticks_by_spell":periodic_ticks_by_spell,
        "energize_by_spell":energize_by_spell.iter().map(|(spell, (attempted, effective))| (spell, json!({"attempted": attempted, "effective": effective}))).collect::<BTreeMap<_, _>>(),
        "absorbs_by_spell":absorbs_by_spell,"aura_uptime":aura_uptime,
        "resource_counters":counters,"target_debuff_uptime":target_debuff_uptime,
        "rotation_status":"registered_literal_simc","duration_seconds":elapsed,"incoming_events":next,"tank_casts":casts,
        "raw_damage":raw,"damage_taken":taken,"damage_taken_by_kind":damage_taken_by_kind,"incoming_before_absorbs":incoming_before_absorbs,"incoming_outcomes":outcomes,"incoming_by_outcome":incoming_by_outcome,
        "defense_mean":defense_mean,
        "self_periodic_damage_taken":self_periodic_taken,
        "mitigation_pct":(raw>0.0).then(||100.0*(raw-taken)/raw),
        "self_heal_per_second":(elapsed>0.0).then(||self_healing/elapsed),
        "absorb_per_second":(elapsed>0.0).then(||absorbed/elapsed),"time_to_death":death,
        "active_mitigation_auras":mitigation_auras,"active_mitigation_uptime":(elapsed>0.0).then(||active_seconds/elapsed),
        "spike_window_seconds":SPIKE_WINDOW_SECONDS,"largest_spike":spike}),
    )
}
