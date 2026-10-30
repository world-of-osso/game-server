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

/// The fixture party tank's combat target: the healer's damage target.
pub(super) fn tank_target(world: &World, party: &[Entity]) -> Option<Entity> {
    let tank = server_follower_apl::party_tank(world, party)?;
    world.get::<crate::networking::CombatTarget>(tank)?.0
}

pub(super) trait Policy {
    fn drive(&mut self, world: &mut World, healer: Entity, party: &[Entity]) -> Result<(), String>;
}

type PreparePolicy = fn(&mut super::dps_parity::Run, Entity) -> Result<Box<dyn Policy>, String>;
type Registration = (u32, PreparePolicy);

// Spec owners add their own policy here. No test fixture is a production policy.
const POLICIES: &[Registration] = &[
    (65, super::holy_paladin_simc::prepare),
    (256, super::discipline_priest_replay::prepare),
    (257, super::holy_priest_replay::prepare),
    (105, super::restoration_druid_replay::prepare),
    (264, super::restoration_shaman_replay::prepare),
    (270, super::mistweaver_replay::prepare),
    (1468, super::preservation_evoker_replay::prepare),
];

/// The replay's follower companions, equipped by `equip_companions`.
pub(crate) fn spawn_reference_party(
    world: &mut World,
    tank: Entity,
    healer: Entity,
) -> Result<crate::followers::simulator::Party, String> {
    let party = crate::followers::simulator::spawn_party_for_reference_healer(world, tank, healer)?;
    super::dps_parity::equip_companions(world, &party)?;
    Ok(party)
}

fn apply_optimal_raid_environment(
    run: &mut super::dps_parity::Run,
    members: &[Entity],
) -> Result<(), String> {
    use crate::simc_tank_timeline::RaidBuff;

    let buffs =
        [("arcane_intellect", 1459), ("mark_of_the_wild", 1126)].map(|(name, spell_id)| RaidBuff {
            name: name.into(),
            spell_id,
        });
    let debuffs =
        [("chaos_brand", 1490), ("mystic_touch", 113746)].map(|(name, spell_id)| RaidBuff {
            name: name.into(),
            spell_id,
        });
    for &member in members {
        super::tank_replay::apply_simc_raid_buffs_to(run, member, &buffs)?;
    }
    super::tank_replay::apply_simc_dummy_debuffs(run, &debuffs)?;
    for &member in members {
        super::tank_replay::apply_replay_bloodlust(run.app.world_mut(), member)?;
    }
    Ok(())
}

pub(super) fn replay_simc(
    spec: u32,
    trace: &Path,
    duration: u32,
    seed: u64,
) -> Result<Value, String> {
    replay_with_registry(spec, trace, duration, seed, POLICIES)
}

pub(super) fn replay_simc_party(
    spec: u32,
    trace: &Path,
    party_trace: &Path,
    duration: u32,
    seed: u64,
) -> Result<Value, String> {
    replay_with_party_registry(spec, trace, Some(party_trace), duration, seed, POLICIES)
}

fn replay_with_registry(
    spec: u32,
    trace: &Path,
    duration: u32,
    seed: u64,
    registry: &[Registration],
) -> Result<Value, String> {
    replay_with_party_registry(spec, trace, None, duration, seed, registry)
}

