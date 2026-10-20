//! Polymorph 118 on real world.db data: a level-10 mage's cast turns an attacking
//! Kobold Vermin into the Polymorphed Sheep (template 16372), which stops swinging and
//! wanders; Frostbolt damage past 10% of its health breaks it, expiry ends it, and
//! both restore the kobold's display. Skipped when data/world.db is absent.

use rusqlite::{Connection, OpenFlags};
use shared::casting::CastState;
use shared::components::{
    CreatureMotion, Health, ModelDisplay, Npc, Position, UnitLevel, UnitTarget,
};
use shared::protocol::CastFailed;
use shared::spell_data::CastFailReason;

use super::*;
use crate::creature_ai::{CreatureState, SpawnPoint};
use crate::spell_cast::test_harness::{Harness, harness_with_plugins};
use crate::world_data::WorldData;

const POLYMORPH: u32 = 118;
const FROSTBOLT: u32 = 116;
/// Frostbolt's TRIGGER_MISSILE spell: 2.448 × SP school damage, `SPELL_ATTR3_NOT_A_PROC`.
const FROSTBOLT_DAMAGE: u32 = 228597;
/// Chilled, cast on each Frostbolt hit by `spell_mage_frostbolt`: MOD_DECREASE_SPEED 50% for 8 s.
const CHILLED: u32 = 205708;
/// Native50240: −200 attacker melee hit chance for30s, no power cost.
const EVASIVE_MANEUVERS: u32 = 50240;
const KOBOLD_VERMIN: u32 = 6;
/// Kobold Vermin's only `creature_template_model` display.
const KOBOLD_DISPLAY: u32 = 10913;
/// Polymorph effect 1 TRANSFORM MiscValue: Polymorphed Sheep, displays 856 and 857.
const SHEEP_DISPLAYS: [u32; 2] = [856, 857];
const TRAINING_DUMMY: u32 = 44548;
/// Northshire Blackrock Spy: ContentTuning 73 (levels 1-30), humanoid.
const BLACKROCK_SPY: u32 = 49874;
/// Blackrock Spy `faction`: FactionTemplate 7, neutral to players.
const SPY_FACTION: u32 = 7;
/// Polymorph `SpellCastTimes` 90 (1.7 s) and `SpellDuration` 3 (60 s), in 50 ms frames.
const POLYMORPH_CAST_FRAMES: usize = 1700 / 50;
const POLYMORPH_DURATION_FRAMES: usize = 60_000 / 50;
const MAGE: u8 = 8;

/// Templates of the kobold, the sheep and the training dummy, with their models.
fn world_data() -> WorldData {
    let path = crate::test_data::world_db_path();
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let expected = crate::world_data::load_expected_stats(&conn);
    let mut templates = crate::world_data::load_templates(&conn, &expected);
    templates
        .retain(|entry, _| [KOBOLD_VERMIN, 16372, TRAINING_DUMMY, BLACKROCK_SPY].contains(entry));
    crate::world_data::load_template_models(&conn, &mut templates);
    WorldData {
        templates,
        ..Default::default()
    }
}

/// The harness mage (Polymorph, Frostbolt) and its kobold spawned as a world creature:
/// spawn point, level, kobold display, a Reactive threat list, creature AI and aggro.
/// With `max_health` the kobold holds 10% of it against damage.
fn polymorph_harness(max_health: f32) -> Option<Harness> {
    let mut h = harness_with_plugins(
        Vec::new(),
        &[
            POLYMORPH,
            FROSTBOLT,
            FROSTBOLT_DAMAGE,
            CHILLED,
            EVASIVE_MANEUVERS,
        ],
        MAGE,
        false,
        |app| {
            app.insert_resource(world_data());
            app.add_plugins((
                crate::creature_ai::CreatureAiPlugin,
                crate::creature_aggro::CreatureAggroPlugin,
                crate::unit_target::UnitTargetPlugin,
            ));
        },
    )?;
    let position = *h.app.world().get::<Position>(h.npc).unwrap();
    let mut npc = h.app.world_mut().entity_mut(h.npc);
    npc.insert((
        Health {
            current: max_health,
            max: max_health,
        },
        UnitLevel(5),
        ModelDisplay {
            display_id: KOBOLD_DISPLAY,
        },
        CreatureMotion::Still,
        SpawnPoint {
            x: position.x,
            y: position.y,
            z: position.z,
            wander_distance: 0.0,
            speed_walk: 1.0,
            speed_run: 1.14286,
        },
        crate::spell_cast::powers::CreatureHealthRegenTimer::default(),
    ));
    let mut commands = h.app.world_mut().commands();
    let mut entity = commands.entity(h.npc);
    crate::creature_aggro::insert_aggro_components(
        &mut entity,
        crate::creature_aggro::AggroMode::Reactive,
    );
    h.app.world_mut().flush();
    h.advance(1);
    Some(h)
}

