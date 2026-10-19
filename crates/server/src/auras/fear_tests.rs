//! Native aura7 MOD_FEAR control state (TC a352b1fa Unit::SetControlled/SetFeared,
//! Spell::CheckCasterAuras, SpellInfo::SpellCancelsAuraEffect/_LoadImmunityInfo).
//! Flee movement is a separate boundary.

use rusqlite::{Connection, OpenFlags};
use shared::components::{
    CreatureMotion, Health, ModelDisplay, Position, UnitLevel, UnitTarget, WeaponType,
};
use shared::protocol::{CombatLogKind, MissKind};

use super::*;
use crate::creature_ai::SpawnPoint;
use crate::item_stats::WeaponDamage;
use crate::spell_cast::test_harness::{Harness, harness, harness_with, harness_with_plugins};
use crate::stats::CharacterStats;
use crate::world_data::WorldData;

/// Native aura7 on a real baked 180yd floor, not an injected route.
fn fleeing_harness() -> (Harness, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let map = dir.path().join("0");
    std::fs::create_dir(&map).unwrap();
    let a = Vec3::new(10.0, 0.0, 10.0);
    let b = Vec3::new(10.0, 0.0, 190.0);
    let c = Vec3::new(190.0, 0.0, 190.0);
    let d = Vec3::new(190.0, 0.0, 10.0);
    let tile = navigation::bake::bake_tile(&[[a, b, c], [a, c, d]], (32, 31)).unwrap();
    assert!(!tile.is_empty());
    std::fs::write(map.join("32_31.nav"), tile).unwrap();
    let mut h = harness_with_plugins(
        Vec::new(),
        &[INTIMIDATING_SHOUT, 122, TURN_EVIL],
        WARRIOR,
        false,
        |app| {
            app.insert_resource(crate::navigation::Navigation::new(dir.path()));
            let conn = Connection::open_with_flags(
                crate::test_data::world_db_path(),
                OpenFlags::SQLITE_OPEN_READ_ONLY,
            )
            .unwrap();
            app.insert_resource(
                crate::scripted_ai::model_reach::load_model_reaches(&conn).unwrap(),
            );
        },
    )
    .unwrap();
    h.app.world_mut().entity_mut(h.caster).insert((
        Position {
            x: 100.0,
            y: 0.2,
            z: 100.0,
        },
        shared::components::MovementControl::default(),
    ));
    h.app.world_mut().entity_mut(h.npc).insert((
        Position {
            x: 110.0,
            y: 0.2,
            z: 100.0,
        },
        ModelDisplay {
            display_id: KOBOLD_DISPLAY,
        },
    ));
    (h, dir)
}

fn assert_flees_then_stops(mut h: Harness, caster: Entity, victim: Entity) {
    let before = *h.app.world().get::<Position>(victim).unwrap();
    let origin = *h.app.world().get::<Position>(caster).unwrap();
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target: victim,
            spell_id: INTIMIDATING_SHOUT,
            effect_mask: 2,
            source: AuraSource::Cast,
        },
    );
    assert!(has_aura(h.app.world(), victim, INTIMIDATING_SHOUT));
    h.advance(20);
    let after = *h.app.world().get::<Position>(victim).unwrap();
    assert!(
        (after.x - origin.x).hypot(after.z - origin.z)
            > (before.x - origin.x).hypot(before.z - origin.z) + 2.0,
        "native feared unit must flee away: {before:?} -> {after:?}"
    );
    if victim == h.caster {
        assert!(
            h.app
                .world()
                .get::<shared::components::MovementControl>(victim)
                .unwrap()
                .controlled
        );
    }
    remove_auras(h.app.world_mut(), victim, RemoveReason::Cancelled, |aura| {
        aura.spell_id == INTIMIDATING_SHOUT
    });
    h.advance(20);
    assert_eq!(*h.app.world().get::<Position>(victim).unwrap(), after);
    if victim == h.caster {
        assert!(
            !h.app
                .world()
                .get::<shared::components::MovementControl>(victim)
                .unwrap()
                .controlled
        );
    }
}

