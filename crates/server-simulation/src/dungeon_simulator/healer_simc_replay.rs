//! SimC dummy pressure replay against a class-built healer and its party.
use super::healer_intake::{Capture, Hit, HitKind};
use super::healer_simc_metrics::Recorder;
use bevy::prelude::{App, Entity, World};
use serde_json::{Value, json};
use shared::components::Health;
use std::path::{Path, PathBuf};

#[cfg(test)]
#[path = "healer_simc_replay_tests.rs"]
mod tests;

const CANONICAL_WORLD_DB: &str = "/home/osso/Projects/world-of-osso/game-server/data/world.db";
const TICKS_PER_SECOND: u32 = 20;
/// Bound on the party's APL precombat before the recorded clock starts.
const MAX_PRECOMBAT_TICKS: u32 = 800;

#[cfg(test)]
pub(super) fn report_death_audit(app: &mut App, actors: &[Entity]) {
    let Some(mut audit) = app
        .world_mut()
        .get_resource_mut::<crate::auras::DeathAuraAudit>()
    else {
        return;
    };
    let removals = std::mem::take(&mut audit.0);
    for (unit, spells) in removals {
        if actors.contains(&unit) {
            let health = app.world().get::<Health>(unit);
            eprintln!(
                "REPLAY_DEATH_AUDIT {}",
                json!({
                    "unit": unit.to_bits(), "spells": spells,
                    "health_after_tick": health.map(|health| health.current),
                    "alive_after_tick": health.is_some_and(|health| health.current > 0.0),
                })
            );
        }
    }
}

/// The dungeon whose Journal gear a class party at `level` wears: the follower
/// DPS harness's content for each measured level.
fn measurement_map(level: u8) -> Result<u32, String> {
    match level {
        70 => Ok(2515),
        90 => Ok(2813),
        level => Err(format!("HEALER_REPLAY_LEVEL_UNSUPPORTED:{level}")),
    }
}

/// The measurement snapshot named by `FOLLOWER_DPS_WORLD_DB` and its SHA-256.
fn read_snapshot() -> Result<(PathBuf, String), String> {
    let path = std::env::var_os("FOLLOWER_DPS_WORLD_DB").ok_or("FOLLOWER_DPS_WORLD_DB absent")?;
    hash_snapshot(Path::new(&path))
}

/// `path` resolved and hashed. Snapshot measurements never read the canonical
/// world database, and every catalog must come from the same snapshot.
fn hash_snapshot(path: &Path) -> Result<(PathBuf, String), String> {
    use sha2::Digest;
    let path = path
        .canonicalize()
        .map_err(|error| format!("measurement snapshot: {error}"))?;
    if Path::new(CANONICAL_WORLD_DB).canonicalize().ok().as_ref() == Some(&path) {
        return Err("HEALER_REPLAY_REQUIRES_SNAPSHOT_WORLD_DB".into());
    }
    let catalogs = crate::test_data::world_db_path()
        .canonicalize()
        .map_err(|error| error.to_string())?;
    if catalogs != path {
        return Err("HEALER_REPLAY_CATALOGS_NOT_FROM_SNAPSHOT".into());
    }
    let mut file = std::fs::File::open(&path)
        .map_err(|error| format!("read measurement snapshot: {error}"))?;
    let mut hasher = sha2::Sha256::new();
    std::io::copy(&mut file, &mut hasher)
        .map_err(|error| format!("hash measurement snapshot: {error}"))?;
    Ok((path, format!("{:x}", hasher.finalize())))
}

pub(super) fn apply_optimal_raid_environment(
    app: &mut App,
    source: Entity,
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
        super::tank_replay::apply_simc_raid_buffs_to(app, member, &buffs)?;
    }
    super::tank_replay::apply_simc_dummy_debuffs(app, members[0], source, &debuffs)?;
    for &member in members {
        super::tank_replay::apply_replay_bloodlust(app.world_mut(), member)?;
    }
    Ok(())
}

pub(super) fn replay_simc(
    spec: u32,
    level: u8,
    trace: &Path,
    duration: u32,
    seed: u64,
) -> Result<Value, String> {
    replay(spec, level, trace, None, duration, seed, 1)
}

pub(super) fn replay_simc_party(
    spec: u32,
    level: u8,
    trace: &Path,
    party_trace: &Path,
    duration: u32,
    seed: u64,
) -> Result<Value, String> {
    replay_simc_party_with_targets(spec, level, trace, party_trace, duration, seed, 1)
}

