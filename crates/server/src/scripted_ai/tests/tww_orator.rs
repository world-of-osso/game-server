//! TC a352b1fa CityOfThreads/boss_orator_krix_vizk.cpp native Normal/Follower boundaries.
use super::*;
use crate::spell_cast::powers::power_value;
use shared::components::PowerType;

const ORATOR: u32 = 216619;
const SUBJUGATE: u32 = 434722;
const TERRORIZE_SELECTOR: u32 = 434808;
const TERRORIZE: u32 = 434779;
const INDOCTRINATION: u32 = 434829;

pub(super) fn fixture(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = encounter_fixture(
        ORATOR,
        2669,
        1,
        difficulty,
        None,
        EncounterState::NotStarted,
        false,
    );
    let boss = boss.unwrap();
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

pub(super) fn pull(h: &mut Harness, boss: Entity) {
    let player = h.player();
    let mut position = *h.app.world().get::<Position>(boss).unwrap();
    position.x += 3.0;
    h.app.world_mut().entity_mut(player).insert(position);
    h.engage(boss);
}

pub(super) fn cast_count(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

fn rotation(difficulty: u32) {
    let (mut h, boss, map) = fixture(difficulty);
    pull(&mut h, boss);
    h.advance(4.3);
    assert!(h.app.world().get::<CastState>(boss).is_none());
    assert_eq!(cast_count(&h, boss, SUBJUGATE), 0);
    h.advance(0.3);
    let cast = h
        .app
        .world()
        .get::<CastState>(boss)
        .expect("source4500ms Subjugate starts ordinary cast");
    assert_eq!(cast.spell_id, SUBJUGATE);
    assert_eq!(cast.target, h.player().to_bits());
    assert_eq!(cast.duration, 1.25);
    let hp = h.health(h.player()).current;
    h.advance(1.4);
    assert_eq!(cast_count(&h, boss, SUBJUGATE), 1);
    assert!(h.health(h.player()).current < hp);
    h.advance(3.5);
    assert_eq!(cast_count(&h, boss, TERRORIZE_SELECTOR), 1);
    let cast = h
        .app
        .world()
        .get::<CastState>(boss)
        .expect("ordinary selector starts native3s child");
    assert_eq!(cast.spell_id, TERRORIZE);
    assert_eq!(cast.duration, 3.0);
    h.advance(3.2);
    assert_eq!(cast_count(&h, boss, TERRORIZE), 1);
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.source == Some(boss.to_bits())
                && event.spell_id == Some(TERRORIZE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    let other = h.players[1].0;
    assert!(h.inbox(1).log.iter().all(
        |event| event.source == Some(other.to_bits()) && event.target == Some(other.to_bits())
    ));
    let player = h.player();
    crate::combat::kill_unit(h.app.world_mut(), player, boss);
    h.advance(0.1);
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 1),
        Some(EncounterState::Done)
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|event| event.encounter_id == 2907)
    );
    assert!(h.inbox(1).ends.is_empty());
}

fn energy(difficulty: u32) {
    let (mut h, boss, map) = fixture(difficulty);
    assert_eq!(
        power_value(h.app.world(), boss, PowerType::Energy),
        Some((0.0, 100.0))
    );
    assert!(h.has_aura(boss, 454689), "source Oration appearance aura");
    pull(&mut h, boss);
    h.advance(0.8);
    assert_eq!(
        power_value(h.app.world(), boss, PowerType::Energy),
        Some((0.0, 100.0))
    );
    h.advance(0.3);
    assert_eq!(
        power_value(h.app.world(), boss, PowerType::Energy),
        Some((4.0, 100.0))
    );
    h.advance(1.2);
    assert_eq!(
        power_value(h.app.world(), boss, PowerType::Energy),
        Some((8.0, 100.0))
    );
    let player = h.player();
    let hp = h.health(player).current;
    // Source25 four-Energy increments, alternating1200/800ms, defer during native casts.
    // Bound observation by90s; no fabricated Energy or phase callback in this fixture.
    for _ in 0..900 {
        h.advance(0.1);
        if cast_count(&h, boss, INDOCTRINATION) > 0 {
            break;
        }
    }
    assert_eq!(cast_count(&h, boss, INDOCTRINATION), 1);
    assert_eq!(
        power_value(h.app.world(), boss, PowerType::Energy),
        Some((100.0, 100.0))
    );
    h.advance(4.3);
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.source == Some(boss.to_bits())
                && event.spell_id == Some(434832)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0)
    );
    assert!(h.health(player).current < hp);
    assert_eq!(
        power_value(h.app.world(), boss, PowerType::Energy),
        Some((0.0, 100.0))
    );
    h.app.world_mut().get_mut::<Position>(player).unwrap().x += 500.0;
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    h.advance(0.1);
    assert_eq!(
        get_boss_state(h.app.world_mut(), map, 1),
        Some(EncounterState::Fail),
        "TC ScriptedCreature::_DespawnAtEvade defaults30s and marks FAIL"
    );
}

#[test]
fn encounter_coverage_tww_orator_normal_native_rotation_and_credit() {
    rotation(1);
}
#[test]
fn encounter_coverage_tww_orator_follower205_native_rotation_and_credit() {
    rotation(205);
}
#[test]
fn encounter_coverage_tww_orator_normal_native_energy_and_indoctrination() {
    energy(1);
}
#[test]
fn encounter_coverage_tww_orator_follower205_native_energy_and_indoctrination() {
    energy(205);
}