fn display(h: &Harness) -> u32 {
    h.app.world().get::<ModelDisplay>(h.npc).unwrap().display_id
}

fn polymorphed(h: &Harness) -> bool {
    has_aura(h.app.world(), h.npc, POLYMORPH)
}

/// The kobold attacks the mage (`AttackStart`) until one swing has landed.
fn kobold_attacks(h: &mut Harness) {
    let (npc, caster) = (h.npc, h.caster);
    crate::creature_aggro::attack_start(h.app.world_mut(), npc, caster);
    for _ in 0..100 {
        h.advance(1);
        if h.health(h.caster) < 1000.0 {
            return;
        }
    }
    panic!(
        "the kobold never hit the mage; actual events={:?}; state={:?}; fixed_time={:?}",
        h.inbox.log,
        h.app.world().get::<CreatureState>(h.npc),
        h.app.world().resource::<Time<Fixed>>().elapsed()
    );
}

/// Let Polymorph finish, including up to two 500 ms damage pushbacks.
fn cast_polymorph(h: &mut Harness) {
    h.cast_and_deliver(POLYMORPH, Some(h.npc));
    assert!(
        h.app.world().get::<CastState>(h.caster).is_some(),
        "{:?}",
        h.inbox.failed
    );
    const MAX_PUSHBACK_FRAMES: usize = 1000 / 50;
    for _ in 0..POLYMORPH_CAST_FRAMES + MAX_PUSHBACK_FRAMES + 2 {
        if h.app.world().get::<CastState>(h.caster).is_none() {
            break;
        }
        h.advance(1);
    }
    assert!(polymorphed(h), "{:?}", h.inbox.failed);
}

/// Frostbolt once the GCD is over; the missile lands within 2 s.
fn cast_frostbolt(h: &mut Harness) {
    h.advance(30);
    h.cast_and_deliver(FROSTBOLT, Some(h.npc));
    h.advance(2000 / 50 + 40);
}

#[test]
fn polymorph_turns_the_attacking_kobold_into_a_wandering_sheep_that_stops_swinging() {
    let Some(mut h) = polymorph_harness(5000.0) else {
        return;
    };
    kobold_attacks(&mut h);
    cast_polymorph(&mut h);
    assert!(SHEEP_DISPLAYS.contains(&display(&h)), "{}", display(&h));
    let aura = auras_of_spell(&h, POLYMORPH);
    assert_eq!(aura.duration_ms, 60_000);
    assert!(aura.harmful);
    // No target shown while confused (SetConfused → SetTarget(Empty)).
    assert_eq!(
        h.app.world().get::<UnitTarget>(h.npc),
        Some(&UnitTarget(None))
    );
    // Where its confusion began (it may already have taken a step since).
    let (x, z) = h.app.world().get::<Confused>(h.npc).unwrap().reference;
    let health = h.health(h.caster);
    let mut walked = false;
    for _ in 0..(8000 / 50) {
        h.advance(1);
        let at = h.app.world().get::<Position>(h.npc).unwrap();
        let from_reference = (at.x - x).hypot(at.z - z);
        assert!(
            from_reference <= 2.01,
            "{from_reference} yd from where it was polymorphed"
        );
        walked |= h.app.world().get::<CreatureMotion>(h.npc) == Some(&CreatureMotion::Walk);
    }
    assert!(walked, "the sheep wanders at walk");
    assert_eq!(h.health(h.caster), health, "the sheep swung at the mage");
    // Still engaged with the mage, and still the kobold under the sheep.
    assert_eq!(
        h.app.world().get::<CreatureState>(h.npc),
        Some(&CreatureState::Pursuing {
            target: h.caster.to_bits()
        })
    );
    assert_eq!(
        h.app.world().get::<Npc>(h.npc).unwrap().template_id,
        KOBOLD_VERMIN
    );
}

