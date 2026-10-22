//! Spec73 missing producers, pinned TC a352b1fa plus native70448 rows.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::Position;
use shared::protocol::{CombatLogKind, EquipmentSlot};

const REND: u32 = 772;
const REND_BLEED: u32 = 388_539;
const THUNDER_CLAP: u32 = 6343;
const SHIELD_CHARGE: u32 = 385_952;
const SHIELD_CHARGE_MOVEMENT: u32 = 385_953;
const SHIELD_CHARGE_DAMAGE: u32 = 385_954;

fn fixture(rend: bool) -> Harness {
    let ids = [
        THUNDER_CLAP,
        REND_BLEED,
        435203,
        SHIELD_CHARGE,
        SHIELD_CHARGE_MOVEMENT,
        SHIELD_CHARGE_DAMAGE,
    ];
    let mut h = harness(Vec::new(), &ids, 1).unwrap();
    h.app.insert_resource(world_db().unwrap().store.clone());
    let mut known = ids.to_vec();
    if rend {
        known.push(REND);
    }
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(known));
    h.app.insert_resource(
        crate::item_templates::load_item_templates(&crate::test_data::world_db_path()).unwrap(),
    );
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::stats::EquippedItems {
            items: vec![(
                EquipmentSlot::OffHand,
                shared::item_data::ItemDefinitionSource::Retail,
                2362,
            )],
        });
    h.advance(3);
    h.inbox = Inbox::default();
    h
}

fn clap(rend: bool) -> (Harness, Vec<Entity>) {
    // Whole native cast attempts retain actual avoidance. A hit is required before
    // inspecting its producer, never manufactured with hit flags or modified RNG.
    for _ in 0..8 {
        let mut h = fixture(rend);
        let near = spawn_npc(&mut h.app, 5.0);
        let far = spawn_npc(&mut h.app, 25.0);
        let ally = spawn_player(&mut h.app, 1, 4.0);
        h.cast_and_deliver(THUNDER_CLAP, None);
        h.advance(5);
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        if h.inbox.log.iter().any(|event| {
            event.spell_id == Some(THUNDER_CLAP)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        }) {
            assert!(!crate::auras::has_aura(h.app.world(), far, REND_BLEED));
            assert!(!crate::auras::has_aura(h.app.world(), ally, REND_BLEED));
            return (h, vec![near, far, ally]);
        }
    }
    panic!("eight native Thunder Clap casts produced no landed damage");
}

#[test]
fn protection_producers_tc_thunder_clap_emits_owned_rend_on_each_landed_enemy() {
    let (mut h, _) = clap(true);
    let landed: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(THUNDER_CLAP)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .map(|event| Entity::try_from_bits(event.target.unwrap()).unwrap())
        .collect();
    assert!(!landed.is_empty());
    for target in &landed {
        assert!(
            crate::auras::has_aura(h.app.world(), *target, REND_BLEED),
            "actual native Clap hit must produce its Rend child"
        );
        assert!(crate::auras::has_aura(h.app.world(), *target, 435203));
        let aura = h
            .app
            .world()
            .get::<crate::auras::ActiveAuras>(*target)
            .unwrap()
            .iter()
            .find(|aura| aura.spell_id == REND_BLEED)
            .unwrap();
        assert_eq!(aura.caster, h.caster);
    }
    h.advance(65);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(REND_BLEED)
                && event.kind == CombatLogKind::Damage
                && event.periodic
                && event.amount > 0),
        "Rend must tick real damage, not just register an aura"
    );
}

#[test]
fn protection_producers_tc_thunder_clap_without_known_rend_preserves_slow_only() {
    let (h, _) = clap(false);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(REND_BLEED))
    );
    assert!(h.inbox.log.iter().any(|event| event.spell_id == Some(435203)
        && event.kind == CombatLogKind::AuraApplied));
}

#[test]
fn protection_producers_tc_shield_charge_moves_then_native_landing_children_deal_damage() {
    let mut h = fixture(false);
    let target = h.npc;
    h.app.world_mut().entity_mut(target).insert(Position {
        x: 10.0,
        y: 0.0,
        z: 0.0,
    });
    let near = spawn_npc(&mut h.app, 12.0);
    let far = spawn_npc(&mut h.app, 30.0);
    h.cast_and_deliver(SHIELD_CHARGE, Some(target));
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|event| event.spell_id == Some(SHIELD_CHARGE_DAMAGE)
                && event.kind == CombatLogKind::Damage),
        "child damage waits for native charge flight"
    );
    h.advance(20);
    assert!(
        h.app.world().get::<Position>(h.caster).unwrap().x > 5.0,
        "TC Dummy must cast native385953 effect96 movement"
    );
    let damage: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(SHIELD_CHARGE_DAMAGE)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0
        })
        .collect();
    assert!(
        damage
            .iter()
            .any(|event| event.target == Some(target.to_bits())),
        "landing must cast native385954 primary damage"
    );
    assert!(
        damage
            .iter()
            .any(|event| event.target == Some(near.to_bits())),
        "native landing child preserves its area damage"
    );
    assert!(
        !damage
            .iter()
            .any(|event| event.target == Some(far.to_bits()))
    );
    assert!(
        crate::auras::has_aura(h.app.world(), target, SHIELD_CHARGE_DAMAGE),
        "native child stun must accompany damage"
    );
}