fn replay_with_party_registry(
    spec: u32,
    trace: &Path,
    party_trace: Option<&Path>,
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
    let party_hits = party_trace
        .map(super::healer_party_trace::read)
        .transpose()?
        .unwrap_or_default();
    let _rng = crate::random::seed_scope(seed);
    let mut run = super::dps_parity::prepare_registered_player(66, 1)?;
    let healer = super::dps_parity::spawn_second_reference_player(&mut run, spec)?;
    run.app
        .world_mut()
        .entity_mut(healer)
        .insert(crate::spell_cast::ObservedResourcePayments::default());
    let tank = run.actor;
    let party = spawn_reference_party(run.app.world_mut(), tank, healer)?;
    let members: Vec<_> = party.members.iter().copied().chain([tank]).collect();
    let party_slots = [
        tank,
        healer,
        party.members[0],
        party.members[1],
        party.members[2],
    ];
    let source = run.targets[0];
    apply_optimal_raid_environment(&mut run, &party_slots)?;
    let started_at = run.app.world().resource::<bevy::prelude::Time>().elapsed();
    super::dps_parity::install_fixture_lifetimes(&mut run, started_at);
    let lifetime = std::time::Duration::from_secs(u64::from(duration));
    let mut target_lifetime = run
        .app
        .world_mut()
        .get_mut::<crate::simc_apl_native::FixtureTarget>(source)
        .ok_or("SIMC_REPLAY_TARGET_LIFETIME_MISSING")?;
    target_lifetime.lifetime = lifetime;
    target_lifetime.expires_at = started_at + lifetime;
    super::dps_parity::engage(&mut run);
    super::healer_replay::freeze_sources(run.app.world_mut(), &[(0, source)].into());
    run.app
        .init_resource::<server_spell_scripts::test_support::FiendAtonementReceipts>();
    run.app.insert_resource(Capture {
        members: members.clone(),
        sources: [(source, (0, 0))].into(),
        ..Default::default()
    });
    crate::combat_refs::set_in_combat_with(run.app.world_mut(), party.tank, source);
    let mut policy = prepare(&mut run, healer)?;
    let mitigation_auras = super::dps_parity::tank_mitigation_auras(&run)?;
    let mut recorder = Recorder::start(run.app.world(), healer, tank, members.clone());
    let mut tank_event_cursor = run
        .app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    let mut tank_casts = std::collections::BTreeMap::<u32, u32>::new();
    let mut healer_casts = std::collections::BTreeMap::<u32, u32>::new();
    let mut mitigation_ticks = 0_u32;
    let mut next = 0;
    let mut next_party = 0;
    for tick in 1..=duration
        .checked_mul(20)
        .ok_or("SIMC_REPLAY_DURATION_OUT_OF_RANGE")?
    {
        let now = run.app.world().resource::<bevy::prelude::Time>().elapsed();
        super::dps_parity::apply_fixed_time_target_health(&mut run, now)?;
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
                recorder.sample_minima(run.app.world());
            }
            next += 1;
        }
        while let Some(hit) = party_hits
            .get(next_party)
            .filter(|hit| hit.time_ms <= u64::from(tick) * 50)
        {
            let target = party_slots[hit.slot];
            if run
                .app
                .world()
                .get::<Health>(target)
                .is_some_and(|health| health.current > 0.0)
            {
                deliver_party_hit(run.app.world_mut(), source, target, hit.amount);
                recorder.sample_minima(run.app.world());
            }
            next_party += 1;
        }
        if run
            .app
            .world()
            .get::<Health>(tank)
            .is_some_and(|health| health.current > 0.0)
        {
            super::dps_parity::drive_literal_tick(&mut run)?;
        }
        if run
            .app
            .world()
            .get::<Health>(healer)
            .is_some_and(|health| health.current > 0.0)
        {
            policy.drive(run.app.world_mut(), healer, &members)?;
        }
        recorder.observe_idle_gcd(run.app.world());
        run.app.update();
        let world = run.app.world();
        recorder.observe_frame(world, tick);
        if mitigation_auras
            .iter()
            .any(|&aura| crate::auras::has_aura(world, tank, aura))
        {
            mitigation_ticks += 1;
        }
        let events = &world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for event in &events[tank_event_cursor..] {
            if event.kind == shared::protocol::CombatLogKind::CastSuccess {
                if let Some(spell) = event.spell_id {
                    if event.source == Some(tank.to_bits()) {
                        *tank_casts.entry(spell).or_default() += 1;
                    } else if event.source == Some(healer.to_bits()) {
                        *healer_casts.entry(spell).or_default() += 1;
                    }
                }
            }
        }
        tank_event_cursor = events.len();
    }
    Ok(
        json!({"spec":spec,"level":90,"reference_tier":"MID2","timeline":"simc_tank_dummy",
        "duration_seconds":duration,"incoming_events":next,"party_incoming_events":next_party,"tank_spec":66,
        "tank_casts":tank_casts,"healer_casts":healer_casts,
        "tank_active_mitigation_seconds":f64::from(mitigation_ticks) / 20.0,
        "metrics":recorder.report(run.app.world(), duration)}),
    )
}

fn deliver_party_hit(world: &mut World, source: Entity, target: Entity, amount: f32) {
    // SimC76f8118 raid_event.cpp:1814-1866: non-critical Shadow spell source roll.
    let spell = crate::spell_info::SpellInfo {
        misc: Some(crate::spell_info::SpellMisc {
            school_mask: 32,
            ..Default::default()
        }),
        ..Default::default()
    };
    let ctx = crate::spell_cast::effects::EffectCtx {
        caster: source,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    crate::spell_cast::effects::deal_damage(world, &ctx, target, amount, false, false);
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
        Kind::BleedApplication => {
            crate::spell_cast::effects::replay_simc_dummy_bleed_application(world, source, tank)
        }
        Kind::Bleed => {
            crate::spell_cast::effects::replay_simc_dummy_bleed(world, source, tank, amount)
        }
        Kind::Swing | Kind::Strike => {
            let table = hit.melee_table.ok_or("SIMC_DUMMY_MELEE_TABLE_ABSENT")?;
            let (outcome, level_difference) = crate::combat::roll_simc_dummy_outcome(
                world,
                source,
                tank,
                hit.kind == Kind::Strike,
                table.miss,
                table.crit,
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
