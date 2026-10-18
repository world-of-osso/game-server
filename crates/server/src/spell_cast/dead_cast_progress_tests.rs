//! Native cast-time spells distinguish dead-start progress from a later death transition.
use bevy::prelude::Entity;
use shared::casting::CastState;
use shared::death::DeathState;
use shared::protocol::{CombatLogKind, DeathChannel, DeathStateUpdate, ReleaseSpirit};

use super::test_harness::*;
use crate::death::{DeathPlugin, PlayerDeathState};

const DEATH_TOUCH: u32 = 5;
const NETHER_EXPLOSION: u32 = 35058;
const GLYPH_OF_THORNS: u32 = 53008;

fn native_fixture() -> Harness {
    native_fixture_with_world_data(None)
}

fn native_fixture_with_world_data(world_data: Option<crate::world_data::WorldData>) -> Harness {
    let mut h = harness_with_plugins(
        Vec::new(),
        &[DEATH_TOUCH, NETHER_EXPLOSION, GLYPH_OF_THORNS],
        2,
        false,
        move |app| {
            app.add_plugins(DeathPlugin);
            if let Some(world_data) = world_data {
                app.insert_resource(world_data);
            }
        },
    )
    .unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::persistence::ClientId(74_001));
    h.app.world_mut().entity_mut(h.connection).insert((
        lightyear::prelude::MessageSender::<DeathStateUpdate>::default(),
        lightyear::prelude::MessageReceiver::<DeathStateUpdate>::default(),
    ));
    h
}

fn kill_player(h: &mut Harness) {
    super::creature::creature_cast(h.app.world_mut(), h.npc, DEATH_TOUCH, Some(h.caster), false)
        .unwrap();
    h.advance(2);
    assert_eq!(h.health(h.caster), 0.0);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerDeathState>(h.caster)
            .unwrap()
            .state,
        DeathState::Dead,
    );
}

fn request_release_spirit(h: &mut Harness) {
    h.app.world_mut().entity_mut(h.connection).insert((
        lightyear::prelude::MessageSender::<ReleaseSpirit>::default(),
        lightyear::prelude::MessageReceiver::<ReleaseSpirit>::default(),
    ));
    h.app
        .world_mut()
        .get_mut::<lightyear::prelude::MessageSender<ReleaseSpirit>>(h.connection)
        .unwrap()
        .send::<DeathChannel>(ReleaseSpirit);
    h.advance(2);
}

fn kill_npc(h: &mut Harness) {
    super::creature::creature_cast(h.app.world_mut(), h.caster, DEATH_TOUCH, Some(h.npc), false)
        .unwrap();
    assert_eq!(h.health(h.npc), 0.0);
    assert!(h.app.world().get::<crate::combat::Corpse>(h.npc).is_some());
}

fn begin_cast(h: &mut Harness, caster: Entity, spell: u32, duration: f32) {
    super::creature::creature_cast(h.app.world_mut(), caster, spell, None, false).unwrap();
    let cast = h.app.world().get::<CastState>(caster).unwrap();
    assert_eq!(cast.spell_id, spell);
    assert_eq!(cast.duration, duration);
}

fn assert_success(h: &Harness, spell: u32) {
    assert!(h.inbox.log.iter().any(|event| {
        event.kind == CombatLogKind::CastSuccess && event.spell_id == Some(spell)
    }));
}

#[test]
fn native_dead_player_active_cast_advances_and_completes_with_real_damage() {
    let mut h = native_fixture();
    kill_player(&mut h);
    let before = h.health(h.npc);
    let caster = h.caster;
    begin_cast(&mut h, caster, NETHER_EXPLOSION, 1.0);
    h.advance(1);
    let cast = h
        .app
        .world()
        .get::<CastState>(caster)
        .expect("eligible dead-start cast progresses");
    assert!(cast.elapsed > 0.0 && cast.elapsed < cast.duration);
    h.advance(25);
    assert_success(&h, NETHER_EXPLOSION);
    assert!(h.health(h.npc) < before);
    assert_eq!(h.health(caster), 0.0);
    assert!(h.app.world().get::<CastState>(caster).is_none());
}

#[test]
fn native_dead_player_passive_cast_completes_without_death_persistent_aura() {
    let mut h = native_fixture();
    kill_player(&mut h);
    let caster = h.caster;
    begin_cast(&mut h, caster, GLYPH_OF_THORNS, 3.0);
    h.advance(1);
    assert!(h.app.world().get::<CastState>(caster).is_some());
    h.advance(70);
    assert_success(&h, GLYPH_OF_THORNS);
    assert_eq!(h.health(caster), 0.0);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        caster,
        GLYPH_OF_THORNS
    ));
    assert!(h.app.world().get::<CastState>(caster).is_none());
}

