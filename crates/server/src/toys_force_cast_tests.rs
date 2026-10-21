//! TC a352b1fa SpellEffects.cpp851–892: effect160 queues a normal target→caster cast.
use super::*;
use crate::auras::{ActiveAuras, AuraApplication, AuraSource};
use shared::components::{CastState, Health, ModelDisplay};

const HITCHING_POST_USE: u32 = 97401;
const HITCHING_POST_MOUNT: u32 = 97390;
const SPORE_ITEM: u32 = 203852;
const SPORE_USE: u32 = 402642;
const SPORE_TRANSFORM: u32 = 402636;

#[test]
fn toy_coverage_force160_native_target_cast_waits_for_its_own_cast_time() {
    let mut h = harness(Vec::new(), &[HITCHING_POST_USE, HITCHING_POST_MOUNT], 8).unwrap();
    let player = h.caster;
    let npc = h.npc;
    crate::spell_cast::creature::creature_cast(
        h.app.world_mut(),
        npc,
        HITCHING_POST_USE,
        Some(player),
        false,
    )
    .unwrap();
    assert!(h.app.world().get::<CastState>(player).is_none());
    assert!(!crate::auras::has_aura(
        h.app.world(),
        player,
        HITCHING_POST_MOUNT
    ));
    h.advance(1);
    let cast = h
        .app
        .world()
        .get::<CastState>(player)
        .expect("forced player starts normal source cast");
    assert_eq!(cast.spell_id, HITCHING_POST_MOUNT);
    assert_eq!(cast.duration, 1.5);
    assert!(h.app.world().get::<CastState>(npc).is_none());
    h.advance(28);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        player,
        HITCHING_POST_MOUNT
    ));
    h.advance(4);
    assert!(crate::auras::has_aura(
        h.app.world(),
        player,
        HITCHING_POST_MOUNT
    ));
    assert!(!crate::auras::has_aura(
        h.app.world(),
        npc,
        HITCHING_POST_MOUNT
    ));
    assert!(
        h.inbox
            .go
            .iter()
            .any(|go| go.spell_id == HITCHING_POST_MOUNT && go.caster == player.to_bits())
    );
}

#[test]
fn toy_coverage_force160_native_target_stun_blocks_the_normal_child() {
    let mut h = harness(
        Vec::new(),
        &[HITCHING_POST_USE, HITCHING_POST_MOUNT, 853],
        8,
    )
    .unwrap();
    let player = h.caster;
    let npc = h.npc;
    crate::auras::apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: npc,
            target: player,
            spell_id: 853,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    crate::spell_cast::creature::creature_cast(
        h.app.world_mut(),
        npc,
        HITCHING_POST_USE,
        Some(player),
        false,
    )
    .unwrap();
    h.advance(35);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        player,
        HITCHING_POST_MOUNT
    ));
    assert!(h.app.world().get::<CastState>(player).is_none());
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|go| go.spell_id == HITCHING_POST_MOUNT)
    );
}

#[test]
fn toy_coverage_force160_spore_actual_use_waits_two_seconds_then_transforms_and_expires() {
    let dir = tempfile::tempdir().unwrap();
    let (mut h, character) = setup(&dir.path().join("spore.redb"));
    let path = crate::test_data::world_db_path();
    let conn =
        rusqlite::Connection::open_with_flags(&path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    h.app.insert_resource(toy_catalog::load(&conn).unwrap());
    h.app.insert_resource(shared_item_templates(&path));
    h.app.insert_resource(shared_world_data(&path));
    h.app
        .insert_resource(crate::spell_info::load_spell_data(&conn).unwrap().0);
    let player = h.caster;
    let health = *h.app.world().get::<Health>(player).unwrap();
    let original = h.app.world().get::<ModelDisplay>(player).copied();
    learn_effect_toy(&mut h, character, SPORE_ITEM);
    assert_eq!(activate(&mut h, SPORE_ITEM)[0].error, None);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == SPORE_USE));
    assert_eq!(h.app.world().get::<ModelDisplay>(player).copied(), original);
    assert_eq!(
        h.app.world().get::<CastState>(player).unwrap().duration,
        2.0
    );
    assert!(!h.inbox.go.iter().any(|go| go.spell_id == SPORE_TRANSFORM));
    h.advance(25);
    assert_eq!(h.app.world().get::<ModelDisplay>(player).copied(), original);
    h.advance(20);
    assert_eq!(
        h.app
            .world()
            .get::<ModelDisplay>(player)
            .unwrap()
            .display_id,
        88121
    );
    assert!(h.inbox.go.iter().any(|go| go.spell_id == SPORE_TRANSFORM));
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(player)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == SPORE_TRANSFORM)
        .unwrap();
    assert_eq!(aura.duration_ms, 300_000);
    h.advance(5960);
    assert!(crate::auras::has_aura(
        h.app.world(),
        player,
        SPORE_TRANSFORM
    ));
    h.advance(70);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        player,
        SPORE_TRANSFORM
    ));
    assert_eq!(h.app.world().get::<ModelDisplay>(player).copied(), original);
    assert_eq!(h.app.world().get::<Health>(player), Some(&health));
    assert_eq!(
        activate(&mut h, SPORE_ITEM)[0].error.as_deref(),
        Some("OnCooldown")
    );
}