pub(super) fn replay_simc_party_with_targets(
    spec: u32,
    level: u8,
    trace: &Path,
    party_trace: &Path,
    duration: u32,
    seed: u64,
    targets: usize,
) -> Result<Value, String> {
    replay(
        spec,
        level,
        trace,
        Some(party_trace),
        duration,
        seed,
        targets,
    )
}

/// A class party (tank, `spec` healer, two DPS and the leader DPS), each in
/// its level's Journal gear and casting through its spec APL, engaged on the
/// SimC tank dummy whose recorded pressure is replayed onto it.
pub(super) struct ClassParty {
    pub(super) app: App,
    /// The engaged dummies; the first one is the recorded pressure source.
    pub(super) sources: Vec<Entity>,
    pub(super) tank: Entity,
    pub(super) healer: Entity,
    /// Tank, healer, two DPS and the leader: the SimC party trace's slots.
    pub(super) slots: [Entity; 5],
}

/// An unengaged class party over the `FOLLOWER_DPS_WORLD_DB` snapshot with
/// `targets` dummies: its APLs stay idle until `engage`.
#[cfg(test)]
pub(super) fn snapshot_party(spec: u32, level: u8, targets: usize) -> ClassParty {
    let (path, _) = read_snapshot().unwrap();
    spawn_class_party(&path, spec, level, targets).unwrap()
}

fn spawn_class_party(
    path: &Path,
    spec: u32,
    level: u8,
    targets: usize,
) -> Result<ClassParty, String> {
    let mut app = super::dps_parity::build_snapshot_app(path)?;
    let copy = super::dps_parity::create_open_follower_copy(&mut app, measurement_map(level)?)?;
    let origin = shared::components::Position {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    };
    let party = crate::followers::simulator::spawn_party_with_healer(
        app.world_mut(),
        copy,
        origin,
        level,
        spec,
    )?;
    let (tank, healer) = (party.tank, party.members[1]);
    let sources = (0..targets)
        .map(|index| super::dps_parity::spawn_level_dummy(&mut app, tank, level, index))
        .collect::<Result<Vec<_>, _>>()?;
    app.world_mut()
        .entity_mut(healer)
        .insert(crate::spell_cast::ObservedResourcePayments::default());
    Ok(ClassParty {
        app,
        sources,
        tank,
        healer,
        slots: [
            tank,
            healer,
            party.members[2],
            party.members[3],
            party.owner,
        ],
    })
}

/// The leader attacks the first dummy and enters combat with every dummy,
/// which engages the whole party as a player's pull does.
pub(super) fn engage(party: &mut ClassParty) {
    let leader = party.slots[4];
    let world = party.app.world_mut();
    world.entity_mut(leader).insert((
        crate::networking::CombatTarget(Some(party.sources[0])),
        crate::combat::AutoAttack::new(party.sources[0]),
    ));
    for &source in &party.sources {
        crate::combat_refs::set_in_combat_with(world, leader, source);
    }
}

/// Hand `unit` to the test: neither its APL nor the follower brain casts, so
/// the test's own casts are the only ones it makes.
#[cfg(test)]
pub(super) fn hold(party: &mut ClassParty, unit: Entity) {
    party
        .app
        .world_mut()
        .entity_mut(unit)
        .remove::<crate::followers::AplControlled>()
        .insert(server::test_support::LiteralParityFixture);
}

/// Equip `item_id` exactly as `spec`'s tracked APL profile lists it, replacing
/// the class-built item in that slot.
#[cfg(test)]
pub(super) fn equip_profile_item(party: &mut ClassParty, unit: Entity, spec: u32, item_id: u32) {
    let profile = server_follower_apl::profiles::profile(spec).unwrap();
    let item = profile
        .equipment
        .iter()
        .find(|item| item.item_id == item_id)
        .unwrap_or_else(|| panic!("spec{spec} profile lacks item{item_id}"));
    let slot = crate::followers::parity_profile::slot(&item.slot).unwrap();
    let mut entity = party.app.world_mut().entity_mut(unit);
    let mut items = entity.get_mut::<crate::stats::EquippedItems>().unwrap();
    items.items.retain(|(equipped, _, _)| *equipped != slot);
    items.items.push((
        slot,
        shared::item_data::ItemDefinitionSource::Retail,
        item_id,
    ));
    let mut modifiers = entity
        .get_mut::<crate::stats::EquippedItemModifiers>()
        .unwrap();
    modifiers.items.retain(|(equipped, _)| *equipped != slot);
    modifiers.items.push((slot, item.modifiers()));
    party.app.update();
}

