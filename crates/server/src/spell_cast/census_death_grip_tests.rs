//! TC a352b1fa spell_dk.cpp663-673; SpellEffects.cpp42/213 share EffectJumpDest.
//! Actual native child casts, reversed actor/target, real jump clock and Blood taunt.
use super::test_harness::{Harness, ObservedTriggeredCasts, harness, world_db};
use bevy::prelude::*;
use shared::components::{Position, Rotation};

const GRIP: u32 = 49576;
const VISUAL: u32 = 243912;
const JUMP: u32 = 49575;
const TAUNT: u32 = 51399;
const BLOOD: u32 = 137008;

fn fixture(blood: bool) -> Harness {
    let mut ids = vec![GRIP, VISUAL, JUMP, TAUNT, 5];
    if blood {
        ids.push(BLOOD);
    }
    let mut h = harness(Vec::new(), &ids, 6).unwrap();
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.app
        .insert_resource(crate::test_data::world_data().clone());
    h.app.insert_resource(ObservedTriggeredCasts::default());
    h.app.add_systems(
        FixedUpdate,
        (
            crate::scripted_ai::jump_movement::release_finished_jumps,
            crate::scripted_ai::jump_movement::advance_jumps,
        )
            .chain(),
    );
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: 15.0,
        y: 0.0,
        z: 0.0,
    });
    h.advance(3);
    assert_eq!(
        crate::auras::has_aura(h.app.world(), h.caster, BLOOD),
        blood
    );
    h
}

fn position(h: &Harness, unit: Entity) -> Position {
    *h.app.world().get::<Position>(unit).unwrap()
}

fn assert_jump_and_land(h: &mut Harness, mover: Entity, destination: Position) {
    h.advance(1);
    let during = position(h, mover);
    assert!(
        during.x > destination.x && during.x < 15.0,
        "actual moving jump, not instantaneous teleport:{during:?}"
    );
    assert!(during.y > destination.y, "actual parabolic jump:{during:?}");
    h.advance(80); // Native distance15/base-run-speed ×3 completes within4s, no clock writes.
    assert_eq!(position(h, mover), destination);
}

#[test]
#[ignore = "pushed native census regression before E213/Death Grip binding"]
fn native_census_death_grip_direct_native213_uses_destination_jump_clock() {
    let mut h = fixture(false);
    let origin = position(&h, h.caster);
    let mover = h.npc;
    let facing = 0.37;
    h.app.world_mut().entity_mut(mover).insert(Rotation {
        y: facing,
        ..default()
    });
    assert_ne!(
        super::attributes(world_db().unwrap().store.get(JUMP).unwrap(), 9) & 0x0080_0000,
        0,
        "native213 requests no facing control"
    );
    super::creature_cast(h.app.world_mut(), mover, JUMP, Some(h.caster), true)
        .expect("native213 aliases TC EffectJumpDest42, not unsupported or instant teleport");
    assert_jump_and_land(&mut h, mover, origin);
    assert_eq!(position(&h, h.caster), origin);
    assert_eq!(h.health(mover), 5000.0);
    assert_eq!(h.app.world().get::<Rotation>(mover).unwrap().y, facing);

    // Native4336 is effect42 with ordinary facing, not a fabricated alias row.
    h.app
        .world_mut()
        .entity_mut(mover)
        .insert(Position { x: 15.0, ..origin });
    super::cast_triggered_at_destination(
        h.app.world_mut(),
        mover,
        4336,
        Vec3::new(origin.x, origin.y, origin.z),
        0,
    );
    assert_jump_and_land(&mut h, mover, origin);
    assert_eq!(
        h.app.world().get::<Rotation>(mover).unwrap().y,
        crate::creature_ai::facing_along(-15.0, 0.0).unwrap()
    );
}

#[test]
#[ignore = "pushed native census regression before E213/Death Grip binding"]
fn native_census_death_grip_nonblood_launches_owned_visual_and_reversed_jump_without_taunt() {
    let mut h = fixture(false);
    let caster = h.caster;
    let victim = h.npc;
    let origin = position(&h, caster);
    h.cast_and_deliver(GRIP, Some(victim));
    assert!(
        h.inbox.failed.is_empty(),
        "source49576E77 must bind:{:?}",
        h.inbox.failed
    );
    let launches = &h.app.world().resource::<ObservedTriggeredCasts>().0;
    assert_eq!(
        launches.as_slice(),
        &[(caster, VISUAL, Some(victim)), (victim, JUMP, Some(caster))]
    );
    assert!(!crate::auras::has_aura(h.app.world(), victim, TAUNT));
    assert_jump_and_land(&mut h, victim, origin);
    assert_eq!(position(&h, caster), origin);
    assert_eq!(h.health(caster), 1000.0);
    assert_eq!(h.health(victim), 5000.0);
}

#[test]
#[ignore = "pushed native census regression before E213/Death Grip binding"]
fn native_census_death_grip_actual_blood_passive_adds_owned_native_taunt_after_jump_launch() {
    let mut h = fixture(true);
    let caster = h.caster;
    let victim = h.npc;
    h.cast_and_deliver(GRIP, Some(victim));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let launches = &h.app.world().resource::<ObservedTriggeredCasts>().0;
    assert_eq!(
        launches.as_slice(),
        &[
            (caster, VISUAL, Some(victim)),
            (victim, JUMP, Some(caster)),
            (caster, TAUNT, Some(victim))
        ]
    );
    let taunt = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(victim)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == TAUNT)
        .expect("real51399 taunt aura");
    assert_eq!(taunt.caster, caster);
    assert!(taunt.effects.iter().any(|effect| effect.aura_type == 11));
}

#[test]
#[ignore = "pushed native census regression before E213/Death Grip binding"]
fn native_census_death_grip_refused_range_cast_launches_no_children() {
    let mut h = fixture(false);
    h.app.world_mut().entity_mut(h.npc).insert(Position {
        x: 100.0,
        y: 0.0,
        z: 0.0,
    });
    h.cast_and_deliver(GRIP, Some(h.npc));
    assert!(!h.inbox.failed.is_empty());
    assert!(
        h.app
            .world()
            .resource::<ObservedTriggeredCasts>()
            .0
            .is_empty()
    );
    assert_eq!(position(&h, h.npc).x, 100.0);
}

#[test]
#[ignore = "pushed native census regression before E213/Death Grip binding"]
fn native_census_death_grip_real_native_death_refuses_parent_without_children() {
    let mut h = fixture(false);
    super::creature_cast(h.app.world_mut(), h.caster, 5, Some(h.npc), true).unwrap();
    h.advance(3);
    assert_eq!(h.health(h.npc), 0.0);
    h.app
        .world_mut()
        .resource_mut::<ObservedTriggeredCasts>()
        .0
        .clear();
    h.cast_and_deliver(GRIP, Some(h.npc));
    assert!(!h.inbox.failed.is_empty());
    assert!(
        h.app
            .world()
            .resource::<ObservedTriggeredCasts>()
            .0
            .is_empty()
    );
}
