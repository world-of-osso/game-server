//! Native 300s dummy qualification with explicit per-spec fixture controllers.
include!("module_list.in");
use crate::simc_apl_native::{FixtureAdds, FixtureTarget};
use bevy::prelude::*;
pub(crate) use inputs::equip_companions;
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::time::{Duration, Instant};

const TICKS: u32 = 6_000;
const ADD_WAVE_TICKS: u32 = 600;
const LAST_30S_START: u32 = TICKS - ADD_WAVE_TICKS;
#[cfg(test)]
mod consecration_vision_tests;
#[cfg(test)]
mod elemental_rod_tests;
#[cfg(test)]
mod follower_dps_diagnostic;
#[cfg(test)]
mod follower_pull_safety_tests;
#[cfg(test)]
mod follower_tank_precombat;
#[cfg(test)]
mod follower_tank_scenario;
#[cfg(test)]
mod refining_fire_tests;
#[cfg(test)]
mod tank_fixture_tests;
#[cfg(test)]
mod vengeance_consume_soul_tests;
#[cfg(test)]
mod vengeful_wrath_tests;

const DUMMY_ENTRY: u32 = 31_146;
const DUMMY_LATERAL_STEP: f32 = 0.5;
const FIGHT_DURATION: Duration = Duration::from_secs(300);
const ADD_LIFETIME: Duration = Duration::from_secs(30);
// SimC76f8118 raid_event.cpp2700–2701: equal cooldown/duration adds10ms.
const ADD_INTERVAL: Duration = Duration::from_millis(30_010);

pub(crate) trait SpecDriver: std::any::Any {
    fn supports_actor(_identity: &crate::followers::simulator::ParityActor) -> bool
    where
        Self: Sized,
    {
        true
    }

    fn drive(&mut self, world: &mut World, actor: Entity, targets: &[Entity])
    -> Result<(), String>;
    fn label(&self) -> &'static str;
    /// Tank adapters must explicitly identify their native active-mitigation auras.
    fn tank_active_mitigation_auras(&self) -> Option<&'static [u32]> {
        None
    }
    /// Named monotonic counters read at combat start and after replay.
    fn tank_resource_counters(
        &self,
        _world: &World,
        _actor: Entity,
    ) -> BTreeMap<&'static str, u64> {
        BTreeMap::new()
    }
    /// Native target debuffs owned by this tank, sampled over replay time.
    fn tank_target_debuff_auras(&self) -> &'static [u32] {
        &[]
    }
    fn report(&self) -> Value {
        Value::Null
    }
}

type PrepareDriver = fn(&mut Run) -> Result<Box<dyn SpecDriver>, String>;

struct SpecRegistration {
    spec_id: u32,
    prepare: PrepareDriver,
    supports_actor: fn(&crate::followers::simulator::ParityActor) -> bool,
    report_key: Option<&'static str>,
}

// Register a spec only here and in its own module. The table is sorted by spec ID.
// Until rustfmt traverses macro-declared modules, format changed rotation files
// directly with `rustfmt --edition 2024` as well as running `cargo fmt`.
macro_rules! spec_drivers {
    ($($spec_id:literal => $module:ident,)*) => {
        $(mod $module;)*
        const DRIVERS: &[SpecRegistration] = &[$(SpecRegistration {
            spec_id: $spec_id,
            prepare: $module::prepare_driver,
            supports_actor: <$module::Rotation as SpecDriver>::supports_actor,
            report_key: $module::REPORT_KEY,
        },)*];
    };
}

spec_drivers! {
    62 => arcane_rotation,
    63 => fire_rotation,
    64 => frost_mage_rotation,
    66 => protection_paladin_rotation,
    70 => ret_rotation,
    71 => arms_rotation,
    72 => fury_rotation,
    73 => protection_warrior_rotation,
    103 => feral_rotation,
    104 => guardian_tank_rotation,
    250 => blood_tank_rotation,
    251 => frost_rotation,
    252 => unholy_rotation,
    253 => beast_mastery_rotation,
    254 => marksmanship_rotation,
    255 => survival_rotation,
    258 => shadow_rotation,
    259 => assassination_rotation,
    260 => outlaw_rotation,
    261 => subtlety_rotation,
    262 => elemental_rotation,
    263 => enhancement_rotation,
    265 => affliction_rotation,
    266 => demonology_rotation,
    267 => destruction_rotation,
    268 => brewmaster_rotation,
    269 => windwalker_rotation,
    577 => havoc_rotation,
    581 => vengeance_tank_rotation,
    1467 => devastation_rotation,
    1480 => devourer_rotation,
}

pub(crate) struct Run {
    pub(crate) app: App,
    pub(crate) actor: Entity,
    pub(crate) targets: Vec<Entity>,
    damage_targets: Vec<Entity>,
    pub(crate) target_waves: Vec<Value>,
    profile: inputs::Profile,
    gaps: Vec<String>,
    player_rotation: Option<player_rotation::PlayerRotation>,
    spec_driver: Option<Box<dyn SpecDriver>>,
}

fn spawn_dummy(app: &mut App, actor: Entity, index: usize) -> Result<Entity, String> {
    // Keep the reference's three-target pack in front without changing native cones.
    let lateral_step = index.div_ceil(2) as f32 * DUMMY_LATERAL_STEP;
    let lateral = if index.is_multiple_of(2) {
        lateral_step
    } else {
        -lateral_step
    };
    let target = crate::zones::summon_creature(
        app.world_mut(),
        actor,
        DUMMY_ENTRY,
        [1.5, lateral, 0.0, 0.0],
    )
    .ok_or("native training dummy31146 could not spawn")?;
    if app
        .world()
        .get::<crate::creature_scripts::TrainingDummy>(target)
        .is_none()
    {
        return Err("native31146 lacks its actual training-dummy script".into());
    }
    face_dummy_away_from_actor(app.world_mut(), actor, target);
    // SIMC_FIXED_TIME_TARGET_HEALTH owns this fixture's current health. Source31146
    // has RegenHealth=1; its ordinary two-second heal must not alter phase gates.
    // Keep the template, script, flags and damage handling unchanged.
    app.world_mut()
        .entity_mut(target)
        .remove::<crate::spell_cast::powers::CreatureHealthRegenTimer>();
    Ok(target)
}

// SimC76f8118 attack.cpp277–279: position=back excludes frontal parries.
// Keep the native pack positions/cones; change only the opponent's facing.
fn face_dummy_away_from_actor(world: &mut World, actor: Entity, target: Entity) {
    use shared::components::{Position, Rotation};
    let actor = *world.get::<Position>(actor).expect("parity actor position");
    let target_position = *world
        .get::<Position>(target)
        .expect("parity dummy position");
    let orientation = (actor.z - target_position.z).atan2(target_position.x - actor.x);
    world
        .get_mut::<Rotation>(target)
        .expect("parity dummy rotation")
        .y = orientation + std::f32::consts::FRAC_PI_2;
}

