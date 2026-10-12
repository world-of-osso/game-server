//! Real source58873, fixed-tick lifetime, owner input and landing.
use super::test_harness::*;
use bevy::prelude::*;
use lightyear::prelude::*;
use shared::components::{MovementControl, MovementSpeed, PlayerMotion, Position, Rotation};
use shared::protocol::{InputChannel, PlayerInput};

fn source_push_harness(spells: &[u32]) -> Harness {
    let mut h = harness_with_plugins(Vec::new(), spells, 1, false, |app| {
        app.init_resource::<crate::friends::AutoAfkScheduler>();
        app.add_systems(FixedUpdate, crate::networking::process_player_inputs);
    })
    .unwrap();
    h.app
        .insert_resource(crate::terrain_height::test_helpers::flat_ground(
            33.0, 33.0, 0.0,
        ));
    let los_dir = crate::ground::ground_data_dir().join("los");
    h.app
        .insert_resource(crate::line_of_sight::LineOfSight::load(&los_dir).unwrap());
    h.app.world_mut().entity_mut(h.caster).insert((
        Position {
            x: 5.0,
            y: 0.0,
            z: -5.0,
        },
        Rotation {
            x: 0.0,
            y: 0.0,
            z: 0.0,
        },
        MovementSpeed(0.0),
        PlayerMotion::default(),
        crate::networking::FallTracker::grounded(0.0),
    ));
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: 3.0,
        y: 0.0,
        z: -5.0,
    });
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<PlayerInput>::default(),
        MessageReceiver::<PlayerInput>::default(),
    ));
    h
}

#[test]
fn arena_knockback_source_effect_drives_ballistic_player_motion_and_rejects_owner_input() {
    let mut h = source_push_harness(&[58873]);
    let before = *h.app.world().get::<Position>(h.caster).unwrap();
    super::cast_triggered(h.app.world_mut(), h.npc, 58873, None, 0);
    h.app
        .world_mut()
        .get_mut::<MessageSender<PlayerInput>>(h.connection)
        .unwrap()
        .send::<InputChannel>(PlayerInput {
            direction: [1.0, 0.0, 0.0],
            facing_yaw: 0.0,
            jumping: false,
            running: true,
            swimming: false,
            flying: false,
            position: [1000.0, 1000.0, 1000.0],
            epoch: 0,
        });
    h.advance(1);
    let control = *h.app.world().get::<MovementControl>(h.caster).unwrap();
    assert!(
        control.controlled,
        "source knockback takes movement ownership"
    );
    assert_eq!(control.epoch, 1);
    let airborne = *h.app.world().get::<Position>(h.caster).unwrap();
    assert!(
        (airborne.x - before.x - 1.0).abs() < 0.001,
        "20yd/s from source MiscValue200"
    );
    let expected_y = 3.5 * 0.05 - 0.5 * shared::skyriding::GRAVITY * 0.05 * 0.05;
    assert!(
        (airborne.y - expected_y).abs() < 0.001,
        "source BasePoints35 upward3.5yd/s"
    );
    assert_eq!(airborne.z, before.z);
    h.advance(20);
    let landed = *h.app.world().get::<Position>(h.caster).unwrap();
    assert_eq!(landed.y, 0.0);
    assert!(landed.x > 12.0 && landed.x <= 13.1);
    let control = *h.app.world().get::<MovementControl>(h.caster).unwrap();
    assert!(!control.controlled);
    assert_eq!(control.epoch, 2, "landing invalidates pre-launch reports");
    h.app
        .world_mut()
        .get_mut::<MessageSender<PlayerInput>>(h.connection)
        .unwrap()
        .send::<InputChannel>(PlayerInput {
            direction: [1.0, 0.0, 0.0],
            facing_yaw: 0.0,
            jumping: false,
            running: true,
            swimming: false,
            flying: false,
            position: [1000.0, 1000.0, 1000.0],
            epoch: 1,
        });
    h.advance(2);
    assert_eq!(
        *h.app.world().get::<Position>(h.caster).unwrap(),
        landed,
        "stale in-flight owner reports never undo landing"
    );
}