fn auras_of_spell(h: &Harness, spell_id: u32) -> AuraInstance {
    h.app
        .world()
        .get::<ActiveAuras>(h.npc)
        .and_then(|auras| auras.iter().find(|aura| aura.spell_id == spell_id).cloned())
        .expect("aura on the kobold")
}

#[test]
fn frostbolt_damage_past_a_tenth_of_its_health_breaks_polymorph_and_the_kobold_fights_again() {
    // 10% of 4000 = 400 against Frostbolt's ~245 (2.448 × SP 100): the second one breaks it.
    let Some(mut h) = polymorph_harness(4000.0) else {
        return;
    };
    kobold_attacks(&mut h);
    cast_polymorph(&mut h);
    cast_frostbolt(&mut h);
    // Read the hit from the log: polymorph regeneration heals the kobold meanwhile.
    let hits = frostbolt_hits(&h);
    assert_eq!(hits.len(), 1, "{:?}", h.inbox.failed);
    assert!((230..=260).contains(&hits[0]), "{hits:?}");
    assert!(polymorphed(&h), "one Frostbolt is below the 400 threshold");
    assert!(SHEEP_DISPLAYS.contains(&display(&h)));
    cast_frostbolt(&mut h);
    assert_eq!(frostbolt_hits(&h).len(), 2);
    assert!(!polymorphed(&h), "the second Frostbolt breaks it");
    assert_eq!(display(&h), KOBOLD_DISPLAY);
    assert!(h.app.world().get::<Confused>(h.npc).is_none());
    assert_kobold_attacks_again(&mut h);
    assert_eq!(
        h.app.world().get::<UnitTarget>(h.npc),
        Some(&UnitTarget(Some(h.caster.to_bits())))
    );
}

fn frostbolt_hits(h: &Harness) -> Vec<i32> {
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.kind == shared::protocol::CombatLogKind::Damage
                && event.spell_id == Some(FROSTBOLT_DAMAGE)
        })
        .map(|event| event.amount)
        .collect()
}

#[test]
fn polymorph_expires_after_60_seconds_and_restores_the_kobold() {
    let Some(mut h) = polymorph_harness(5000.0) else {
        return;
    };
    kobold_attacks(&mut h);
    cast_polymorph(&mut h);
    h.advance(POLYMORPH_DURATION_FRAMES - 60);
    assert!(polymorphed(&h));
    assert!(SHEEP_DISPLAYS.contains(&display(&h)));
    h.advance(80);
    assert!(!polymorphed(&h));
    assert_eq!(display(&h), KOBOLD_DISPLAY);
    assert_kobold_attacks_again(&mut h);
}

fn assert_kobold_attacks_again(h: &mut Harness) {
    let first_event = h.inbox.log.len();
    h.advance(100);
    let swings: Vec<_> = h.inbox.log[first_event..]
        .iter()
        .filter(|event| {
            event.source == Some(h.npc.to_bits())
                && event.target == Some(h.caster.to_bits())
                && event.spell_id.is_none()
                && matches!(
                    event.kind,
                    shared::protocol::CombatLogKind::Damage
                        | shared::protocol::CombatLogKind::Miss(_)
                )
        })
        .collect();
    assert!(
        !swings.is_empty(),
        "the kobold must resume an actual melee swing at the mage"
    );
}

#[test]
fn polymorph_expiry_with_native_evasive_maneuvers_still_resumes_swing_attempts() {
    let mut h = polymorph_harness(5000.0).unwrap();
    kobold_attacks(&mut h);
    cast_polymorph(&mut h);
    h.advance(POLYMORPH_DURATION_FRAMES - 60);
    assert!(polymorphed(&h));
    h.advance(80);
    assert!(!polymorphed(&h));
    assert_eq!(display(&h), KOBOLD_DISPLAY);
    h.cast_and_deliver(EVASIVE_MANEUVERS, Some(h.caster));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(has_aura(h.app.world(), h.caster, EVASIVE_MANEUVERS));
    let health = h.health(h.caster);
    let first_event = h.inbox.log.len();
    assert_kobold_attacks_again(&mut h);
    assert_eq!(
        h.health(h.caster),
        health,
        "native guaranteed misses deal no damage"
    );
    assert!(h.inbox.log[first_event..].iter().any(|event| {
        event.source == Some(h.npc.to_bits())
            && event.target == Some(h.caster.to_bits())
            && event.spell_id.is_none()
            && event.kind == shared::protocol::CombatLogKind::Miss(shared::protocol::MissKind::Miss)
    }));
}