fn spawn_targets(app: &mut App, actor: Entity, count: usize) -> Result<Vec<Entity>, String> {
    (0..count)
        .map(|index| spawn_dummy(app, actor, index))
        .collect()
}

pub(super) fn install_fixture_lifetimes(run: &mut Run, started_at: Duration) {
    for (index, &target) in run.targets.iter().enumerate() {
        let lifetime = if index == 0 {
            FIGHT_DURATION
        } else {
            ADD_LIFETIME
        };
        run.app
            .world_mut()
            .entity_mut(target)
            .insert(FixtureTarget {
                expires_at: started_at + lifetime,
                lifetime,
                is_boss: index == 0,
            });
    }
    if run.targets.len() == 3 {
        run.app
            .world_mut()
            .entity_mut(run.actor)
            .insert(FixtureAdds {
                next_spawn_at: started_at + ADD_INTERVAL,
                expires_at: Some(started_at + ADD_LIFETIME),
                lifetime: ADD_LIFETIME,
                interval: ADD_INTERVAL,
                count: 2,
            });
    }
}

fn retire_add_wave(run: &mut Run) -> Vec<Entity> {
    let retired = run.targets.split_off(1);
    for &unit in &retired {
        crate::zones::unsummon(run.app.world_mut(), unit);
    }
    run.app
        .world_mut()
        .get_mut::<FixtureAdds>(run.actor)
        .unwrap()
        .expires_at = None;
    retired
}

fn spawn_add_wave(run: &mut Run, retired: &[Entity], started_at: Duration) -> Result<(), String> {
    let now = run.app.world().resource::<Time>().elapsed();
    let adds = *run.app.world().get::<FixtureAdds>(run.actor).unwrap();
    for index in 1..=adds.count {
        let unit = spawn_dummy(&mut run.app, run.actor, index)?;
        run.app.world_mut().entity_mut(unit).insert(FixtureTarget {
            expires_at: now + adds.lifetime,
            lifetime: adds.lifetime,
            is_boss: false,
        });
        run.targets.push(unit);
        run.damage_targets.push(unit);
    }
    {
        let mut state = run
            .app
            .world_mut()
            .get_mut::<FixtureAdds>(run.actor)
            .unwrap();
        state.expires_at = Some(now + adds.lifetime);
        state.next_spawn_at = now + adds.interval;
    }
    apply_fixed_time_target_health(run, now)?;
    engage_fixture_opponents(run);
    let retired_still_present = retired
        .iter()
        .filter(|&&unit| run.app.world().get_entity(unit).is_ok())
        .count();
    run.target_waves
        .push(json!({"at": (now - started_at).as_secs_f64(),
        "entities": run.targets.iter().map(|unit| unit.to_bits()).collect::<Vec<_>>(),
        "retired_still_present": retired_still_present}));
    Ok(())
}

pub(crate) fn read_reference(
    spec_id: u32,
) -> Result<(inputs::Profile, Vec<inputs::AllocatedTrait>), String> {
    let directory = inputs::directory()?;
    let profile: inputs::Profile =
        inputs::read_json(&directory.join(format!("{spec_id}.profile.json")))?;
    if profile.spec_id != spec_id {
        return Err("reference spec identity mismatch".into());
    }
    let traits =
        inputs::read_json(&directory.join(format!("{spec_id}.selected-traits-active.json")))?;
    Ok((profile, traits))
}

fn prepare(spec_id: u32, count: usize) -> Result<Run, String> {
    let (profile, traits) = read_reference(spec_id)?;
    let identity = inputs::read_actor(&profile)?;
    prepare_reference(profile, &traits, count, identity)
}

/// Capture the explicit literal controller before any native setup frame.
fn prepare_literal(spec_id: u32, count: usize) -> Result<Run, String> {
    let (profile, traits) = read_reference(spec_id)?;
    let identity = inputs::read_actor(&profile)?;
    prepare_literal_reference(profile, &traits, count, identity)
}

fn prepare_literal_reference(
    profile: inputs::Profile,
    traits: &[inputs::AllocatedTrait],
    count: usize,
    identity: crate::followers::simulator::ParityActor,
) -> Result<Run, String> {
    let mut run = spawn_literal_reference(profile, traits, count, identity)?;
    preparation::consume_profile_food(&mut run)?;
    Ok(run)
}

fn spawn_literal_reference(
    profile: inputs::Profile,
    traits: &[inputs::AllocatedTrait],
    count: usize,
    identity: crate::followers::simulator::ParityActor,
) -> Result<Run, String> {
    let mut run = spawn_reference(profile, traits, count, identity)?;
    let encounter = crate::simc_apl_native::Encounter::Fixture {
        started_at: crate::spell_cast::now(run.app.world()),
        duration: f64::from(TICKS) / 20.0,
        style: crate::simc_apl_native::FightStyle::Patchwerk,
    };
    literal_rotation::capture_fixture(run.app.world_mut(), run.actor, encounter)?;
    Ok(run)
}

fn measurement_driver(
    spec_id: u32,
    identity: &crate::followers::simulator::ParityActor,
) -> Option<&'static SpecRegistration> {
    let index = DRIVERS
        .binary_search_by_key(&spec_id, |driver| driver.spec_id)
        .ok()?;
    let driver = &DRIVERS[index];
    (driver.supports_actor)(identity).then_some(driver)
}

#[test]
fn literal_spec_selection_applies_to_player_and_follower() {
    use crate::followers::simulator::ParityActor;
    for spec_id in [
        63, 64, 70, 71, 72, 253, 254, 258, 259, 260, 262, 263, 269, 577, 1467,
    ] {
        for identity in [ParityActor::Player { race: 1 }, ParityActor::Follower] {
            assert!(
                measurement_driver(spec_id, &identity).is_some(),
                "spec {spec_id}"
            );
        }
    }
    assert!(measurement_driver(0, &ParityActor::Player { race: 1 }).is_none());
    assert!(measurement_driver(0, &ParityActor::Follower).is_none());
}

#[test]
fn literal_follower_only_specs_explicitly_preserve_their_actor_restriction() {
    use crate::followers::simulator::ParityActor;
    for spec_id in [70, 266] {
        assert!(measurement_driver(spec_id, &ParityActor::Follower).is_some());
        assert!(measurement_driver(spec_id, &ParityActor::Player { race: 1 }).is_none());
    }
}

fn prepare_measurement(spec_id: u32, count: usize) -> Result<Run, String> {
    prepare_measurement_with(spec_id, count, |_| Ok(()))
}

/// Prepare only a registered literal player driver; never substitute a follower policy.
pub(crate) fn prepare_registered_player(spec_id: u32, count: usize) -> Result<Run, String> {
    prepare_registered_player_with(spec_id, count, |_| Ok(()))
}