#[test]
fn arena_knockback_source_root_and_area_conditions_prevent_invalid_pushes() {
    let mut h = source_push_harness(&[58873, 339]);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.npc,
            target: h.caster,
            spell_id: 339,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    let before = *h.app.world().get::<Position>(h.caster).unwrap();
    super::cast_triggered(h.app.world_mut(), h.npc, 58873, None, 0);
    h.advance(2);
    assert_eq!(*h.app.world().get::<Position>(h.caster).unwrap(), before);
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Expired,
        |aura| aura.spell_id == 339,
    );
    let other_copy = spawn_player(&mut h.app, 1, 6.0);
    h.app.world_mut().entity_mut(other_copy).insert((
        Position {
            x: 6.0,
            y: 0.0,
            z: -5.0,
        },
        crate::maps::InMap {
            map_id: 0,
            instance_id: 7,
        },
    ));
    let far = spawn_player(&mut h.app, 1, 30.0);
    h.app.world_mut().entity_mut(far).insert(Position {
        x: 30.0,
        y: 0.0,
        z: -5.0,
    });
    super::cast_triggered(h.app.world_mut(), h.npc, 58873, None, 0);
    h.advance(1);
    assert!(
        h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled
    );
    for unit in [other_copy, far] {
        assert!(
            !h.app
                .world()
                .get::<MovementControl>(unit)
                .unwrap()
                .controlled,
            "source area does not leak across copy/radius"
        );
    }
}

#[test]
fn arena_knockback_transfer_cancels_its_lease_but_never_cancels_a_newer_owner() {
    let mut h = source_push_harness(&[58873]);
    super::cast_triggered(h.app.world_mut(), h.npc, 58873, None, 0);
    h.advance(1);
    let before = *h.app.world().get::<Position>(h.caster).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .remove::<crate::maps::InMap>();
    h.advance(1);
    assert_eq!(*h.app.world().get::<Position>(h.caster).unwrap(), before);
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled
    );
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::maps::InMap::continent(0));
    super::cast_triggered(h.app.world_mut(), h.npc, 58873, None, 0);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(MovementControl {
            epoch: 100,
            controlled: true,
        });
    h.advance(1);
    assert_eq!(
        *h.app.world().get::<MovementControl>(h.caster).unwrap(),
        MovementControl {
            epoch: 100,
            controlled: true
        }
    );
    assert_eq!(*h.app.world().get::<Position>(h.caster).unwrap(), before);
}

#[test]
fn arena_knockback_landing_preserves_realm_fall_damage_and_actual_feather_fall() {
    for feather in [false, true] {
        let mut h = source_push_harness(&[58873, 130]);
        h.app.world_mut().get_mut::<Position>(h.caster).unwrap().y = 5.0;
        h.app.world_mut().get_mut::<Position>(h.npc).unwrap().y = 5.0;
        if feather {
            crate::auras::apply_aura(
                h.app.world_mut(),
                &crate::auras::AuraApplication {
                    caster: h.caster,
                    target: h.caster,
                    spell_id: 130,
                    effect_mask: u32::MAX,
                    source: crate::auras::AuraSource::Cast,
                },
            );
        }
        let initial = h.health(h.caster);
        let mut peak = 5.0f32;
        super::cast_triggered(h.app.world_mut(), h.npc, 58873, None, 0);
        for _ in 0..40 {
            h.advance(1);
            peak = peak.max(h.app.world().get::<Position>(h.caster).unwrap().y);
            if !h
                .app
                .world()
                .get::<MovementControl>(h.caster)
                .unwrap()
                .controlled
            {
                break;
            }
        }
        assert_eq!(h.app.world().get::<Position>(h.caster).unwrap().y, 0.0);
        let damage = initial - h.health(h.caster);
        if feather {
            assert_eq!(damage, 0.0);
        } else {
            let expected = initial * shared::movement::fall_damage_percent(peak);
            assert!(
                (damage - expected).abs() < 0.01,
                "observed peak feeds the existing realm fall rule"
            );
            assert!(damage > 0.0);
        }
    }
}
