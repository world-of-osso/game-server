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
fn arena_knockback_stunned_target_still_receives_source_push() {
    let mut h = source_push_harness(&[58873, 853]);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.npc,
            target: h.caster,
            spell_id: 853,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(h.caster)
            .unwrap()
            .is_stunned()
    );
    super::cast_triggered(h.app.world_mut(), h.npc, 58873, None, 0);
    h.advance(1);
    assert!(
        h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled
    );
    assert!(h.app.world().get::<Position>(h.caster).unwrap().x > 5.0);
}

#[test]
fn arena_knockback_negative_source_npc_speed_is_upward_and_reverses_horizontal_direction() {
    let mut h = source_push_harness(&[252751]);
    h.app.insert_resource(crate::world_data::load_world_data(
        &crate::test_data::world_db_path(),
    ));
    h.app.world_mut().entity_mut(h.npc).insert(Rotation {
        x: 0.0,
        y: 0.0,
        z: 0.0,
    });
    let before = *h.app.world().get::<Position>(h.npc).unwrap();
    super::cast_triggered(h.app.world_mut(), h.npc, 252751, Some(h.npc), 0);
    h.advance(1);
    let after = *h.app.world().get::<Position>(h.npc).unwrap();
    assert!(
        after.y > before.y,
        "TC MotionMaster takes abs of NPC vertical speed"
    );
    assert!(
        (after.z - before.z - 2.5).abs() < 0.001,
        "source -50yd/s reverses the coincident facing direction"
    );
}

#[test]
fn arena_knockback_coincident_origin_uses_target_facing_plus_pi() {
    let mut h = source_push_harness(&[58873]);
    let before = *h.app.world().get::<Position>(h.caster).unwrap();
    h.app.world_mut().entity_mut(h.npc).insert(before);
    super::cast_triggered(h.app.world_mut(), h.npc, 58873, None, 0);
    h.advance(1);
    let after = *h.app.world().get::<Position>(h.caster).unwrap();
    assert_eq!(after.x, before.x);
    assert!((after.z - before.z + 1.0).abs() < 0.001);
}

#[test]
fn arena_knockback_source_destination_144_uses_destination_and_requires_it() {
    let mut h = source_push_harness(&[781]);
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(781)
        .unwrap()
        .clone();
    let effect = spell
        .effects
        .iter()
        .find(|effect| effect.effect == 144)
        .unwrap();
    let before = *h.app.world().get::<Position>(h.caster).unwrap();
    let mut ctx = super::effects::EffectCtx {
        caster: h.caster,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    super::knockback::knock_back(h.app.world_mut(), &ctx, effect, h.caster);
    assert!(
        !h.app
            .world()
            .get::<MovementControl>(h.caster)
            .unwrap()
            .controlled
    );
    ctx.dest = Some(super::targets::Dest {
        position: Vec3::new(7.0, 0.0, -5.0),
        orientation: 0.0,
    });
    super::knockback::knock_back(h.app.world_mut(), &ctx, effect, h.caster);
    h.advance(1);
    let after = *h.app.world().get::<Position>(h.caster).unwrap();
    assert!((after.x - before.x + 1.0).abs() < 0.001);
    assert!(after.y > before.y);
}

#[test]
fn arena_knockback_source_boss_and_immunity_flags_reject_player_pushes() {
    let mut h = source_push_harness(&[58873]);
    h.app.insert_resource(crate::world_data::load_world_data(
        &crate::test_data::world_db_path(),
    ));
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(58873)
        .unwrap()
        .clone();
    let ctx = super::effects::EffectCtx {
        caster: h.caster,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let effect = &spell.effects[0];
    let before = *h.app.world().get::<Position>(h.npc).unwrap();
    for entry in [10340, 2748, 929] {
        h.app
            .world_mut()
            .get_mut::<shared::components::Npc>(h.npc)
            .unwrap()
            .template_id = entry;
        super::knockback::knock_back(h.app.world_mut(), &ctx, effect, h.npc);
        h.advance(1);
        assert_eq!(
            *h.app.world().get::<Position>(h.npc).unwrap(),
            before,
            "source entry{entry}"
        );
        assert!(
            !h.app
                .world()
                .get::<MovementControl>(h.npc)
                .is_some_and(|control| control.controlled)
        );
    }
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