pub(crate) fn prepare_registered_player_with(
    spec_id: u32,
    count: usize,
    before_prepare: impl FnOnce(&mut Run) -> Result<(), String>,
) -> Result<Run, String> {
    if !DRIVERS.iter().any(|driver| driver.spec_id == spec_id) {
        return Err(format!("NATIVE_LITERAL_DRIVER_UNAVAILABLE:{spec_id}"));
    }
    let (profile, _) = read_reference(spec_id)?;
    require_mid2_profile(&profile)?;
    let identity = inputs::read_actor(&profile)?;
    if !matches!(
        identity,
        crate::followers::simulator::ParityActor::Player { .. }
    ) {
        return Err("TANK_REPLAY_REQUIRES_PLAYER_ACTOR".into());
    }
    if measurement_driver(spec_id, &identity).is_none() {
        return Err(format!("NATIVE_LITERAL_DRIVER_UNAVAILABLE:{spec_id}"));
    }
    prepare_measurement_with(spec_id, count, before_prepare)
}

pub(crate) fn require_mid2_profile(profile: &inputs::Profile) -> Result<(), String> {
    if profile.level != "90"
        || profile
            .options
            .get("reference_tier")
            .and_then(Value::as_str)
            != Some("MID2")
    {
        return Err(format!(
            "NATIVE_TANK_REPLAY_REQUIRES_MID2_LEVEL_90:{}",
            profile.spec_id
        ));
    }
    Ok(())
}

pub(crate) fn tank_mitigation_auras(run: &Run) -> Result<&'static [u32], String> {
    run.spec_driver
        .as_ref()
        .and_then(|driver| driver.tank_active_mitigation_auras())
        .filter(|auras| !auras.is_empty())
        .ok_or_else(|| {
            format!(
                "NATIVE_TANK_MITIGATION_AURAS_UNAVAILABLE:{}",
                run.profile.spec_id
            )
        })
}

pub(crate) fn tank_resource_counters(run: &Run) -> Result<BTreeMap<&'static str, u64>, String> {
    let driver = run
        .spec_driver
        .as_ref()
        .ok_or("NATIVE_TANK_DRIVER_UNAVAILABLE")?;
    Ok(driver.tank_resource_counters(run.app.world(), run.actor))
}

pub(crate) fn tank_target_debuff_auras(run: &Run) -> Result<&'static [u32], String> {
    let driver = run
        .spec_driver
        .as_ref()
        .ok_or("NATIVE_TANK_DRIVER_UNAVAILABLE")?;
    Ok(driver.tank_target_debuff_auras())
}

/// `before_prepare` runs after consumables, before the driver's pre-pull plan.
fn prepare_measurement_with(
    spec_id: u32,
    count: usize,
    before_prepare: impl FnOnce(&mut Run) -> Result<(), String>,
) -> Result<Run, String> {
    let (profile, traits) = read_reference(spec_id)?;
    let identity = inputs::read_actor(&profile)?;
    if let Some(driver) = measurement_driver(spec_id, &identity) {
        let mut run = spawn_literal_reference(profile, &traits, count, identity)?;
        if spec_id == 64 {
            frost_mage_rotation::configure_actor_reactions(run.app.world_mut(), run.actor)?;
        } else if spec_id == 1480 {
            devourer_rotation::configure_actor_reactions(run.app.world_mut(), run.actor)?;
        }
        preparation::consume_profile_food(&mut run)?;
        before_prepare(&mut run)?;
        run.spec_driver = Some((driver.prepare)(&mut run)?);
        Ok(run)
    } else {
        prepare_reference(profile, &traits, count, identity)
    }
}

fn prepare_reference(
    profile: inputs::Profile,
    traits: &[inputs::AllocatedTrait],
    count: usize,
    identity: crate::followers::simulator::ParityActor,
) -> Result<Run, String> {
    let mut run = spawn_reference(profile, traits, count, identity)?;
    preparation::consume_profile_food(&mut run)?;
    preparation::start_configured_native_pet(&mut run)?;
    preparation::finish_pending_setup(&mut run)?;
    Ok(run)
}

pub(crate) fn spawn_reference(
    profile: inputs::Profile,
    traits: &[inputs::AllocatedTrait],
    count: usize,
    identity: crate::followers::simulator::ParityActor,
) -> Result<Run, String> {
    let mut app = super::runtime::build_app();
    app.add_systems(Last, melee_return::advance);
    inputs::install_reference_catalogs(&mut app)?;
    let copy = super::runtime::create_copy(&mut app);
    // Origin-centered parity fixtures have no frozen dungeon geometry.
    app.world_mut().remove_resource::<super::Space>();
    crate::simc_apl_native::install_open_arena(app.world_mut());
    let actor = spawn_reference_actor(&mut app, copy, &profile, traits, identity)?;
    let targets = spawn_targets(&mut app, actor, count)?;
    let target_waves = vec![
        json!({"at": 0.0, "entities": targets.iter().map(|unit| unit.to_bits()).collect::<Vec<_>>()}),
    ];
    let is_player = app
        .world()
        .get::<shared::components::Player>(actor)
        .is_some();
    let gaps = report::gaps(&profile, is_player);
    Ok(Run {
        app,
        actor,
        damage_targets: targets.clone(),
        targets,
        target_waves,
        profile,
        gaps,
        player_rotation: None,
        spec_driver: None,
    })
}

fn spawn_reference_actor(
    app: &mut App,
    copy: crate::maps::InMap,
    profile: &inputs::Profile,
    traits: &[inputs::AllocatedTrait],
    identity: crate::followers::simulator::ParityActor,
) -> Result<Entity, String> {
    let level = profile
        .level
        .parse()
        .map_err(|error| format!("reference level: {error}"))?;
    let actor = crate::followers::simulator::spawn_parity_attacker(
        app.world_mut(),
        profile.spec_id,
        level,
        copy,
        identity,
    )?;
    // Pinned player.cpp1288: this Patchwerk fixture configures no movement event.
    app.world_mut()
        .entity_mut(actor)
        .insert(crate::simc_apl_native::FixtureMovement {
            distance_to_move: 0.0,
        });
    let race = inputs::read_reference_race(&profile.race)?;
    app.world_mut()
        .entity_mut(actor)
        .insert(server::test_support::ReferenceRace(race));
    inputs::equip(app.world_mut(), actor, profile)?;
    inputs::install_traits(app.world_mut(), actor, traits)?;
    profile_initialization::initialize_declared_powers(app.world_mut(), actor, profile)?;
    profile_effects::install(app.world_mut(), actor, profile)?;
    Ok(actor)
}

/// Install a second MID2 player in the registered actor's existing native world.
pub(crate) fn spawn_second_reference_player(run: &mut Run, spec_id: u32) -> Result<Entity, String> {
    let (profile, traits) = read_reference(spec_id)?;
    require_mid2_profile(&profile)?;
    let race = inputs::read_reference_race(&profile.race)?;
    let copy = *run
        .app
        .world()
        .get::<crate::maps::InMap>(run.actor)
        .ok_or("reference tank copy absent")?;
    let actor = spawn_reference_actor(
        &mut run.app,
        copy,
        &profile,
        &traits,
        crate::followers::simulator::ParityActor::Player { race },
    )?;
    // The reference party is single-faction, including during setup updates.
    let faction = *run
        .app
        .world()
        .get::<shared::components::UnitFactionTemplate>(run.actor)
        .ok_or("reference tank faction absent")?;
    run.app.world_mut().entity_mut(actor).insert(faction);
    crate::followers::simulator::assign_reference_player_id(run.app.world_mut(), actor, 2);
    for _ in 0..4 {
        run.app.update();
    }
    profile_effects::install_food(run.app.world_mut(), actor, &profile)?;
    preparation::finish_pending_setup(run)?;
    Ok(actor)
}