/// Run the party's APL precombat until the tank and healer enter combat.
pub(super) fn finish_precombat(app: &mut App, actors: &[Entity]) -> Result<(), String> {
    for _ in 0..MAX_PRECOMBAT_TICKS {
        app.update();
        let world = app.world();
        if let Some(&failed) = actors.iter().find(|&&actor| {
            world
                .get::<server_follower_apl::FollowerAplFailed>(actor)
                .is_some()
        }) {
            return Err(format!("HEALER_REPLAY_APL_FAILED:{failed}"));
        }
        if actors.iter().all(|&actor| {
            world
                .get::<server_follower_apl::FollowerApl>(actor)
                .is_some_and(|apl| apl.in_combat())
        }) {
            return Ok(());
        }
    }
    Err("HEALER_REPLAY_PRECOMBAT_INCOMPLETE".into())
}

fn replay(
    spec: u32,
    level: u8,
    trace: &Path,
    party_trace: Option<&Path>,
    duration: u32,
    seed: u64,
    targets: usize,
) -> Result<Value, String> {
    if duration == 0 {
        return Err("SIMC_REPLAY_DURATION_ZERO".into());
    }
    if targets == 0 {
        return Err("SIMC_REPLAY_TARGETS_ZERO".into());
    }
    let ticks = duration
        .checked_mul(TICKS_PER_SECOND)
        .ok_or("SIMC_REPLAY_DURATION_OUT_OF_RANGE")?;
    let (snapshot, snapshot_sha256) = read_snapshot()?;
    let hits = crate::simc_tank_timeline::read_trace(trace, "Tank_Dummy_Dungeon")?;
    let party_hits = party_trace
        .map(super::healer_party_trace::read)
        .transpose()?
        .unwrap_or_default();
    let _rng = crate::random::seed_scope(seed);
    let mut party = spawn_class_party(&snapshot, spec, level, targets)?;
    #[cfg(test)]
    if std::env::var_os("FOLLOWER_AUDIT_DEATH").is_some() {
        party.app.init_resource::<crate::auras::DeathAuraAudit>();
    }
    // SimC replays a fixed-length fight against a dummy that never dies.
    party
        .app
        .insert_resource(server_follower_apl::registry::FixedLengthFight {
            duration: f64::from(duration),
        });
    engage(&mut party);
    let ClassParty {
        mut app,
        sources,
        tank,
        healer,
        slots,
    } = party;
    let source = sources[0];
    apply_optimal_raid_environment(&mut app, source, &slots)?;
    let debuffs = [("chaos_brand", 1490), ("mystic_touch", 113746)].map(|(name, spell_id)| {
        crate::simc_tank_timeline::RaidBuff {
            name: name.into(),
            spell_id,
        }
    });
    for &extra in &sources[1..] {
        super::tank_replay::apply_simc_dummy_debuffs(&mut app, tank, extra, &debuffs)?;
    }
    finish_precombat(&mut app, &[tank, healer])?;
    #[cfg(test)]
    report_death_audit(&mut app, &slots);
    let frozen_sources = sources
        .iter()
        .enumerate()
        .map(|(index, &source)| (index as u32, source))
        .collect();
    super::healer_replay::freeze_sources(app.world_mut(), &frozen_sources);
    app.init_resource::<server_spell_scripts::test_support::FiendAtonementReceipts>();
    app.insert_resource(Capture {
        members: slots.to_vec(),
        sources: [(source, (0, 0))].into(),
        ..Default::default()
    });
    let mut recorder = Recorder::start(app.world(), healer, tank, slots.to_vec());
    let mut event_cursor = app
        .world()
        .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
        .0
        .len();
    let mut tank_casts = std::collections::BTreeMap::<u32, u32>::new();
    let mut healer_casts = std::collections::BTreeMap::<u32, u32>::new();
    let mut mitigation_ticks = 0_u32;
    let mut next = 0;
    let mut next_party = 0;
    let mut pain_target_ticks = 0_u64;
    let mut atonement_target_ticks = 0_u64;
    let mut pain_periodic_hits = 0_u64;
    let mut damage_by_dummy = std::collections::BTreeMap::<u64, u64>::new();
    let alive = |world: &World, unit: Entity| {
        world
            .get::<Health>(unit)
            .is_some_and(|health| health.current > 0.0)
    };
    for tick in 1..=ticks {
        let elapsed_ms = u64::from(tick) * 50;
        while let Some(hit) = hits.get(next).filter(|hit| hit.time_ms <= elapsed_ms) {
            if alive(app.world(), tank) {
                deliver_hit(app.world_mut(), source, tank, hit, tick)?;
                recorder.sample_minima(app.world());
            }
            next += 1;
        }
        while let Some(hit) = party_hits
            .get(next_party)
            .filter(|hit| hit.time_ms <= elapsed_ms)
        {
            let target = slots[hit.slot];
            if alive(app.world(), target) {
                deliver_party_hit(app.world_mut(), source, target, hit.amount);
                recorder.sample_minima(app.world());
            }
            next_party += 1;
        }
        recorder.observe_idle_gcd(app.world());
        app.update();
        #[cfg(test)]
        report_death_audit(&mut app, &slots);
        let world = app.world();
        recorder.observe_frame(world, tick);
        let marked = |unit: &Entity, spell| {
            world
                .get::<crate::auras::ActiveAuras>(*unit)
                .is_some_and(|auras| {
                    auras
                        .iter()
                        .any(|aura| aura.spell_id == spell && aura.caster == healer)
                })
        };
        pain_target_ticks += sources.iter().filter(|unit| marked(unit, 589)).count() as u64;
        atonement_target_ticks += slots.iter().filter(|unit| marked(unit, 194384)).count() as u64;
        // The default party's tank is a Protection Paladin.
        if super::dps_parity::PROTECTION_ACTIVE_MITIGATION_AURAS
            .iter()
            .any(|&aura| crate::auras::has_aura(world, tank, aura))
        {
            mitigation_ticks += 1;
        }
        let events = &world
            .resource::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0;
        for event in &events[event_cursor..] {
            if event.kind == shared::protocol::CombatLogKind::Damage
                && event
                    .target
                    .is_some_and(|target| sources.iter().any(|source| source.to_bits() == target))
            {
                *damage_by_dummy.entry(event.target.unwrap()).or_default() +=
                    event.amount.max(0) as u64;
            }
            #[cfg(test)]
            if std::env::var_os("HEALER_TANK_TRACE_EVENTS").is_some()
                && (event.source == Some(healer.to_bits()) || event.target == Some(tank.to_bits()))
            {
                eprintln!(
                    "HEALER_REPLAY_EVENT {}",
                    json!({"tick":tick,"event":event,"tank_health":world.get::<Health>(tank).map(|health| (health.current, health.max))})
                );
                if event.spell_id == Some(1253593)
                    && let Some(target) = event.target.and_then(Entity::try_from_bits)
                    && !slots.contains(&target)
                {
                    eprintln!(
                        "HEALER_REPLAY_NONPARTY {}",
                        json!({"tick": tick, "target": target.to_bits(),
                            "npc": world.get::<shared::components::Npc>(target).map(|npc| &npc.name),
                            "summon": format!("{:?}", world.get::<crate::spell_cast::summon::SummonedCreature>(target)),
                            "hunter_pet": format!("{:?}", world.get::<crate::hunter_pet::HunterPet>(target))})
                    );
                }
            }
            if event.source == Some(healer.to_bits())
                && event.spell_id == Some(589)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.periodic
            {
                pain_periodic_hits += 1;
            }
            if event.kind != shared::protocol::CombatLogKind::CastSuccess {
                continue;
            }
            let Some(spell) = event.spell_id else {
                continue;
            };
            if event.source == Some(tank.to_bits()) {
                *tank_casts.entry(spell).or_default() += 1;
            } else if event.source == Some(healer.to_bits()) {
                *healer_casts.entry(spell).or_default() += 1;
            }
        }
        event_cursor = events.len();
    }
    Ok(
        json!({"spec":spec,"level":level,"actors":"class_followers","timeline":"simc_tank_dummy",
        "world_db_sha256":snapshot_sha256,
        "duration_seconds":duration,"target_count":targets,"incoming_events":next,"party_incoming_events":next_party,"tank_spec":crate::followers::simulator::DEFAULT_PARTY[0],
        "pain_target_seconds":pain_target_ticks as f64 / f64::from(TICKS_PER_SECOND),
        "atonement_target_seconds":atonement_target_ticks as f64 / f64::from(TICKS_PER_SECOND),
        "pain_periodic_hits":pain_periodic_hits,"damage_by_dummy":damage_by_dummy,
        "tank_casts":tank_casts,"healer_casts":healer_casts,
        "tank_active_mitigation_seconds":f64::from(mitigation_ticks) / f64::from(TICKS_PER_SECOND),
        "metrics":recorder.report(app.world(), duration)}),
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
