//! Real Seething Shore parachute chain (TC a352b1fa battleground_seething_shore.cpp:530-641)
//! and MODIFY_FALL_DAMAGE_PCT 374 (`Player::HandleFall`) through the local spell store,
//! fixed-tick aura periodics and the production gravity/knockback landings.
use bevy::prelude::*;
use shared::components::{MovementControl, Position, Rotation, VerticalVelocity};
use shared::protocol::{CombatLogKind, EnvironmentalKind};

use crate::auras::{ActiveAuras, RemoveReason};
use crate::networking::FallTracker;
use crate::spell_cast::test_harness::{Harness, harness_with_plugins};

const ROCKET_PARACHUTE: u32 = 250_917;
const ROCKET_PARACHUTE_2: u32 = 250_921;
const NO_FALLING_DAMAGE: u32 = 262_359;
const PARACHUTE: u32 = 269_779;
/// 269779 Effect0 EffectBasePoints in local DB2.
const PARACHUTE_AURA: u32 = 269_776;
const CLAIM_JUMPER_TRACKER: u32 = 261_955;
const DEATH_FROM_ABOVE_TRACKER: u32 = 261_954;
const SPELLS: &[u32] = &[
    ROCKET_PARACHUTE,
    ROCKET_PARACHUTE_2,
    NO_FALLING_DAMAGE,
    PARACHUTE,
    PARACHUTE_AURA,
    CLAIM_JUMPER_TRACKER,
    DEATH_FROM_ABOVE_TRACKER,
];

fn falling_harness(height: f32) -> Harness {
    let mut h = harness_with_plugins(Vec::new(), SPELLS, 1, false, |app| {
        app.add_systems(FixedUpdate, crate::networking::apply_terrain_gravity);
    })
    .expect("local world.db");
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&crate::ground::ground_data_dir().join("los"))
            .unwrap(),
    );
    h.app
        .insert_resource(crate::terrain_height::test_helpers::flat_ground(
            33.0, 33.0, 0.0,
        ));
    h.app.world_mut().entity_mut(h.caster).insert((
        Position {
            x: 5.0,
            y: height,
            z: -5.0,
        },
        Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        VerticalVelocity(0.0),
        FallTracker::grounded(height),
        MovementControl::default(),
    ));
    h
}

fn height(h: &Harness) -> f32 {
    h.app.world().get::<Position>(h.caster).unwrap().y
}

fn has_aura(h: &Harness, spell_id: u32) -> bool {
    h.app
        .world()
        .get::<ActiveAuras>(h.caster)
        .is_some_and(|auras| auras.has_spell(spell_id))
}

fn applied(h: &Harness, spell_id: u32) -> usize {
    h.inbox
        .log
        .iter()
        .filter(|log| log.kind == CombatLogKind::AuraApplied && log.spell_id == Some(spell_id))
        .count()
}

fn falling_logs(h: &Harness) -> usize {
    h.inbox
        .log
        .iter()
        .filter(|log| log.kind == CombatLogKind::Environmental(EnvironmentalKind::Falling))
        .count()
}

fn disembark(h: &mut Harness) {
    // transport::disembark: TC transport_seething_shore::OnRemovePassenger.
    let player = h.caster;
    crate::spell_cast::cast_triggered(h.app.world_mut(), player, ROCKET_PARACHUTE, Some(player), 0);
    crate::spell_cast::cast_triggered(
        h.app.world_mut(),
        player,
        NO_FALLING_DAMAGE,
        Some(player),
        0,
    );
}

/// 250917 ticks 250921; 250921's 50 ms ground check deploys 269779 once at
/// `GetPositionZ() - GetFloorZ() <= 30`; the landing deals no falling damage.
#[test]
fn seething_parachute_ticks_deploy_once_at_floor_threshold_and_land_without_fall_damage() {
    let mut h = falling_harness(80.0);
    disembark(&mut h);
    h.advance(1);
    assert!(has_aura(&h, ROCKET_PARACHUTE) && has_aura(&h, NO_FALLING_DAMAGE));
    // 250917 Attributes_5 0x200 EXTRA_INITIAL_PERIOD: its first tick is at application.
    assert!(
        has_aura(&h, ROCKET_PARACHUTE_2),
        "250917 periodic dummy casts 250921"
    );
    assert!(
        !has_aura(&h, PARACHUTE_AURA),
        "80 yd is above the deploy height"
    );

    let mut heights = vec![height(&h)];
    let mut deployed_at = None;
    for frame in 0..200 {
        h.advance(1);
        heights.push(height(&h));
        if deployed_at.is_none() && has_aura(&h, PARACHUTE_AURA) {
            deployed_at = Some(frame + 1);
        }
        if height(&h) == 0.0 {
            break;
        }
    }
    let k = deployed_at.expect("269779 deployed before landing");
    assert!(heights[k - 2] > 30.0, "no deploy above 30 yd: {heights:?}");
    assert!(heights[k] <= 30.0, "deployed at {}", heights[k]);
    assert!(has_aura(&h, CLAIM_JUMPER_TRACKER) && has_aura(&h, DEATH_FROM_ABOVE_TRACKER));
    h.advance(10);
    assert_eq!(height(&h), 0.0);
    assert_eq!(applied(&h, PARACHUTE), 1, "the ground check deploys once");
    assert_eq!(applied(&h, PARACHUTE_AURA), 1);
    assert!(
        !has_aura(&h, ROCKET_PARACHUTE_2) && !has_aura(&h, ROCKET_PARACHUTE),
        "269776 effect164 removes250921, whose removal stops250917 ticks"
    );
    assert_eq!(h.health(h.caster), 1000.0);
    assert_eq!(falling_logs(&h), 0);
}