fn fixture_player(run: &Run) -> Entity {
    let world = run.app.world();
    if world.get::<shared::components::Player>(run.actor).is_some() {
        run.actor
    } else {
        world
            .get::<crate::followers::Follower>(run.actor)
            .unwrap()
            .owner
    }
}

fn engage_fixture_opponents(run: &mut Run) {
    let owner = fixture_player(run);
    for &target in &run.targets {
        crate::combat_refs::set_in_combat_with(run.app.world_mut(), owner, target);
    }
}

// SimC76f8118 sc_demon_hunter.cpp3927–3938 models Eye Beam as target AoE.
// The native fixture pack is at +X; player Rotation.y=0 instead faces +Z.
// Establish encounter facing, not a replacement cone or target list.
fn face_actor_toward_primary(world: &mut World, actor: Entity, primary: Entity) {
    use shared::components::{Position, Rotation};
    let from = *world
        .get::<Position>(actor)
        .expect("fixture actor position");
    let to = *world
        .get::<Position>(primary)
        .expect("fixture primary position");
    let orientation = (from.z - to.z).atan2(to.x - from.x);
    world
        .get_mut::<Rotation>(actor)
        .expect("fixture actor rotation")
        .y = orientation + std::f32::consts::FRAC_PI_2;
}

pub(crate) fn engage(run: &mut Run) {
    face_actor_toward_primary(run.app.world_mut(), run.actor, run.targets[0]);
    for &target in &run.targets {
        face_dummy_away_from_actor(run.app.world_mut(), run.actor, target);
    }
    let owner = fixture_player(run);
    let world = run.app.world_mut();
    // A player selects its target directly; a follower retains its quiet controller.
    let literal_player = owner == run.actor
        && world
            .get::<literal_rotation::LiteralParityFixture>(run.actor)
            .is_some();
    world
        .entity_mut(owner)
        .insert(crate::networking::CombatTarget(Some(run.targets[0])));
    // SimC76f8118 sc_hunter.cpp7269–7274: the literal source starts its
    // weapon action. Engagement must not inject an extra melee attack first.
    if !literal_player {
        world
            .entity_mut(owner)
            .insert(crate::combat::AutoAttack::new(run.targets[0]));
    }
    if world
        .get::<literal_rotation::LiteralParityFixture>(run.actor)
        .is_none()
    {
        world
            .resource_mut::<crate::spell_cast::test_harness::ObservedCombatEvents>()
            .0
            .clear();
    }
    engage_fixture_opponents(run);
}

fn target_snapshot(run: &Run) -> Vec<Value> {
    let world = run.app.world();
    run.targets
        .iter()
        .map(|&target| {
            let native = *world.get::<shared::components::UnitLevel>(target).unwrap();
            let viewer = world.get::<shared::components::Level>(run.actor).unwrap().0;
            let effective = shared::level_scaling::level_for_viewer(
                native,
                world.get::<shared::level_scaling::LevelScaling>(target),
                viewer,
            );
            json!({"entry": DUMMY_ENTRY, "native_level": native.0, "effective_level": effective,
            "health": world.get::<shared::components::Health>(target),
            "armor": world.get::<crate::level_scaling::CreatureArmor>(target).map(|armor| armor.0)})
        })
        .collect()
}

/// Main uses the global fight deadline; sc_enemy.cpp1053–1101 timed adds
/// project their own expiration. Native training-dummy hit handling stays inert.
pub(super) fn apply_fixed_time_target_health(run: &mut Run, at: Duration) -> Result<(), String> {
    for &target in &run.targets {
        let metadata = *run
            .app
            .world()
            .get::<FixtureTarget>(target)
            .ok_or("native fixture target lifetime missing")?;
        if metadata.lifetime.is_zero() {
            return Err("native fixture target has zero lifetime".into());
        }
        let remaining = metadata.remaining(at).as_secs_f64() / metadata.lifetime.as_secs_f64();
        let mut health = run
            .app
            .world_mut()
            .get_mut::<shared::components::Health>(target)
            .ok_or_else(|| {
                format!("SIMC_FIXED_TIME_TARGET_HEALTH target{target:?} lacks native Health")
            })?;
        health.current = (f64::from(health.max) * remaining) as f32;
    }
    Ok(())
}

pub(crate) fn next_fixture_boundary(run: &Run, frame_end: Duration) -> Duration {
    let Some(adds) = run.app.world().get::<FixtureAdds>(run.actor) else {
        return frame_end;
    };
    let boundary = adds
        .expires_at
        .map_or(adds.next_spawn_at, |at| at.min(adds.next_spawn_at));
    frame_end.min(boundary)
}

pub(super) fn advance_fixture_step(run: &mut Run, boundary: Duration) -> Result<(), String> {
    let now = run.app.world().resource::<Time>().elapsed();
    let delta = boundary
        .checked_sub(now)
        .filter(|delta| !delta.is_zero())
        .ok_or("native fixture clock did not advance")?;
    apply_fixed_time_target_health(run, boundary)?;
    run.app
        .insert_resource(bevy::time::TimeUpdateStrategy::ManualDuration(delta));
    run.app.update();
    if run.app.world().resource::<Time>().elapsed() != boundary {
        return Err("native fixture clock differs from requested boundary".into());
    }
    Ok(())
}

pub(crate) fn apply_fixture_add_events(
    run: &mut Run,
    retired: &mut Vec<Entity>,
    started_at: Duration,
) -> Result<(), String> {
    let Some(adds) = run.app.world().get::<FixtureAdds>(run.actor).copied() else {
        return Ok(());
    };
    let now = run.app.world().resource::<Time>().elapsed();
    if adds.expires_at.is_some_and(|at| at <= now) {
        *retired = retire_add_wave(run);
    }
    if adds.next_spawn_at <= now {
        spawn_add_wave(run, retired, started_at)?;
    }
    Ok(())
}

fn advance_fixture_frame(
    run: &mut Run,
    frame_end: Duration,
    started_at: Duration,
    retired: &mut Vec<Entity>,
    observe: &mut impl FnMut(&Run),
) -> Result<(), String> {
    while run.app.world().resource::<Time>().elapsed() < frame_end {
        let boundary = next_fixture_boundary(run, frame_end);
        advance_fixture_step(run, boundary)?;
        apply_fixture_add_events(run, retired, started_at)?;
        if run.app.world().resource::<Time>().elapsed() < started_at + FIGHT_DURATION {
            drive_player(run)?;
        }
        observe(run);
    }
    Ok(())
}