/// The user's Retail observation: a sheep regains 5% of its max health every second.
#[test]
fn polymorphed_kobold_regenerates_5_percent_of_its_health_every_second() {
    let Some(mut h) = polymorph_harness(900.0) else {
        return;
    };
    kobold_attacks(&mut h);
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = 100.0;
    // Engaged and not polymorphed: no regeneration (Creature::Update `!IsEngaged()`).
    h.advance(2000 / 50 + 2);
    assert_eq!(h.health(h.npc), 100.0);
    cast_polymorph(&mut h);
    // The first polymorphed tick may come from the 2 s timer already running.
    let mut samples = Vec::new();
    for frame in 0..(6000 / 50) {
        let before = h.health(h.npc);
        h.advance(1);
        if h.health(h.npc) != before {
            samples.push((frame, h.health(h.npc) - before));
        }
    }
    assert!(samples.len() >= 5, "{samples:?}");
    assert!(samples.iter().all(|&(_, gain)| gain == 45.0), "{samples:?}");
    // One second (20 frames) between ticks after the first.
    let gaps: Vec<i32> = samples.windows(2).map(|w| w[1].0 - w[0].0).collect();
    assert!(gaps[1..].iter().all(|&gap| gap == 20), "{samples:?}");
}

#[test]
fn polymorph_cannot_target_a_mechanical_training_dummy() {
    let Some(mut h) = polymorph_harness(5000.0) else {
        return;
    };
    h.app.world_mut().get_mut::<Npc>(h.npc).unwrap().template_id = TRAINING_DUMMY;
    h.cast_and_deliver(POLYMORPH, Some(h.npc));
    assert_eq!(
        h.inbox
            .failed
            .last()
            .map(|failed: &CastFailed| failed.reason),
        Some(CastFailReason::InvalidTarget)
    );
    assert!(!polymorphed(&h));
}

/// The kobold made into the Northshire Blackrock Spy a level-10 mage sees: native level
/// 30 (ContentTuning 73) with its level-30 health, neutral FactionTemplate 7, and the
/// `AggroMode` and detection range its template gives a world spawn (flags_extra 0:
/// Aggressive, 20 yd).
fn spy_harness() -> Option<(Harness, f32)> {
    let mut h = polymorph_harness(1.0)?;
    let template = &h.app.world().resource::<WorldData>().templates[&BLACKROCK_SPY];
    let stats = template
        .tuning
        .as_ref()
        .expect("the spy is tuned")
        .stats
        .clone();
    let (speed_walk, speed_run) = (template.speed_walk, template.speed_run);
    let max_health = stats.max_health(30);
    h.app.world_mut().entity_mut(h.caster).insert(UnitLevel(10));
    let mut spy = h.app.world_mut().entity_mut(h.npc);
    spy.get_mut::<Npc>().unwrap().template_id = BLACKROCK_SPY;
    let mut spawn = spy.get::<SpawnPoint>().unwrap().clone();
    (spawn.speed_walk, spawn.speed_run) = (speed_walk, speed_run);
    spy.insert((
        spawn,
        shared::level_scaling::LevelScaling {
            content_tuning_id: 73,
            min_level: 1,
            max_level: 30,
            delta: 0,
        },
        crate::level_scaling::ScaledStats(stats),
        UnitLevel(30),
        Health {
            current: max_health,
            max: max_health,
        },
        shared::components::UnitFactionTemplate(SPY_FACTION),
    ));
    let mut commands = h.app.world_mut().commands();
    let mut entity = commands.entity(h.npc);
    crate::creature_aggro::insert_aggro_components(
        &mut entity,
        crate::creature_aggro::AggroMode::Aggressive,
    );
    h.app.world_mut().flush();
    Some((h, max_health))
}

#[test]
fn frostbolt_breaks_polymorph_on_a_level_scaled_blackrock_spy() {
    // A level-30 spy scaled to the level-10 mage: Frostbolt's ~245 damage costs it far
    // more of its native health, and 10% of that health is what breaks Polymorph.
    let Some((mut h, max_health)) = spy_harness() else {
        return;
    };
    kobold_attacks(&mut h);
    cast_polymorph(&mut h);
    let hit = frostbolt_hits(&h).len();
    cast_frostbolt(&mut h);
    let hits = frostbolt_hits(&h);
    assert_eq!(hits.len(), hit + 1, "{:?}", h.inbox.failed);
    assert!(
        (hits[hit] as f32) < max_health * 0.1,
        "the dealt damage {} alone is below the threshold {}",
        hits[hit],
        max_health * 0.1
    );
    assert!(!polymorphed(&h), "the spy's health loss breaks it");
    assert_eq!(display(&h), KOBOLD_DISPLAY);
}

