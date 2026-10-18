//! Native55224 separates a 1.1s prepare from its real 8s channel and target stun.
use shared::casting::{CastState, CastType};
use shared::death::DeathState;
use shared::protocol::{DeathChannel, DeathStateUpdate, ReleaseSpirit};
use shared::spell_data::CastFailReason;

use super::test_harness::*;
use crate::auras::has_aura;
use crate::death::{DeathPlugin, PlayerDeathState};

const DEATH_TOUCH: u32 = 5;
const ARCHIVIST_SCAN: u32 = 55224;
const FROSTBOLT: u32 = 116;
const FROSTBOLT_DAMAGE: u32 = 228597;

fn native_fixture(world_data: Option<crate::world_data::WorldData>) -> Harness {
    let mut h = harness_with_plugins(
        Vec::new(),
        &[DEATH_TOUCH, ARCHIVIST_SCAN, FROSTBOLT, FROSTBOLT_DAMAGE],
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
        .insert(crate::persistence::ClientId(74_002));
    h.app.world_mut().entity_mut(h.connection).insert((
        lightyear::prelude::MessageSender::<DeathStateUpdate>::default(),
        lightyear::prelude::MessageReceiver::<DeathStateUpdate>::default(),
        lightyear::prelude::MessageSender::<ReleaseSpirit>::default(),
        lightyear::prelude::MessageReceiver::<ReleaseSpirit>::default(),
    ));
    h
}

fn kill_player(h: &mut Harness, triggered: bool) {
    super::creature::creature_cast(
        h.app.world_mut(),
        h.npc,
        DEATH_TOUCH,
        Some(h.caster),
        triggered,
    )
    .unwrap();
    h.advance(3);
    assert_eq!(h.health(h.caster), 0.0);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerDeathState>(h.caster)
            .unwrap()
            .state,
        DeathState::Dead
    );
}

fn start_native_channel(h: &mut Harness) {
    super::creature::creature_cast(
        h.app.world_mut(),
        h.caster,
        ARCHIVIST_SCAN,
        Some(h.npc),
        false,
    )
    .unwrap();
    let preparing = h.app.world().get::<CastState>(h.caster).unwrap();
    assert_eq!(preparing.cast_type, CastType::Normal);
    assert_eq!(preparing.duration, 1.1);
    h.advance(30);
    let channel = h
        .app
        .world()
        .get::<CastState>(h.caster)
        .expect("native prepare becomes channel");
    assert_eq!(channel.spell_id, ARCHIVIST_SCAN);
    assert_eq!(channel.cast_type, CastType::Channel);
    assert_eq!(channel.duration, 8.0);
    assert!(has_aura(h.app.world(), h.npc, ARCHIVIST_SCAN));
    let failure =
        super::creature::creature_cast(h.app.world_mut(), h.npc, FROSTBOLT, Some(h.caster), false)
            .expect_err("the native channel's stun must block an ordinary target cast");
    assert_eq!(failure.to_contract().unwrap().0, CastFailReason::Stunned);
}

#[test]
fn native_dead_start_channel_applies_actual_stun_and_completes_naturally() {
    let mut h = native_fixture(None);
    kill_player(&mut h, false);
    start_native_channel(&mut h);
    assert_eq!(h.health(h.caster), 0.0);
    h.advance(180);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    assert!(!has_aura(h.app.world(), h.npc, ARCHIVIST_SCAN));
    assert_eq!(h.health(h.caster), 0.0);
}

#[test]
fn native_caster_death_during_channel_removes_its_actual_target_stun() {
    let mut h = native_fixture(None);
    start_native_channel(&mut h);
    // The stunned NPC uses the actual bool-FULL_MASK API, not an edited spell flag.
    kill_player(&mut h, true);
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    assert!(!has_aura(h.app.world(), h.npc, ARCHIVIST_SCAN));
}

#[test]
fn native_successful_repop_interrupts_dead_start_channel_and_target_stun() {
    let data = prepare_class_world().world_data();
    let mut h = native_fixture(Some(data));
    kill_player(&mut h, false);
    assert!(
        h.app
            .world()
            .get::<PlayerDeathState>(h.caster)
            .unwrap()
            .graveyard
            .is_some()
    );
    start_native_channel(&mut h);
    h.app
        .world_mut()
        .get_mut::<lightyear::prelude::MessageSender<ReleaseSpirit>>(h.connection)
        .unwrap()
        .send::<DeathChannel>(ReleaseSpirit);
    h.advance(3);
    assert_eq!(
        h.app
            .world()
            .get::<PlayerDeathState>(h.caster)
            .unwrap()
            .state,
        DeathState::Ghost
    );
    assert!(h.app.world().get::<CastState>(h.caster).is_none());
    assert!(!has_aura(h.app.world(), h.npc, ARCHIVIST_SCAN));
}