pub(crate) fn capture_engagement(run: &Run) -> Value {
    let mut sample = report::engagement(run.app.world(), run.actor, &run.targets);
    // The last add retirement is270.09s, inside the final30s qualification window.
    // Keep live target inspection, but never subtract a retired wave's earned damage.
    sample["damage_to_targets"] = json!(report::total_damage(run.app.world(), &run.damage_targets));
    sample
}

fn advance_combat(run: &mut Run) -> Result<(f64, BTreeMap<u32, f64>, Vec<Value>), String> {
    advance_combat_observed(run, |_| {})
}

fn advance_combat_observed(
    run: &mut Run,
    mut observe: impl FnMut(&Run),
) -> Result<(f64, BTreeMap<u32, f64>, Vec<Value>), String> {
    let start_time = run
        .app
        .world()
        .resource::<Time<Virtual>>()
        .elapsed_secs_f64();
    let started_at = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(run, started_at);
    if run
        .app
        .world()
        .get::<shared::components::Player>(run.actor)
        .is_some()
        && run
            .app
            .world()
            .get::<literal_rotation::LiteralParityFixture>(run.actor)
            .is_none()
    {
        run.player_rotation = Some(player_rotation::PlayerRotation::load(
            run.profile.spec_id,
            start_time,
        )?);
    }
    let mut buffs = BTreeMap::new();
    let mut samples = vec![capture_engagement(run)];
    observe(run);
    drive_player(run)?;
    let frame = FIGHT_DURATION / TICKS;
    let mut retired = Vec::new();
    for tick in 1..=TICKS {
        advance_fixture_frame(
            run,
            started_at + frame * tick,
            started_at,
            &mut retired,
            &mut observe,
        )?;
        report::sample_buffs(run.app.world(), run.actor, &mut buffs);
        if matches!(tick, 1 | 20 | LAST_30S_START | TICKS) {
            samples.push(capture_engagement(run));
        }
    }
    let seconds = run
        .app
        .world()
        .resource::<Time<Virtual>>()
        .elapsed_secs_f64()
        - start_time;
    let buff_uptimes = buffs
        .into_iter()
        .map(|(spell, ticks)| (spell, f64::from(ticks) * 100.0 / f64::from(TICKS)))
        .collect();
    Ok((seconds, buff_uptimes, samples))
}

use literal_rotation::LiteralTickProgress;

pub(crate) fn drive_literal_tick(run: &mut Run) -> Result<(), String> {
    let driver = run.spec_driver.as_mut().expect("literal driver is present");
    let (actor, targets) = (run.actor, &run.targets);
    literal_rotation::drive_tick(run.app.world_mut(), actor, |world| {
        driver.drive(world, actor, targets)
    })
}

pub(crate) fn drive_literal_off_gcd(run: &mut Run) -> Result<bool, String> {
    let world = run.app.world_mut();
    world.init_resource::<LiteralTickProgress>();
    world.resource_mut::<LiteralTickProgress>().0 = false;
    run.spec_driver
        .as_mut()
        .expect("literal driver is present")
        .drive(world, run.actor, &run.targets)?;
    Ok(world.resource::<LiteralTickProgress>().0)
}

fn drive_player(run: &mut Run) -> Result<(), String> {
    if run.spec_driver.is_some() {
        drive_literal_tick(run)?;
    }
    if let Some(rotation) = &mut run.player_rotation {
        rotation.drive(run.app.world_mut(), run.actor, run.targets[0])?;
    }
    Ok(())
}

fn driver_report(run: &Run) -> serde_json::Map<String, Value> {
    let mut output = serde_json::Map::new();
    for registration in DRIVERS {
        if let Some(key) = registration.report_key {
            let report = run
                .spec_driver
                .as_ref()
                .filter(|_| registration.spec_id == run.profile.spec_id)
                .map_or(Value::Null, |driver| driver.report());
            output.insert(key.into(), report);
        }
    }
    let label = if let Some(driver) = &run.spec_driver {
        driver.label()
    } else if run.player_rotation.is_some() {
        "NATIVE_ASSISTED_COMBAT_70448"
    } else {
        "NATIVE_FOLLOWER_KIT"
    };
    output.insert("driver".into(), json!(label));
    output
}

fn measure(spec_id: u32, count: usize) -> Result<Value, String> {
    let mut reports = Vec::new();
    for seed in multi_seed::read_seeds()? {
        let report = measure_seed(spec_id, count, seed)?;
        multi_seed::write_seed_report(spec_id, count, seed, &report)?;
        eprintln!(
            "NATIVE_SEED_DPS spec{spec_id} targets{count} seed{seed}: {}",
            report["combat"]["dps"]
        );
        reports.push(report);
    }
    Ok(multi_seed::summarize(reports))
}

pub(crate) fn combat_report(run: &Run, seconds: f64) -> Value {
    report::combat(run.app.world(), &run.damage_targets, seconds)
}

fn measure_seed(spec_id: u32, count: usize, seed: u64) -> Result<Value, String> {
    if let Some(directory) = std::env::var_os("DPS_PARITY_TANK_TRACE_DIR") {
        let path =
            std::path::Path::new(&directory).join(format!("{spec_id}.{count}.{seed}.simc.log"));
        let started = Instant::now();
        let mut replay = super::tank_replay::replay_simc_with_targets(
            spec_id, &path, count, 300, seed, true, 1.0,
        )?;
        if !replay["time_to_death"].is_null() {
            return Err(format!(
                "tank died during DPS measurement: {}",
                replay["time_to_death"]
            ));
        }
        replay["spec_id"] = json!(spec_id);
        replay["target_count"] = json!(count);
        replay["simulated_seconds"] = json!(replay["duration_seconds"]);
        replay["seed"] = json!(seed);
        replay["wall_seconds"] = json!(started.elapsed().as_secs_f64());
        return Ok(replay);
    }
    let _seed_scope = crate::random::seed_scope(seed);
    let mut run = prepare_measurement(spec_id, count)?;
    if run.spec_driver.is_none() {
        return Err(format!("NATIVE_LITERAL_DRIVER_UNAVAILABLE:{spec_id}"));
    }
    let before = report::stats(run.app.world(), run.actor);
    let targets = target_snapshot(&run);
    engage(&mut run);
    let started = Instant::now();
    let (seconds, buff_uptimes, engagement_samples) = advance_combat(&mut run)?;
    let combat = report::combat(run.app.world(), &run.damage_targets, seconds);
    let mut output = json!({"spec_id": spec_id, "target_count": count, "simulated_seconds": seconds,
        "actor_kind": if run.app.world().get::<shared::components::Player>(run.actor).is_some() { "player" } else { "follower" },
        "assisted_decisions": run.player_rotation.as_ref().map_or(0, |rotation| rotation.decisions),
        "wall_seconds": started.elapsed().as_secs_f64(), "seed": seed, "combat": combat,
        "target_health_policy": {"name": "SIMC_FIXED_TIME_TARGET_HEALTH", "clock": "per_target_lifetime",
            "duration_seconds": FIGHT_DURATION.as_secs_f64(), "add_lifetime_seconds": ADD_LIFETIME.as_secs_f64(),
            "add_interval_seconds": ADD_INTERVAL.as_secs_f64(), "initial_pct": 100, "death_pct": 0},
        "stats_before": before, "stats_after": report::stats(run.app.world(), run.actor),
        "targets_before": targets, "targets_after": target_snapshot(&run),
        "target_waves": run.target_waves, "pet": report::pet(run.app.world(), run.actor, &run.damage_targets),
        "buff_uptime_pct": buff_uptimes, "engagement_samples": engagement_samples, "gaps": run.gaps,
        "reference_race": run.profile.race, "server_build": 70448, "simc_build": 70391});
    output.as_object_mut().unwrap().extend(driver_report(&run));
    Ok(output)
}