#[test]
fn neutral_spy_ignores_a_nearby_mage_then_fights_back_and_keeps_the_frostbolt_damage() {
    let Some((mut h, max_health)) = spy_harness() else {
        return;
    };
    // FactionTemplate 7 is neutral to the Human mage 3 yd away: no aggro on sight.
    h.advance(4000 / 50);
    assert_eq!(
        h.app.world().get::<CreatureState>(h.npc),
        Some(&CreatureState::Idle)
    );
    assert_eq!(h.health(h.caster), 1000.0);
    cast_frostbolt(&mut h);
    assert_eq!(frostbolt_hits(&h).len(), 1, "{:?}", h.inbox.failed);
    assert_eq!(
        h.app.world().get::<CreatureState>(h.npc),
        Some(&CreatureState::Pursuing {
            target: h.caster.to_bits()
        })
    );
    let wounded = h.health(h.npc);
    assert!(wounded < max_health);
    // Engaged and not polymorphed: 3 regeneration intervals pass without healing.
    h.advance(6000 / 50);
    assert_eq!(h.health(h.npc), wounded, "the spy healed while engaged");
    assert!(h.health(h.caster) < 1000.0, "the spy fights back");
}

#[test]
fn evading_spy_keeps_its_damage_until_home_then_regenerates_a_third_every_2_seconds() {
    let Some((mut h, max_health)) = spy_harness() else {
        return;
    };
    kobold_attacks(&mut h);
    assert_spy_evades_and_regenerates(&mut h, max_health);
}

#[test]
#[ignore = "native forced-miss RED before polymorph readiness event oracle"]
fn polymorph_native_forced_miss_engagement_still_evades_and_regenerates() {
    let (mut h, max_health) = spy_harness().unwrap();
    h.cast_and_deliver(EVASIVE_MANEUVERS, Some(h.caster));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(has_aura(h.app.world(), h.caster, EVASIVE_MANEUVERS));
    assert_eq!(
        h.app
            .world()
            .get::<ActiveAuras>(h.caster)
            .unwrap()
            .modifier(184),
        -200.0
    );
    kobold_attacks(&mut h);
    assert_eq!(
        h.health(h.caster),
        1000.0,
        "native misses do not reduce mage health"
    );
    assert!(h.inbox.log.iter().any(|event| {
        event.source == Some(h.npc.to_bits())
            && event.target == Some(h.caster.to_bits())
            && event.spell_id.is_none()
            && event.kind == shared::protocol::CombatLogKind::Miss(shared::protocol::MissKind::Miss)
    }));
    assert_spy_evades_and_regenerates(&mut h, max_health);
}

fn assert_spy_evades_and_regenerates(h: &mut Harness, max_health: f32) {
    // Its home is 10 yd away: 1.7 s of running back at 6 yd/s.
    h.app.world_mut().get_mut::<SpawnPoint>(h.npc).unwrap().x += 10.0;
    let wounded = (max_health / 10.0).floor();
    h.app.world_mut().get_mut::<Health>(h.npc).unwrap().current = wounded;
    // The mage dies: no valid target is left and the spy evades.
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 0.0;
    h.advance(1);
    assert_eq!(
        h.app.world().get::<CreatureState>(h.npc),
        Some(&CreatureState::Returning)
    );
    assert_eq!(h.health(h.npc), wounded, "evading does not restore health");
    let mut healed = Vec::new();
    let mut previous = wounded;
    for _ in 0..(8000 / 50) {
        h.advance(1);
        let health = h.health(h.npc);
        if health != previous {
            let evading =
                h.app.world().get::<CreatureState>(h.npc) == Some(&CreatureState::Returning);
            assert!(!evading, "healed while evading");
            healed.push(health - previous);
            previous = health;
        }
    }
    let third = (max_health / 3.0).floor();
    assert_eq!(healed.len(), 3, "{healed:?}");
    assert_eq!(&healed[..2], &[third, third]);
    assert_eq!(h.health(h.npc), max_health);
}

