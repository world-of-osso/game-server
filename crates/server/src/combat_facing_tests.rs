//! TC a352b1fa Unit.cpp2404–2414: front arc/288, player-only behind dodge gate.
use bevy::ecs::system::RunSystemOnce;
use bevy::prelude::*;
use shared::components::{Level, Position, Rotation, UnitLevel};
use shared::protocol::{CombatLogEvent, CombatLogKind, MissKind};

use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::{Harness, harness, harness_with, world_db};

const DUMMY: u32 = 31_144;
const EVASION: u32 = 5_277;
const ALL_DIRECTIONS: u32 = 97_574;
const NPC_SWINGS: usize = 256;
const PLAYER_SWINGS: usize = 5;

pub(super) fn collect_swings(
    h: &mut Harness,
    attacker: Entity,
    victim: Entity,
    count: usize,
) -> Vec<CombatLogEvent> {
    let mut cursor = h.inbox.log.len();
    h.app
        .world_mut()
        .run_system_once(move |mut commands: Commands| {
            super::start_attack(&mut commands, attacker, victim, None);
        })
        .unwrap();
    let mut swings = Vec::new();
    // Bound ordinary delivery, never rewrite native swing timers or RNG.
    for _ in 0..count * 60 {
        h.advance(1);
        swings.extend(
            h.inbox.log[cursor..]
                .iter()
                .filter(|event| {
                    event.source == Some(attacker.to_bits())
                        && event.target == Some(victim.to_bits())
                        && event.spell_id.is_none()
                        && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
                })
                .cloned(),
        );
        cursor = h.inbox.log.len();
        if swings.len() == count {
            super::attack_stop(h.app.world_mut(), attacker);
            h.advance(3);
            return swings;
        }
    }
    panic!("only {} of{count} ordinary swing attempts", swings.len());
}

fn dummy_fixture(behind: bool, all_directions: bool) -> (Harness, Entity) {
    let mut h = harness(Vec::new(), &[ALL_DIRECTIONS], 1).unwrap();
    h.app.world_mut().entity_mut(h.caster).insert((
        Level(80),
        UnitLevel(80),
        Position {
            x: if behind { 0.0 } else { 6.0 },
            y: 0.0,
            z: 0.0,
        },
    ));
    h.app.insert_resource(crate::world_data::load_world_data(
        &world_db().unwrap().powers_db,
    ));
    let caster = h.caster;
    let victim =
        crate::zones::summon_creature(h.app.world_mut(), caster, DUMMY, [3.0, 0.0, 0.0, 0.0])
            .expect("native unflagged level80 Grandmaster Dummy");
    assert_eq!(h.app.world().get::<UnitLevel>(victim).unwrap().0, 80);
    assert_eq!(
        crate::spell_cast::targets::orientation(h.app.world(), victim),
        0.0
    );
    if all_directions {
        crate::spell_cast::creature_cast(
            h.app.world_mut(),
            victim,
            ALL_DIRECTIONS,
            Some(victim),
            true,
        )
        .expect("actual native self-only aura288 cast");
        h.advance(3);
        assert!(
            h.app
                .world()
                .get::<ActiveAuras>(victim)
                .unwrap()
                .has_type(288)
        );
    }
    (h, victim)
}

fn rogue_fixture(behind: bool, all_directions: bool) -> Harness {
    let mut h = harness_with(Vec::new(), &[EVASION, ALL_DIRECTIONS], 4, true).unwrap();
    h.app.world_mut().entity_mut(h.caster).insert(Rotation {
        x: 0.0,
        y: std::f32::consts::FRAC_PI_2,
        z: 0.0,
    });
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: if behind { -3.0 } else { 3.0 },
        y: 0.0,
        z: 0.0,
    });
    h.advance(3);
    assert_eq!(
        crate::spell_cast::targets::orientation(h.app.world(), h.caster),
        0.0
    );
    if all_directions {
        h.cast_and_deliver(ALL_DIRECTIONS, Some(h.caster));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        assert!(
            h.app
                .world()
                .get::<ActiveAuras>(h.caster)
                .unwrap()
                .has_type(288)
        );
    }
    h.cast_and_deliver(EVASION, Some(h.caster));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(
        h.app
            .world()
            .get::<ActiveAuras>(h.caster)
            .unwrap()
            .modifier(49),
        200.0
    );
    assert!(
        h.app
            .world()
            .get::<crate::stats::CharacterStats>(h.caster)
            .unwrap()
            .defense
            .dodge
            >= 200.0
    );
    h
}