#[test]
#[ignore = "requires private spec253 exact profile and selected traits"]
fn beast_mastery_literal_fixture_casts_reference_actions() {
    let mut run = prepare_measurement(253, 1).expect("native BM fixture");
    assert!(
        run.app
            .world()
            .get::<literal_rotation::LiteralParityFixture>(run.actor)
            .is_some(),
        "BM must suppress follower kit decisions before setup and execute the literal source"
    );
    engage(&mut run);
    for _ in 0..200 {
        run.app.update();
        drive_player(&mut run).expect("literal BM decision");
    }
    let combat = report::combat(run.app.world(), &run.damage_targets, 10.0);
    let actions = combat["actions"].as_array().unwrap();
    for spell in [75, 217200] {
        assert!(
            actions.iter().any(|action| action["spell_id"] == spell),
            "literal Auto Shot/Barbed Shot must execute: {spell}; {combat}"
        );
    }
}

#[test]
#[ignore = "requires native Demonology profile and player actor"]
fn dps_measurement_rejects_demonology_player_without_literal_driver() {
    let error = measure_seed(266, 1, 70448).unwrap_err();
    assert_eq!(error, "NATIVE_LITERAL_DRIVER_UNAVAILABLE:266");
}

#[test]
#[ignore = "requires private spec254 exact profile and selected traits"]
fn marksmanship_literal_fixture_casts_reference_actions() {
    let report = measure(254, 1).expect("native Marksmanship fixture");
    assert_eq!(report["driver"], "LITERAL_SIMC_MARKSMANSHIP_76f8118");
    let actions = report["combat"]["actions"].as_array().unwrap();
    for spell in [75, 19434, 257044] {
        assert!(
            actions.iter().any(|action| action["spell_id"] == spell),
            "literal Auto Shot/Aimed Shot/Rapid Fire must actually execute: {spell}"
        );
    }
}

fn measure_tank_replays(spec: u32, seeds: &str) -> Vec<serde_json::Value> {
    let directory = std::path::Path::new(
        "/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/healer-tank-replay",
    );
    let trace = directory.join(format!("{spec}.simc-trace.log"));
    seeds
        .split(',')
        .map(|seed| {
            let seed: u64 = seed.parse().expect("numeric tank replay seed");
            let replay =
                super::tank_replay::replay_simc_with_healing(spec, &trace, 300, seed, true)
                    .expect("tank replay measurement");
            validate_tank_replay(&replay);
            json!({"seed":seed,"report":replay})
        })
        .collect()
}

fn validate_tank_replay(replay: &Value) {
    assert_eq!(replay["tank_dummy_scenario"]["attacker_level"], 92);
    assert!(
        (replay["tank_dummy_scenario"]["armor_constant"]
            .as_f64()
            .unwrap()
            - 4531.03)
            .abs()
            < 0.01
    );
    assert!(
        replay["incoming_before_absorbs"]
            .as_object()
            .unwrap()
            .values()
            .map(|amount| amount.as_f64().unwrap())
            .sum::<f64>()
            > 0.0
    );
}

#[test]
fn tank_measurement_accepts_fully_absorbed_incoming_damage() {
    let replay = json!({"tank_dummy_scenario": {"attacker_level": 92, "armor_constant": 4531.03}, "incoming_before_absorbs": {"melee": 1200}, "damage_taken": 0});
    validate_tank_replay(&replay);
}

#[test]
#[ignore = "requires Guardian profile and replay trace"]
fn guardian_tank_measurement_runs_each_requested_seed() {
    let reports = measure_tank_replays(104, "70448,1");
    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0]["seed"], 70448);
    assert_eq!(reports[1]["seed"], 1);
    assert_ne!(
        reports[0]["report"]["damage_taken"],
        reports[1]["report"]["damage_taken"]
    );
}