#[test]
fn navigation_deferred_fear_holds_until_collision_and_path_publication() {
    let (mut h, _dir) = fleeing_harness();
    let (caster, victim) = (h.caster, h.npc);
    start_fleeing(&mut h, victim, caster);
    h.app
        .world_mut()
        .resource_mut::<Time>()
        .advance_by(std::time::Duration::from_millis(50));
    let before = *h.app.world().get::<Position>(victim).unwrap();
    super::fleeing::update(h.app.world_mut());
    assert_eq!(
        *h.app.world().get::<Position>(victim).unwrap(),
        before,
        "unpublished collision and path cannot move a feared unit"
    );
    crate::navigation::drain_fixture(&mut h.app);
    super::fleeing::update(h.app.world_mut());
    assert_eq!(
        *h.app.world().get::<Position>(victim).unwrap(),
        before,
        "collision completion alone is not a published path"
    );
    crate::navigation::drain_fixture(&mut h.app);
    super::fleeing::update(h.app.world_mut());
    let after = *h.app.world().get::<Position>(victim).unwrap();
    assert!(
        after.x > before.x,
        "published real route must flee away: {after:?}"
    );
}

#[test]
fn fear_native_creature_flees_on_navmesh_and_stops_on_removal() {
    let (h, _dir) = fleeing_harness();
    let (caster, victim) = (h.caster, h.npc);
    assert_flees_then_stops(h, caster, victim);
}

#[test]
fn fear_native_player_flees_on_navmesh_loses_control_and_stops_on_removal() {
    let (h, _dir) = fleeing_harness();
    let (caster, victim) = (h.npc, h.caster);
    assert_flees_then_stops(h, caster, victim);
}

fn start_fleeing(h: &mut Harness, victim: Entity, caster: Entity) {
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target: victim,
            spell_id: INTIMIDATING_SHOUT,
            effect_mask: 2,
            source: AuraSource::Cast,
        },
    );
}

#[test]
fn fear_native_expiry_stops_player_and_restores_control() {
    let (mut h, _dir) = fleeing_harness();
    let (player, npc) = (h.caster, h.npc);
    start_fleeing(&mut h, player, npc);
    h.advance(20);
    assert!(
        h.app
            .world()
            .get::<shared::components::MovementControl>(player)
            .unwrap()
            .controlled
    );
    h.advance(200);
    assert!(!has_aura(h.app.world(), player, INTIMIDATING_SHOUT));
    let stopped = *h.app.world().get::<Position>(player).unwrap();
    assert!(
        !h.app
            .world()
            .get::<shared::components::MovementControl>(player)
            .unwrap()
            .controlled
    );
    h.advance(20);
    assert_eq!(*h.app.world().get::<Position>(player).unwrap(), stopped);
}

#[test]
fn fear_removal_does_not_release_newer_player_movement_owner() {
    let (mut h, _dir) = fleeing_harness();
    let (player, npc) = (h.caster, h.npc);
    start_fleeing(&mut h, player, npc);
    h.advance(4);
    let owner = {
        let mut control = h
            .app
            .world_mut()
            .get_mut::<shared::components::MovementControl>(player)
            .unwrap();
        control.epoch += 1;
        *control
    };
    let stopped = *h.app.world().get::<Position>(player).unwrap();
    h.advance(10);
    assert_eq!(*h.app.world().get::<Position>(player).unwrap(), stopped);
    remove_auras(h.app.world_mut(), player, RemoveReason::Cancelled, |aura| {
        aura.spell_id == INTIMIDATING_SHOUT
    });
    assert_eq!(
        *h.app
            .world()
            .get::<shared::components::MovementControl>(player)
            .unwrap(),
        owner
    );
}

#[test]
fn fear_unbaked_map_never_substitutes_direct_movement() {
    let (mut h, _dir) = fleeing_harness();
    let (player, npc) = (h.caster, h.npc);
    h.app
        .world_mut()
        .entity_mut(player)
        .insert(crate::maps::InMap::continent(34));
    let before = *h.app.world().get::<Position>(player).unwrap();
    start_fleeing(&mut h, player, npc);
    h.advance(80);
    assert_eq!(*h.app.world().get::<Position>(player).unwrap(), before);
    assert!(
        h.app
            .world()
            .get::<shared::components::MovementControl>(player)
            .unwrap()
            .controlled
    );
}