fn parry_or_block(event: &CombatLogEvent) -> bool {
    event.kind == CombatLogKind::Miss(MissKind::Parry) || event.blocked > 0
}

fn assert_evasion_live(h: &Harness) {
    assert!(crate::auras::has_aura(h.app.world(), h.caster, EVASION));
}

#[test]
fn native_melee_facing_npc_behind_rejects_parry_block_but_retains_dodge() {
    let (mut h, victim) = dummy_fixture(true, false);
    let caster = h.caster;
    let swings = collect_swings(&mut h, caster, victim, NPC_SWINGS);
    // P(missing the old6%parry+3%block bug)=.91^256=3.27e-11.
    assert!(!swings.iter().any(parry_or_block), "{swings:?}");
    // NPC dodge stays3%; P(noDodge)=.97^256≈4.11e-4<1/1000.
    assert!(
        swings
            .iter()
            .any(|event| event.kind == CombatLogKind::Miss(MissKind::Dodge)),
        "{swings:?}"
    );
}

#[test]
fn native_melee_facing_npc_front_retains_parry_or_block() {
    let (mut h, victim) = dummy_fixture(false, false);
    let caster = h.caster;
    let swings = collect_swings(&mut h, caster, victim, NPC_SWINGS);
    assert!(swings.iter().any(parry_or_block), "{swings:?}");
}

#[test]
fn native_melee_facing_npc_288_restores_parry_or_block_from_behind() {
    let (mut h, victim) = dummy_fixture(true, true);
    let caster = h.caster;
    let swings = collect_swings(&mut h, caster, victim, NPC_SWINGS);
    assert!(swings.iter().any(parry_or_block), "{swings:?}");
}

#[test]
fn native_melee_facing_player_behind_rejects_actual_evasion_dodge() {
    let mut h = rogue_fixture(true, false);
    let (attacker, victim) = (h.npc, h.caster);
    let swings = collect_swings(&mut h, attacker, victim, PLAYER_SWINGS);
    assert_evasion_live(&h);
    // Five ordinary attempts fit native10sEvasion; P(all5Miss)=.05^5=3.125e-7.
    assert!(
        !swings
            .iter()
            .any(|event| event.kind == CombatLogKind::Miss(MissKind::Dodge)),
        "{swings:?}"
    );
    assert!(
        swings
            .iter()
            .any(|event| event.kind == CombatLogKind::Damage && event.amount > 0),
        "{swings:?}"
    );
}

#[test]
fn native_melee_facing_player_front_retains_actual_evasion_dodge() {
    let mut h = rogue_fixture(false, false);
    let (attacker, victim) = (h.npc, h.caster);
    let swings = collect_swings(&mut h, attacker, victim, PLAYER_SWINGS);
    assert_evasion_live(&h);
    assert!(
        swings
            .iter()
            .any(|event| event.kind == CombatLogKind::Miss(MissKind::Dodge)),
        "{swings:?}"
    );
}

#[test]
fn native_melee_facing_player_288_restores_evasion_dodge_from_behind() {
    let mut h = rogue_fixture(true, true);
    let (attacker, victim) = (h.npc, h.caster);
    let swings = collect_swings(&mut h, attacker, victim, PLAYER_SWINGS);
    assert_evasion_live(&h);
    assert!(
        swings
            .iter()
            .any(|event| event.kind == CombatLogKind::Miss(MissKind::Dodge)),
        "{swings:?}"
    );
}