#[test]
#[ignore = "opt-in real profile gear, allocated traits and300s native combat; requires private DB/evidence path"]
fn dps_parity_native_bis_dummy_records_300s() {
    let spec: u32 = std::env::var("DPS_PARITY_SPEC")
        .expect("DPS_PARITY_SPEC")
        .parse()
        .expect("numeric specialization");
    let count: usize = std::env::var("DPS_PARITY_TARGETS")
        .expect("DPS_PARITY_TARGETS")
        .parse()
        .expect("numeric target count");
    assert!(matches!(count, 1 | 3));
    if count == 1
        && matches!(spec, 270 | 1468)
        && std::env::var_os("HEALER_IMPACT_REPLAY").is_some()
    {
        let directory = std::path::Path::new(
            "/home/osso/Projects/world-of-osso/game-engine/data/diagnostics/healer-tank-replay",
        );
        let report = super::healer_replay::replay_simc_party(
            spec,
            &directory.join("66.simc-trace.log"),
            &directory.join("66.simc-party-pulse-seed70448.debug.log"),
            300,
            70448,
        )
        .expect("native healer impact replay");
        let file = inputs::directory()
            .unwrap()
            .join(format!("{spec}.native.{count}.json"));
        std::fs::write(
            file,
            serde_json::to_vec_pretty(&json!({"spec_id":spec,"healer_replay":report})).unwrap(),
        )
        .unwrap();
        eprintln!("NATIVE_HEALER_REPLAY spec{spec} seed70448: complete");
        return;
    }
    let result = measure(spec, count);
    let mut output = match &result {
        Ok(report) => report.clone(),
        Err(error) => json!({"spec_id": spec, "target_count": count, "measurement_error": error}),
    };
    if result.is_ok()
        && count == 1
        && matches!(spec, 66 | 73 | 104 | 250 | 268 | 581)
        && std::env::var_os("HEALER_TANK_MEASURE_REPLAYS").is_some()
    {
        let seeds = std::env::var("HEALER_TANK_SEEDS").expect("HEALER_TANK_SEEDS");
        output["tank_replays"] = json!(measure_tank_replays(spec, &seeds));
    }
    let file = inputs::directory()
        .unwrap()
        .join(format!("{spec}.native.{count}.json"));
    std::fs::write(&file, serde_json::to_vec_pretty(&output).unwrap()).unwrap();
    let report = result.unwrap_or_else(|error| panic!("native spec{spec}: {error}"));
    assert!((report["simulated_seconds"].as_f64().unwrap() - 300.0).abs() < 0.001);
    assert!(
        report["combat"]["total_damage"].as_f64().unwrap() > 0.0,
        "actual production kit must deal damage; full report {}",
        file.display()
    );
    if count == 3 {
        let waves = report["target_waves"].as_array().unwrap();
        assert_eq!(
            waves.len(),
            10,
            "two adds must retire and respawn every30s across300s"
        );
        let main = waves[0]["entities"][0].as_u64().unwrap();
        let mut adds = std::collections::BTreeSet::new();
        for (index, wave) in waves.iter().enumerate() {
            assert!(
                (wave["at"].as_f64().unwrap() - index as f64 * ADD_INTERVAL.as_secs_f64()).abs()
                    < 0.0001
            );
            if index > 0 {
                assert_eq!(
                    wave["retired_still_present"], 0,
                    "old wave must actually despawn"
                );
            }
            assert_eq!(
                wave["entities"][0].as_u64().unwrap(),
                main,
                "main dummy must persist"
            );
            for add in wave["entities"].as_array().unwrap().iter().skip(1) {
                assert!(
                    adds.insert(add.as_u64().unwrap()),
                    "retired add identity must not be reused"
                );
            }
        }
        assert_eq!(adds.len(), 20);
    }
    if matches!(spec, 253 | 266) {
        assert_eq!(
            report["pet"]["present"], true,
            "native pet-class execution requires its authored pet"
        );
        assert!(
            !report["pet"]["casts"].as_array().unwrap().is_empty(),
            "real pet must execute its learned casts"
        );
        assert!(
            report["pet"]["damage"].as_i64().unwrap() > 0,
            "real pet must damage the dummy"
        );
    }
    let samples = report["engagement_samples"].as_array().unwrap();
    if report["actor_kind"] == "player" {
        assert_eq!(samples[0]["actor_is_player"], true);
        assert!(
            samples[0]["owner"].is_null(),
            "player has no synthetic controller"
        );
    } else {
        assert_eq!(
            samples[0]["actor_is_player"], false,
            "measured follower stays an NPC"
        );
        assert_eq!(
            samples[0]["owner_is_player"], true,
            "owner uses native player initialization"
        );
        let owner = samples[0]["owner"]["entity"].as_u64().unwrap();
        let owner_damage: f64 = report["combat"]["actions"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|action| action["actor"].as_u64() == Some(owner))
            .map(|action| action["damage"].as_f64().unwrap())
            .sum();
        assert_eq!(
            owner_damage, 0.0,
            "out-of-range controller must not contribute damage"
        );
    }
    let end_damage = samples.last().unwrap()["damage_to_targets"]
        .as_i64()
        .unwrap();
    let last_30s_damage = end_damage
        - samples[samples.len() - 2]["damage_to_targets"]
            .as_i64()
            .unwrap();
    assert!(
        last_30s_damage > 0,
        "fixture must sustain actual damage in the final30s, not merely land first-tick hits"
    );
    eprintln!(
        "NATIVE_DPS spec{spec} targets{count}: {}",
        report["combat"]["dps"]
    );
}

#[test]
#[ignore = "uses the real Vengeance three-target fixture and literal APL"]
fn vengeance_three_target_add_event_has_no_boss() {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    use server::simc_apl::Facts;
    let _seed = crate::random::seed_scope(70448);
    let mut run = prepare_registered_player(581, 3).unwrap();
    engage(&mut run);
    let start = run.app.world().resource::<Time>().elapsed();
    install_fixture_lifetimes(&mut run, start);
    let bindings = Bindings::default();
    let encounter = Encounter::Fixture {
        started_at: start.as_secs_f64(),
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let facts = NativeFacts::new(run.app.world(), run.actor, &bindings, encounter, 3);
    assert_eq!(facts.value("raid_event.adds.exists").unwrap(), 1.0);
    assert_eq!(facts.value("raid_event.adds.count").unwrap(), 2.0);
    assert_eq!(facts.value("raid_event.adds.has_boss").unwrap(), 0.0);
    drive_literal_tick(&mut run).expect("Vengeance APL must accept configured ordinary adds");
}

#[test]
#[ignore = "opt-in actual300s per-wave health/TTD; requires private parity inputs"]
fn dps_parity_fixture_add_health_and_ttd_follow_each_summon_lifetime() {
    use crate::simc_apl_native::{Bindings, Encounter, FightStyle, NativeFacts};
    use server::simc_apl::{Action, ActionFacts, Facts};
    let _seed_scope = crate::random::seed_scope(70_448);
    let (profile, traits) = read_reference(66).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        3,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    engage(&mut run);
    let started_at = crate::spell_cast::now(run.app.world());
    let encounter = Encounter::Fixture {
        started_at,
        duration: 300.0,
        style: FightStyle::Patchwerk,
    };
    let bindings = Bindings::default();
    let mut seen = [[false; 2]; 10];
    let mut gaps = [false; 9];
    let mut errors = Vec::new();
    let (seconds, _, _) = advance_combat_observed(&mut run, |run| {
        let elapsed = crate::spell_cast::now(run.app.world()) - started_at;
        let millis = (elapsed * 1000.0).round() as u32;
        let wave = (millis / 30_010) as usize;
        if wave >= seen.len() {
            return;
        }
        let age = millis % 30_010;
        let facts = NativeFacts::new(
            run.app.world(),
            run.actor,
            &bindings,
            encounter,
            run.targets.len(),
        );
        let remaining = f64::from(30_000_u32.saturating_sub(age)) / 1000.0;
        let next_spawn = f64::from((wave as u32 + 1) * 30_010 - millis) / 1000.0;
        // Observe actual installed event state at every native boundary/frame,
        // including each real absent gap; never infer event presence from count.
        for (property, expected) in [
            ("exists", 1.0),
            ("count", 2.0),
            ("duration", 30.0),
            ("cooldown", 30.01),
            ("remains", remaining),
            ("in", next_spawn),
        ] {
            match facts.value(&format!("raid_event.adds.{property}")) {
                Ok(value) if (value - expected).abs() < 0.001 => {}
                other => errors.push(format!(
                    "{elapsed}s adds.{property}: {other:?}, expected{expected}"
                )),
            }
        }
        if age >= 30_000 {
            gaps[wave] = true;
            if run.targets.len() != 1 {
                errors.push(format!(
                    "{elapsed}s: expected absent adds, got{} targets",
                    run.targets.len()
                ));
            }
            return;
        }
        let sample_index = if age >= 27_000 {
            1 //10% health: both Execute thresholds have been crossed in this wave.
        } else if age >= 15_000 {
            0 //50% health: same wave before Execute eligibility.
        } else {
            return;
        };
        if seen[wave][sample_index] {
            return;
        }
        seen[wave][sample_index] = true;
        if run.targets.len() != 3 {
            errors.push(format!(
                "wave{wave}: expected3 targets, got{}",
                run.targets.len()
            ));
        }
        for (index, &target) in run.targets.iter().enumerate() {
            // sc_enemy.cpp1053–1101: timed adds use their own expiration, not300s.
            let expected_ttd = if index == 0 {
                300.0 - elapsed
            } else {
                f64::from(30_000 - age) / 1000.0
            };
            let lifetime = if index == 0 { 300.0 } else { 30.0 };
            let expected_pct = expected_ttd / lifetime * 100.0;
            let health = run
                .app
                .world()
                .get::<shared::components::Health>(target)
                .unwrap();
            let native_pct = f64::from(health.current) * 100.0 / f64::from(health.max);
            let action = Action {
                source_row: None,
                name: "auto_attack".into(),
                target: Some(target.to_bits()),
                options: Default::default(),
            };
            let readings = [
                ("native health", Ok(native_pct), expected_pct),
                (
                    "target.health.pct",
                    facts.value_for(&action, "target.health.pct"),
                    expected_pct,
                ),
                (
                    "target.time_to_die",
                    facts.value_for(&action, "target.time_to_die"),
                    expected_ttd,
                ),
                // player.hpp964 and raid_event.cpp77: main boss, timed ENEMY_ADDs not bosses.
                (
                    "target.is_boss",
                    facts.value_for(&action, "target.is_boss"),
                    if index == 0 { 1.0 } else { 0.0 },
                ),
            ];
            for (label, actual, expected) in readings {
                match actual {
                    Ok(value) if (value - expected).abs() < 0.001 => {}
                    other => errors.push(format!(
                        "wave{wave}/sample{sample_index}/target{index} {label}: {other:?}, expected{expected}"
                    )),
                }
            }
        }
    })
    .unwrap();
    assert!((seconds - 300.0).abs() < 0.001);
    assert!(
        seen.into_iter().flatten().all(|seen| seen),
        "all10 waves must be observed before and during their Execute phase"
    );
    assert!(errors.is_empty(), "{}", errors.join("\n"));
    assert!(
        gaps.into_iter().all(|seen| seen),
        "all9 actual10ms absent windows must be observed"
    );
}

#[test]
#[ignore = "opt-in actual300s native add lifetimes; requires private parity inputs"]
fn dps_parity_fixture_add_start_schedule_matches_simc_10ms_cooldown_clamp() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let (profile, traits) = read_reference(66).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        3,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    engage(&mut run);
    let (seconds, _, _) = advance_combat(&mut run).unwrap();
    assert!((seconds - 300.0).abs() < 0.001);
    assert_eq!(run.target_waves.len(), 10);
    // Pinned raid_event.cpp2700–2701: cooldown<=duration becomes duration+0.01s.
    for (index, wave) in run.target_waves.iter().enumerate() {
        let expected = index as f64 * 30.01;
        let actual = wave["at"].as_f64().unwrap();
        assert!(
            (actual - expected).abs() < 0.0001,
            "wave{index} started at{actual}s, source requires{expected}s"
        );
    }
}