#[test]
fn fear_runs_at_native_speed_then_pauses_before_next_route() {
    let (mut h, _dir) = fleeing_harness();
    let (player, npc) = (h.caster, h.npc);
    start_fleeing(&mut h, player, npc);
    let mut travelled = 0.0;
    let mut previous = *h.app.world().get::<Position>(player).unwrap();
    let mut stopped = None;
    for _ in 0..120 {
        h.advance(1);
        let next = *h.app.world().get::<Position>(player).unwrap();
        let step = (next.x - previous.x).hypot(next.z - previous.z);
        assert!(step <= 7.0 * 0.05 + 0.001, "run speed: {step}");
        travelled += step;
        if travelled > 1.0 && step < 0.0001 {
            stopped = Some(next);
            break;
        }
        previous = next;
    }
    let stopped = stopped.expect("route≤30yd ends and pauses");
    assert!(travelled <= 30.01);
    h.advance(14);
    assert_eq!(
        *h.app.world().get::<Position>(player).unwrap(),
        stopped,
        "pause≥800ms"
    );
    h.advance(20);
    assert_ne!(
        *h.app.world().get::<Position>(player).unwrap(),
        stopped,
        "pause≤1500ms"
    );
}

#[test]
fn fear_destination_stops_before_live_collision_wall() {
    let (mut h, _dir) = fleeing_harness();
    let (player, npc) = (h.caster, h.npc);
    let mut los =
        crate::line_of_sight::LineOfSight::load(&crate::ground::ground_data_dir().join("los"))
            .unwrap();
    let vertices = vec![
        Vec3::new(114.0, 0.0, 0.0),
        Vec3::new(114.0, 4.0, 0.0),
        Vec3::new(114.0, 4.0, 200.0),
        Vec3::new(114.0, 0.0, 200.0),
    ];
    let wall = std::sync::Arc::new(los::LosTile::build(
        32,
        31,
        vertices,
        vec![[0, 1, 2], [0, 2, 3]],
    ));
    los.set_live_hulls(std::collections::HashMap::from([(
        crate::maps::InMap::continent(0),
        vec![wall],
    )]));
    h.app.insert_resource(los);
    start_fleeing(&mut h, npc, player);
    h.advance(40);
    let after = h.app.world().get::<Position>(npc).unwrap();
    assert!(
        after.x > 110.0 && after.x <= 113.1,
        "first collision: {after:?}"
    );
}

#[test]
fn fear_native_root_pauses_running_until_root_is_removed() {
    let (mut h, _dir) = fleeing_harness();
    let (player, npc) = (h.caster, h.npc);
    start_fleeing(&mut h, npc, player);
    h.advance(10);
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: player,
            target: npc,
            spell_id: 122,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    assert!(has_aura(h.app.world(), npc, 122));
    let stopped = *h.app.world().get::<Position>(npc).unwrap();
    h.advance(10);
    assert_eq!(*h.app.world().get::<Position>(npc).unwrap(), stopped);
    remove_auras(h.app.world_mut(), npc, RemoveReason::Cancelled, |aura| {
        aura.spell_id == 122
    });
    h.advance(10);
    assert_ne!(*h.app.world().get::<Position>(npc).unwrap(), stopped);
}

#[test]
fn fear_removing_one_of_two_native_auras_keeps_control_lost() {
    let (mut h, _dir) = fleeing_harness();
    let (player, npc) = (h.caster, h.npc);
    start_fleeing(&mut h, player, npc);
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: npc,
            target: player,
            spell_id: TURN_EVIL,
            effect_mask: 1,
            source: AuraSource::Cast,
        },
    );
    assert!(has_aura(h.app.world(), player, TURN_EVIL));
    h.advance(4);
    remove_auras(h.app.world_mut(), player, RemoveReason::Cancelled, |aura| {
        aura.spell_id == INTIMIDATING_SHOUT
    });
    let before = *h.app.world().get::<Position>(player).unwrap();
    h.advance(10);
    assert_ne!(*h.app.world().get::<Position>(player).unwrap(), before);
    assert!(
        h.app
            .world()
            .get::<shared::components::MovementControl>(player)
            .unwrap()
            .controlled
    );
    remove_auras(h.app.world_mut(), player, RemoveReason::Cancelled, |aura| {
        aura.spell_id == TURN_EVIL
    });
    assert!(
        !h.app
            .world()
            .get::<shared::components::MovementControl>(player)
            .unwrap()
            .controlled
    );
}

