//! SimC dummy pressure replay against a native MID2 healer and party.
use super::healer_intake::{Capture, Hit, HitKind};
use super::healer_simc_metrics::Recorder;
use bevy::prelude::{Entity, World};
use serde_json::{Value, json};
use shared::components::Health;
use std::path::Path;

#[cfg(test)]
#[path = "healer_simc_replay_tests.rs"]
mod tests;

pub(super) trait Policy {
    fn drive(&mut self, world: &mut World, healer: Entity, party: &[Entity]) -> Result<(), String>;
}

type PreparePolicy = fn(&mut super::dps_parity::Run) -> Result<Box<dyn Policy>, String>;
type Registration = (u32, PreparePolicy);

// Spec owners add their own policy here. No test fixture is a production policy.
const POLICIES: &[Registration] = &[
    (264, super::restoration_shaman_replay::prepare),
    (270, super::mistweaver_replay::prepare),
];

pub(super) fn replay_simc(
    spec: u32,
    trace: &Path,
    duration: u32,
    seed: u64,
) -> Result<Value, String> {
    replay_with_registry(spec, trace, duration, seed, POLICIES)
}

fn replay_with_registry(
    spec: u32,
    trace: &Path,
    duration: u32,
    seed: u64,
    registry: &[Registration],
) -> Result<Value, String> {
    let prepare = registry
        .iter()
        .find(|(id, _)| *id == spec)
        .map(|(_, prepare)| *prepare)
        .ok_or_else(|| format!("NATIVE_HEALER_POLICY_ABSENT:{spec}"))?;
    let canonical = Path::new("/home/osso/Projects/world-of-osso/game-server/data/world.db")
        .canonicalize()
        .map_err(|error| format!("canonical world database: {error}"))?;
    let selected = std::env::var_os("DPS_PARITY_WORLD_DB").ok_or("DPS_PARITY_WORLD_DB absent")?;
    if Path::new(&selected)
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
    if duration == 0 {
        return Err("SIMC_REPLAY_DURATION_ZERO".into());
    }
    let hits = crate::simc_tank_timeline::read_trace(trace, "Tank_Dummy_Dungeon")?;
    let (profile, traits) = super::dps_parity::read_reference(spec)?;
    super::dps_parity::require_mid2_profile(&profile)?;
    let race = super::dps_parity::read_reference_race(&profile.race)?;
    let _rng = crate::random::seed_scope(seed);
    let mut run = super::dps_parity::spawn_reference(
        profile,
        &traits,
        1,
        crate::followers::simulator::ParityActor::Player { race },
    )?;
    for _ in 0..4 {
        run.app.update();
    }
    let healer = run.actor;
    let party =
        crate::followers::simulator::spawn_party_for_healer_player(run.app.world_mut(), healer)?;
    let members: Vec<_> = party.members.iter().copied().chain([healer]).collect();
    let source = run.targets[0];
    super::healer_replay::freeze_sources(run.app.world_mut(), &[(0, source)].into());
    run.app.insert_resource(Capture {
        members: members.clone(),
        sources: [(source, (0, 0))].into(),
        ..Default::default()
    });
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), party.tank, source);
    let mut policy = prepare(&mut run)?;
    let mut recorder = Recorder::start(run.app.world(), healer, members.clone());
    let mut next = 0;
    for tick in 1..=duration
        .checked_mul(20)
        .ok_or("SIMC_REPLAY_DURATION_OUT_OF_RANGE")?
    {
        while let Some(hit) = hits
            .get(next)
            .filter(|hit| hit.time_ms <= u64::from(tick) * 50)
        {
            if run
                .app
                .world()
                .get::<Health>(party.tank)
                .is_some_and(|health| health.current > 0.0)
            {
                deliver_hit(run.app.world_mut(), source, party.tank, hit, tick)?;
            }
            next += 1;
        }
        if run
            .app
            .world()
            .get::<Health>(healer)
            .is_some_and(|health| health.current > 0.0)
        {
            policy.drive(run.app.world_mut(), healer, &members)?;
        }
        run.app.update();
        recorder.observe_frame(run.app.world(), tick);
    }
    Ok(
        json!({"spec":spec,"level":90,"reference_tier":"MID2","timeline":"simc_tank_dummy",
        "duration_seconds":duration,"incoming_events":next,"metrics":recorder.report(duration)}),
    )
}

fn deliver_hit(
    world: &mut World,
    source: Entity,
    tank: Entity,
    hit: &crate::simc_tank_timeline::Hit,
    tick: u32,
) -> Result<(), String> {
    use crate::simc_tank_timeline::Kind;
    let amount = hit.amount as f32;
    if !amount.is_finite() {
        return Err("SIMC_DUMMY_AMOUNT_OUT_OF_RANGE".into());
    }
    let scale = crate::level_scaling::damage_to_hp_scale(world, source, tank);
    world.resource_mut::<Capture>().active_hit_scale = Some((tank, scale));
    match hit.kind {
        Kind::Bleed => {
            crate::spell_cast::effects::replay_simc_dummy_bleed(world, source, tank, amount)
        }
        Kind::Swing | Kind::Strike => {
            let (outcome, level_difference) = crate::combat::roll_simc_dummy_outcome(
                world,
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
            if hit.kind == Kind::Strike {
                let damage = super::tank_replay::source_amount(&recorded)?;
                crate::spell_cast::effects::replay_simc_dummy_strike(
                    world,
                    source,
                    tank,
                    damage as f32,
                    outcome == crate::melee::MeleeOutcome::Crit,
                    outcome == crate::melee::MeleeOutcome::Block,
                );
            } else {
                crate::combat::replay_healer_hit(world, source, tank, &recorded)?;
            }
        }
    }
    world.resource_mut::<Capture>().active_hit_scale = None;
    Ok(())
}