/// 250921 Effect0 removal removes 250917, so no tick re-applies 250921.
#[test]
fn seething_parachute_ground_check_removal_removes_rocket_parachute() {
    let mut h = falling_harness(200.0);
    disembark(&mut h);
    h.advance(6);
    assert!(has_aura(&h, ROCKET_PARACHUTE_2));
    let player = h.caster;
    crate::auras::remove_auras(h.app.world_mut(), player, RemoveReason::Cancelled, |aura| {
        aura.spell_id == ROCKET_PARACHUTE_2
    });
    assert!(!has_aura(&h, ROCKET_PARACHUTE));
    h.advance(10);
    assert!(!has_aura(&h, ROCKET_PARACHUTE_2) && !has_aura(&h, ROCKET_PARACHUTE));
}

/// 269779 Effect1 `SetFallInformation(0, GetPositionZ())`: the landing measures the
/// fall from the deploy height while retaining the source default upward knockback.
#[test]
fn seething_parachute_resets_authoritative_fall_at_deploy_height() {
    let mut h = falling_harness(60.0);
    while height(&h) > 25.0 {
        h.advance(1);
    }
    let deploy = height(&h);
    let player = h.caster;
    crate::spell_cast::cast_triggered(h.app.world_mut(), player, PARACHUTE, Some(player), 0);
    assert!(has_aura(&h, PARACHUTE_AURA));
    assert!(
        h.app
            .world()
            .get::<MovementControl>(player)
            .unwrap()
            .controlled
    );
    let mut peak = deploy;
    for _ in 0..200 {
        h.advance(1);
        peak = peak.max(height(&h));
        if height(&h) == 0.0 {
            break;
        }
    }
    assert!(
        peak > deploy,
        "source default effect98 produces its upward hop"
    );
    assert_eq!(height(&h), 0.0);
    h.advance(2);
    let expected = 1000.0 * (1.0 - shared::movement::fall_damage_percent(peak));
    assert!(
        (h.health(player) - expected).abs() < 0.5,
        "health {} expected {expected} from {deploy}",
        h.health(player)
    );
}

/// `damage *= GetTotalAuraMultiplier(SPELL_AURA_MODIFY_FALL_DAMAGE_PCT)` on the gravity
/// and the knockback landing; 262359 is -100.
#[test]
fn seething_no_falling_damage_aura_negates_gravity_and_knockback_landings() {
    for shielded in [false, true] {
        let mut h = falling_harness(40.0);
        let player = h.caster;
        if shielded {
            crate::spell_cast::cast_triggered(
                h.app.world_mut(),
                player,
                NO_FALLING_DAMAGE,
                Some(player),
                0,
            );
        }
        while height(&h) > 0.0 {
            h.advance(1);
        }
        h.advance(2);
        let expected = if shielded {
            1000.0
        } else {
            1000.0 * (1.0 - shared::movement::fall_damage_percent(40.0))
        };
        assert!(
            (h.health(player) - expected).abs() < 0.5,
            "gravity shielded={shielded}: {}",
            h.health(player)
        );
        assert_eq!(falling_logs(&h), usize::from(!shielded));

        let los_dir = crate::ground::ground_data_dir().join("los");
        h.app
            .insert_resource(crate::line_of_sight::LineOfSight::load(&los_dir).unwrap());
        let before = h.health(player);
        h.app.world_mut().get_mut::<Position>(player).unwrap().y = 30.0;
        let origin = Vec3::new(5.0, 30.0, -5.0);
        crate::knockback::launch(h.app.world_mut(), player, origin, 0.0, 0.0).unwrap();
        for _ in 0..60 {
            h.advance(1);
        }
        assert_eq!(height(&h), 0.0);
        let knockback_damage = before - h.health(player);
        if shielded {
            assert_eq!(knockback_damage, 0.0);
        } else {
            assert!(knockback_damage > 0.0);
        }
    }
}