const INTIMIDATING_SHOUT: u32 = 5246;
const SLEEP_WALK: u32 = 360806;
const SLAM: u32 = 1464;
const TURN_EVIL: u32 = 10326;
const BERSERKER_RAGE: u32 = 18499;
const WILL_OF_THE_FORSAKEN: u32 = 7744;
const BARKSKIN: u32 = 22812;
const EVASION: u32 = 5277;
const KOBOLD_VERMIN: u32 = 6;
const KOBOLD_DISPLAY: u32 = 10913;
const WARRIOR: u8 = 1;
const ROGUE: u8 = 4;
const DRUID: u8 = 11;
const FLEEING: &str = "Can't do that while fleeing";

fn kobold_data() -> WorldData {
    let path = crate::test_data::world_db_path();
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let expected = crate::world_data::load_expected_stats(&conn);
    let mut templates = crate::world_data::load_templates(&conn, &expected);
    templates.retain(|entry, _| *entry == KOBOLD_VERMIN);
    crate::world_data::load_template_models(&conn, &mut templates);
    WorldData {
        templates,
        ..Default::default()
    }
}

/// A warrior with Intimidating Shout and a world kobold with creature AI and aggro.
fn shout_harness() -> Harness {
    let mut h = harness_with_plugins(Vec::new(), &[INTIMIDATING_SHOUT], WARRIOR, false, |app| {
        app.insert_resource(kobold_data());
        app.add_plugins((
            crate::creature_ai::CreatureAiPlugin,
            crate::creature_aggro::CreatureAggroPlugin,
            crate::unit_target::UnitTargetPlugin,
        ));
    })
    .expect("real world.db required");
    let position = *h.app.world().get::<Position>(h.npc).unwrap();
    h.app.world_mut().entity_mut(h.npc).insert((
        Health {
            current: 5000.0,
            max: 5000.0,
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
    ));
    let mut commands = h.app.world_mut().commands();
    let mut entity = commands.entity(h.npc);
    crate::creature_aggro::insert_aggro_components(
        &mut entity,
        crate::creature_aggro::AggroMode::Reactive,
    );
    h.app.world_mut().flush();
    h.advance(1);
    h
}

fn kobold_swings_within(h: &mut Harness, frames: usize) -> bool {
    let health = h.health(h.caster);
    h.advance(frames);
    h.health(h.caster) < health
}

/// The npc fears the harness player with one native fear effect.
fn fear_player(h: &mut Harness, spell_id: u32, index: u8) {
    let (npc, player) = (h.npc, h.caster);
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: npc,
            target: player,
            spell_id,
            effect_mask: 1 << index,
            source: AuraSource::Cast,
        },
    );
    assert!(has_aura(h.app.world(), player, spell_id));
}

fn cast_outcome(
    h: &mut Harness,
    spell_id: u32,
    target: Option<Entity>,
) -> Result<(), Option<String>> {
    h.inbox.failed.clear();
    h.inbox.go.clear();
    h.cast_and_deliver(spell_id, target);
    if h.inbox.go.iter().any(|go| go.spell_id == spell_id) {
        return Ok(());
    }
    let failure = h
        .inbox
        .failed
        .iter()
        .find(|failed| failed.spell_id == spell_id);
    Err(failure.and_then(|failed| failed.detail.clone()))
}

#[test]
fn class_completeness_native_fear_stops_swings_and_clears_target_until_removed() {
    let mut h = shout_harness();
    let (npc, warrior) = (h.npc, h.caster);
    crate::creature_aggro::attack_start(h.app.world_mut(), npc, warrior);
    assert!(kobold_swings_within(&mut h, 100), "the kobold never hit");
    h.advance(30);
    assert_eq!(cast_outcome(&mut h, INTIMIDATING_SHOUT, Some(npc)), Ok(()));
    assert!(has_aura(h.app.world(), npc, INTIMIDATING_SHOUT));
    assert_eq!(
        h.app.world().get::<UnitTarget>(npc),
        Some(&UnitTarget(None))
    );
    assert!(!kobold_swings_within(&mut h, 120), "a feared kobold swung");
    remove_auras(h.app.world_mut(), npc, RemoveReason::Cancelled, |aura| {
        aura.spell_id == INTIMIDATING_SHOUT
    });
    assert!(kobold_swings_within(&mut h, 120), "the kobold fights again");
    assert_eq!(
        h.app.world().get::<UnitTarget>(npc),
        Some(&UnitTarget(Some(warrior.to_bits())))
    );
}