#[test]
#[ignore = "opt-in real300s dummy clock and add replacement; requires private parity inputs"]
fn dps_parity_fixture_fixed_time_main_health_follows_global_clock() {
    let _seed_scope = crate::random::seed_scope(70_448);
    let (profile, traits) = read_reference(66).unwrap();
    let mut run = prepare_reference(
        profile,
        &traits,
        3,
        crate::followers::simulator::ParityActor::Follower,
    )
    .unwrap();
    engage(&mut run);
    let (seconds, _, samples) = advance_combat(&mut run).unwrap();
    assert!((seconds - 300.0).abs() < 0.001);
    let expected = [
        100.0,
        100.0 * 5999.0 / 6000.0,
        100.0 * 5980.0 / 6000.0,
        10.0,
        0.0,
    ];
    assert_eq!(samples.len(), expected.len());
    for (sample, expected_pct) in samples.iter().zip(expected) {
        // Adds' independent phases are checked twice in every wave above.
        for target in sample["targets"].as_array().unwrap().iter().take(1) {
            let health = target["state"]["health"].as_array().unwrap();
            let observed = health[0].as_f64().unwrap() * 100.0 / health[1].as_f64().unwrap();
            assert!(
                (observed - expected_pct).abs() < 0.001,
                "SIMC_FIXED_TIME_TARGET_HEALTH expected{expected_pct}% but observed{observed}%: {target}"
            );
        }
    }
    assert_eq!(run.target_waves.len(), 10);
}

#[test]
#[ignore = "opt-in exact-BiS player authored decision or explicit source gap; requires private parity inputs"]
fn dps_parity_fixture_player_uses_authored_decision_or_records_rule_gap() {
    match measure(71, 1) {
        Ok(report) => {
            assert_eq!(report["actor_kind"], "player");
            assert_eq!(report["driver"], "NATIVE_ASSISTED_COMBAT_70448");
            assert!(report["assisted_decisions"].as_u64().unwrap() > 0);
        }
        Err(error) => {
            assert!(error.starts_with("NATIVE_ASSISTED_COMBAT_GAP:"), "{error}");
            let lowercase = error.to_lowercase();
            assert!(
                lowercase.contains("rule") && lowercase.contains("unverified"),
                "{error}"
            );
            eprintln!("AUTHORED_PLAYER_RULE_GAP: {error}");
        }
    }
}

#[test]
#[ignore = "opt-in native Arms71 player with pinned level90 gear/traits; requires private parity inputs"]
fn dps_parity_native_player_profile_spawns_with_pinned_gear() {
    let run = prepare(71, 1).expect("pinned native Arms71 player profile must spawn");
    let world = run.app.world();
    let player = world
        .get::<shared::components::Player>(run.actor)
        .expect("measured actor must be a real player");
    assert_eq!(player.class, 1);
    assert!(world.get::<shared::components::Npc>(run.actor).is_none());
    assert!(world.get::<crate::followers::Follower>(run.actor).is_none());
    assert_eq!(
        world.get::<shared::components::Level>(run.actor).unwrap().0,
        90
    );
    let expected_items: Vec<_> = run
        .profile
        .equipment
        .iter()
        .map(|item| {
            (
                inputs::slot(&item.slot).unwrap(),
                shared::item_data::ItemDefinitionSource::Retail,
                item.item_id,
            )
        })
        .collect();
    assert_eq!(
        world
            .get::<crate::stats::EquippedItems>(run.actor)
            .unwrap()
            .items,
        expected_items
    );
    let expected_modifiers: Vec<_> = run
        .profile
        .equipment
        .iter()
        .map(|item| (inputs::slot(&item.slot).unwrap(), item.modifiers()))
        .collect();
    assert_eq!(
        world
            .get::<crate::stats::EquippedItemModifiers>(run.actor)
            .unwrap()
            .items,
        expected_modifiers
    );
    let stats = world
        .get::<crate::stats::CharacterStats>(run.actor)
        .unwrap();
    assert!(
        stats.main_hand.min > 1.0,
        "native BiS weapon stats must be applied"
    );
    assert!(stats.primary.strength > 0.0);
}