#[test]
fn native_living_player_active_cast_still_completes_with_real_damage() {
    let mut h = native_fixture();
    let before = h.health(h.npc);
    let caster = h.caster;
    begin_cast(&mut h, caster, NETHER_EXPLOSION, 1.0);
    h.advance(25);
    assert_success(&h, NETHER_EXPLOSION);
    assert!(h.health(h.npc) < before);
}

#[test]
fn native_player_dying_during_flagged_cast_still_interrupts_before_completion() {
    let mut h = native_fixture();
    let before = h.health(h.npc);
    let caster = h.caster;
    begin_cast(&mut h, caster, NETHER_EXPLOSION, 1.0);
    kill_player(&mut h);
    assert!(h.app.world().get::<CastState>(caster).is_none());
    h.advance(25);
    assert!(!h.inbox.log.iter().any(|event| {
        event.kind == CombatLogKind::CastSuccess && event.spell_id == Some(NETHER_EXPLOSION)
    }));
    assert_eq!(h.health(h.npc), before);
}

#[test]
fn native_npc_death_interrupts_flagged_cast_before_death_touch_returns() {
    let mut h = native_fixture();
    let caster = h.npc;
    begin_cast(&mut h, caster, NETHER_EXPLOSION, 1.0);
    kill_npc(&mut h);
    assert!(h.app.world().get::<CastState>(caster).is_none());
}

#[test]
fn native_npc_forced_despawn_interrupts_flagged_cast_immediately() {
    let mut h = native_fixture();
    let caster = h.npc;
    begin_cast(&mut h, caster, NETHER_EXPLOSION, 1.0);
    crate::combat::despawn_creature(h.app.world_mut(), caster);
    assert_eq!(h.health(caster), 0.0);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(caster)
            .is_some()
    );
    assert!(h.app.world().get::<CastState>(caster).is_none());
}

#[test]
fn native_npc_corpse_removal_interrupts_an_eligible_dead_start_cast() {
    let mut h = native_fixture();
    kill_npc(&mut h);
    let caster = h.npc;
    let before = h.health(h.caster);
    begin_cast(&mut h, caster, NETHER_EXPLOSION, 1.0);
    h.advance(1);
    assert!(h.app.world().get::<crate::combat::Corpse>(caster).is_none());
    assert!(h.app.world().get::<CastState>(caster).is_none());
    h.advance(25);
    assert_eq!(h.health(h.caster), before);
}

#[test]
fn native_successful_spirit_release_interrupts_a_dead_start_cast() {
    let data = prepare_class_world().world_data();
    let mut h = native_fixture_with_world_data(Some(data));
    kill_player(&mut h);
    assert!(
        h.app
            .world()
            .get::<PlayerDeathState>(h.caster)
            .unwrap()
            .graveyard
            .is_some()
    );
    let caster = h.caster;
    begin_cast(&mut h, caster, NETHER_EXPLOSION, 1.0);
    request_release_spirit(&mut h);
    assert_eq!(
        h.app.world().get::<PlayerDeathState>(caster).unwrap().state,
        DeathState::Ghost
    );
    assert!(h.app.world().get::<CastState>(caster).is_none());
    h.advance(25);
    assert!(!h.inbox.log.iter().any(|event| {
        event.kind == CombatLogKind::CastSuccess && event.spell_id == Some(NETHER_EXPLOSION)
    }));
}

#[test]
fn native_failed_spirit_release_preserves_an_eligible_dead_start_cast() {
    let mut h = native_fixture();
    kill_player(&mut h);
    assert!(
        h.app
            .world()
            .get::<PlayerDeathState>(h.caster)
            .unwrap()
            .graveyard
            .is_none()
    );
    let caster = h.caster;
    begin_cast(&mut h, caster, NETHER_EXPLOSION, 1.0);
    request_release_spirit(&mut h);
    assert_eq!(
        h.app.world().get::<PlayerDeathState>(caster).unwrap().state,
        DeathState::Dead
    );
    assert!(h.app.world().get::<CastState>(caster).is_some());
    h.advance(25);
    assert_success(&h, NETHER_EXPLOSION);
}

#[test]
fn native_npc_dead_phase_does_not_update_a_new_spell_event() {
    let mut h = native_fixture();
    kill_npc(&mut h);
    h.advance(1);
    let caster = h.npc;
    assert!(h.app.world().get::<crate::combat::Corpse>(caster).is_none());
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(caster)
            .is_some()
    );
    let before = h.health(h.caster);
    begin_cast(&mut h, caster, NETHER_EXPLOSION, 1.0);
    h.advance(25);
    let cast = h
        .app
        .world()
        .get::<CastState>(caster)
        .expect("DEAD actor does not run Unit events");
    assert_eq!(cast.elapsed, 0.0);
    assert_eq!(h.health(h.caster), before);
}