fn spy_position(h: &Harness) -> Position {
    *h.app.world().get::<Position>(h.npc).unwrap()
}

/// Ground distance the spy covers in each of `frames` 50 ms frames.
fn spy_steps(h: &mut Harness, frames: usize) -> Vec<f32> {
    (0..frames)
        .map(|_| {
            let from = spy_position(h);
            h.advance(1);
            let to = spy_position(h);
            (to.x - from.x).hypot(to.z - from.z)
        })
        .collect()
}

#[test]
fn spy_chases_at_its_run_speed_and_wanders_polymorphed_at_its_walk_speed() {
    // Blackrock Spy speed_run 0.857143 × 7 yd/s (baseMoveSpeed[MOVE_RUN]) = 6 yd/s;
    // ConfusedMovementGenerator walks it (SetWalk): speed_walk 1 × 2.5 yd/s.
    let Some((mut h, _)) = spy_harness() else {
        return;
    };
    kobold_attacks(&mut h);
    h.app.world_mut().get_mut::<Position>(h.caster).unwrap().x = -25.0;
    h.advance(1);
    let chase: f32 = spy_steps(&mut h, 20).iter().sum();
    assert!((chase - 6.0).abs() < 0.05, "chased {chase} yd in 1 s");
    h.app.world_mut().get_mut::<Position>(h.caster).unwrap().x = spy_position(&h).x - 3.0;
    h.advance(2000 / 50);
    cast_polymorph(&mut h);
    let steps = spy_steps(&mut h, 10_000 / 50);
    let walking: Vec<f32> = steps.into_iter().filter(|&step| step > 0.0).collect();
    assert!(walking.len() > 10, "the sheep barely moved: {walking:?}");
    let full = walking
        .iter()
        .filter(|&&step| (step - 0.125).abs() < 1e-3)
        .count();
    assert!(
        walking.iter().all(|&step| step <= 0.125 + 1e-3),
        "faster than 2.5 yd/s: {walking:?}"
    );
    assert!(
        full * 2 > walking.len(),
        "mostly full 0.125 yd strides: {walking:?}"
    );
}

#[test]
fn frostbolt_chills_the_pulled_spy_to_half_its_run_speed_for_8_seconds() {
    let Some((mut h, _)) = spy_harness() else {
        return;
    };
    // 38 yd apart, inside Frostbolt's 40 yd range; 8 s at 3 yd/s stays inside the 40 yd leash.
    h.app.world_mut().get_mut::<Position>(h.caster).unwrap().x = -35.0;
    h.advance(30);
    h.cast_and_deliver(FROSTBOLT, Some(h.npc));
    for _ in 0..60 {
        if !frostbolt_hits(&h).is_empty() {
            break;
        }
        h.advance(1);
    }
    assert_eq!(frostbolt_hits(&h).len(), 1, "{:?}", h.inbox.failed);
    let chill = auras_of_spell(&h, CHILLED);
    assert_eq!(chill.duration_ms, 8000);
    h.advance(1);
    let chilled: f32 = spy_steps(&mut h, 20).iter().sum();
    assert!((chilled - 3.0).abs() < 0.05, "chilled: {chilled} yd in 1 s");
    h.advance(7200 / 50);
    assert!(
        !has_aura(h.app.world(), h.npc, CHILLED),
        "Chilled lasts 8 s"
    );
    let recovered: f32 = spy_steps(&mut h, 10).iter().sum();
    assert!(
        (recovered - 3.0).abs() < 0.05,
        "{recovered} yd in 0.5 s after Chilled"
    );
}

fn replicated_speed(h: &Harness) -> f32 {
    h.app
        .world()
        .get::<shared::components::MovementSpeed>(h.npc)
        .unwrap()
        .0
}