#[test]
fn class_completeness_native_fear_blocks_casts_unless_the_spell_purges_its_mechanic() {
    let mut h = harness(
        Vec::new(),
        &[INTIMIDATING_SHOUT, SLAM, BERSERKER_RAGE],
        WARRIOR,
    )
    .expect("real world.db required");
    h.set_power(shared::components::PowerType::Rage, 1000.0);
    fear_player(&mut h, INTIMIDATING_SHOUT, 1);
    let npc = h.npc;
    assert_eq!(
        cast_outcome(&mut h, SLAM, Some(npc)),
        Err(Some(FLEEING.into()))
    );
    // Native18499: Attr1 ImmunityPurgesEffect with aura77 fear mechanic.
    assert_eq!(cast_outcome(&mut h, BERSERKER_RAGE, None), Ok(()));
    assert!(!has_aura(h.app.world(), h.caster, INTIMIDATING_SHOUT));
}

#[test]
fn class_completeness_native_sleep_fear_requires_a_matching_purge_mechanic() {
    let mut h = harness(
        Vec::new(),
        &[SLEEP_WALK, BERSERKER_RAGE, WILL_OF_THE_FORSAKEN],
        WARRIOR,
    )
    .expect("real world.db required");
    fear_player(&mut h, SLEEP_WALK, 0);
    // Sleep Walk carries mechanic10; Berserker Rage purges5/14/30/23 only.
    assert_eq!(
        cast_outcome(&mut h, BERSERKER_RAGE, None),
        Err(Some(FLEEING.into()))
    );
    assert_eq!(cast_outcome(&mut h, WILL_OF_THE_FORSAKEN, None), Ok(()));
    assert!(!has_aura(h.app.world(), h.caster, SLEEP_WALK));
}

#[test]
fn class_completeness_native_allow_while_fleeing_uses_allowed_mechanics() {
    // TC adds stun/freeze/knockout/sleep plus fear/horror for Barkskin22812; Turn Evil
    // carries mechanic23, outside that allowed mask.
    let cases = [
        (INTIMIDATING_SHOUT, 1, true),
        (SLEEP_WALK, 0, true),
        (TURN_EVIL, 0, false),
    ];
    for (fear, index, allowed) in cases {
        let mut h = harness(Vec::new(), &[fear, BARKSKIN], DRUID).expect("real world.db required");
        fear_player(&mut h, fear, index);
        let outcome = cast_outcome(&mut h, BARKSKIN, None);
        assert_eq!(outcome.is_ok(), allowed, "{fear}: {outcome:?}");
        // Barkskin does not purge fear; only admission changes.
        assert!(has_aura(h.app.world(), h.caster, fear));
    }
}

#[test]
fn class_completeness_native_feared_victim_cannot_dodge() {
    for feared in [false, true] {
        let mut h = harness_with(Vec::new(), &[EVASION, INTIMIDATING_SHOUT], ROGUE, true)
            .expect("real world.db and computed stats required");
        h.advance(3);
        let rogue = h.caster;
        assert_eq!(cast_outcome(&mut h, EVASION, Some(rogue)), Ok(()));
        if feared {
            fear_player(&mut h, INTIMIDATING_SHOUT, 1);
        }
        let npc = h.npc;
        h.app.world_mut().entity_mut(npc).insert((
            CharacterStats {
                main_hand: WeaponDamage {
                    min: 10.0,
                    max: 10.0,
                    speed: 2.6,
                    kind: WeaponType::OneHand,
                },
                ..Default::default()
            },
            AuraStatMods {
                melee_hit_chance_pct: 100.0,
                ..Default::default()
            },
            crate::combat::AutoAttack::new(rogue),
        ));
        h.advance(4);
        let dodged = h.inbox.log.iter().any(|event| {
            event.source == Some(npc.to_bits())
                && event.kind == CombatLogKind::Miss(MissKind::Dodge)
        });
        assert_eq!(dodged, !feared);
    }
}