#[test]
fn the_spy_replicates_the_speed_of_its_gait_chilled_run_then_sheep_walk() {
    // Clients pace the spy's Run and the sheep's Walk clips by this speed.
    let Some((mut h, _)) = spy_harness() else {
        return;
    };
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(shared::components::MovementSpeed(0.0));
    h.app.world_mut().get_mut::<Position>(h.caster).unwrap().x = -20.0;
    h.advance(30);
    h.cast_and_deliver(FROSTBOLT, Some(h.npc));
    for _ in 0..60 {
        if !frostbolt_hits(&h).is_empty() {
            break;
        }
        h.advance(1);
    }
    h.advance(2);
    assert_eq!(
        h.app.world().get::<CreatureMotion>(h.npc),
        Some(&CreatureMotion::Run)
    );
    assert!(
        (replicated_speed(&h) - 3.0).abs() < 1e-4,
        "{}",
        replicated_speed(&h)
    );
    // Chilled wears off before it is polymorphed; the sheep walks at 2.5 yd/s.
    h.advance(9000 / 50);
    cast_polymorph(&mut h);
    let mut walked = false;
    for _ in 0..(8000 / 50) {
        h.advance(1);
        if h.app.world().get::<CreatureMotion>(h.npc) == Some(&CreatureMotion::Walk) {
            walked = true;
            assert!(
                (replicated_speed(&h) - 2.5).abs() < 1e-4,
                "{}",
                replicated_speed(&h)
            );
        }
    }
    assert!(walked, "the sheep never walked");
}

fn replicated_threat_list(h: &Harness) -> Option<Vec<u64>> {
    h.app
        .world()
        .get::<shared::components::UnitThreatList>(h.npc)
        .map(|list| list.0.clone())
}

#[test]
fn the_pulled_spy_replicates_the_mage_on_its_threat_list_through_polymorph_until_it_evades() {
    // Clients colour a neutral creature's plate hostile while they are on this list.
    let Some((mut h, _)) = spy_harness() else {
        return;
    };
    h.advance(20);
    assert_eq!(replicated_threat_list(&h), None, "unpulled: no list");
    cast_frostbolt(&mut h);
    let mage = h.caster.to_bits();
    assert_eq!(replicated_threat_list(&h), Some(vec![mage]));
    cast_polymorph(&mut h);
    h.advance(40);
    assert!(polymorphed(&h));
    assert_eq!(
        replicated_threat_list(&h),
        Some(vec![mage]),
        "kept while confused"
    );
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 0.0;
    h.advance(2);
    assert_eq!(
        replicated_threat_list(&h),
        Some(Vec::new()),
        "cleared on evade"
    );
}

/// A copy of Tentacle of Isorath Transform 84861 whose TRANSFORM names creature 999999999,
/// which no world.db holds. Citizen Costume 73815: TRANSFORM with MiscValue 0.
const UNKNOWN_TEMPLATE_TRANSFORM: u32 = 900_002;
const TENTACLE_TRANSFORM: u32 = 84_861;
const CITIZEN_COSTUME: u32 = 73_815;

fn apply_transform(h: &mut Harness, spell_id: u32) {
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(spell_id)
        .unwrap()
        .clone();
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.npc,
            target: h.npc,
            spell_id,
            effect_mask: crate::auras::aura_effect_mask(&spell),
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.advance(1);
}

/// AuraEffect::HandleAuraTransform (SpellAuraEffects.cpp:2090-2242): a transform into an
/// unknown template shows the pink pig 16358 until it ends; one with MiscValue 0 (not a
/// special-cased spell) keeps the unit's display.
#[test]
fn transforms_without_a_template_follow_trinitycore() {
    let Some(data) = crate::spell_cast::test_harness::world_db() else {
        return;
    };
    let mut unknown = data.store.get(TENTACLE_TRANSFORM).unwrap().clone();
    unknown.id = UNKNOWN_TEMPLATE_TRANSFORM;
    for effect in &mut unknown.effects {
        if effect.aura == crate::auras::handlers::TRANSFORM {
            effect.misc_values[0] = 999_999_999;
        }
    }
    let Some(mut h) = harness_with_plugins(vec![unknown], &[CITIZEN_COSTUME], MAGE, false, |app| {
        app.insert_resource(world_data());
    }) else {
        return;
    };
    let npc = h.npc;
    h.app.world_mut().entity_mut(npc).insert(ModelDisplay {
        display_id: KOBOLD_DISPLAY,
    });
    apply_transform(&mut h, UNKNOWN_TEMPLATE_TRANSFORM);
    assert_eq!(display(&h), 16358);
    crate::auras::remove_auras(
        h.app.world_mut(),
        npc,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == UNKNOWN_TEMPLATE_TRANSFORM,
    );
    h.advance(1);
    assert_eq!(display(&h), KOBOLD_DISPLAY);

    apply_transform(&mut h, CITIZEN_COSTUME);
    assert!(crate::auras::has_aura(h.app.world(), npc, CITIZEN_COSTUME));
    assert_eq!(display(&h), KOBOLD_DISPLAY);
}
