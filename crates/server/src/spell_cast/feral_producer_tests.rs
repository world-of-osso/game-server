//! Native70448, SimC76f8118 sc_druid.cpp4114–4160; no modified rows or resource grants.
use super::test_harness::*;
use bevy::prelude::*;
use shared::components::PowerType;
use shared::protocol::CombatLogKind;

const CAT_FORM: u32 = 768;
const SHRED: u32 = 5221;
const FERAL_FRENZY: u32 = 274837;
const FERAL_FRENZY_TICK: u32 = 274838;

fn fixture(frenzy_known: bool) -> Harness {
    let mut ids = vec![CAT_FORM, SHRED];
    if frenzy_known {
        ids.push(FERAL_FRENZY);
    }
    let mut h = harness_with(Vec::new(), &ids, 11, true).unwrap();
    h.app.insert_resource(world_db().unwrap().store.clone());
    h.app.insert_resource(load_native_creature_world(&[6]));
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::ActiveSpec(103));
    let caster = h.caster;
    h.cast_and_deliver(CAT_FORM, Some(caster));
    h.advance(220);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(crate::auras::has_aura(h.app.world(), caster, CAT_FORM));
    assert!(h.power(PowerType::Energy) >= 25.0);
    assert_eq!(h.power(PowerType::ComboPoints), 0.0);
    h.inbox = Inbox::default();
    h
}

fn landed_frenzy() -> Harness {
    // Whole native attempts preserve ordinary avoidance and resource payment.
    for _ in 0..8 {
        let mut h = fixture(true);
        let npc = h.npc;
        let energy = h.power(PowerType::Energy);
        h.cast_and_deliver(FERAL_FRENZY, Some(npc));
        assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
        if h.inbox
            .go
            .iter()
            .any(|go| go.spell_id == FERAL_FRENZY && go.hit_targets.contains(&npc.to_bits()))
        {
            assert!(h.power(PowerType::Energy) < energy);
            assert!(crate::auras::has_aura(
                h.app.world(),
                h.caster,
                FERAL_FRENZY
            ));
            return h;
        }
    }
    panic!("eight native Feral Frenzy casts produced no landed root");
}

fn frenzy_damage(h: &Harness, periodic: bool) -> usize {
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(FERAL_FRENZY_TICK)
                && event.kind == CombatLogKind::Damage
                && event.periodic == periodic
                && event.amount > 0
        })
        .count()
}

#[test]
#[ignore = "RED: native Feral Frenzy root must produce its attack/bleed child"]
fn feral_producers_frenzy_native_root_retains_recipient_and_pays_five_combo_points() {
    let mut h = landed_frenzy();
    let other = spawn_npc(&mut h.app, 4.0);
    let ally = spawn_player(&mut h.app, 11, 4.0);
    h.select(Some(other));
    h.advance(35);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(
        frenzy_damage(&h, false) > 0,
        "landed native root must produce actual child attack damage; log={:?}",
        h.inbox.log
    );
    let attacks = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(FERAL_FRENZY_TICK)
                && !event.periodic
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss)
        })
        .count();
    assert_eq!(
        attacks, 5,
        "native one-second/200ms root produces five attacks"
    );
    let gains: Vec<_> = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(FERAL_FRENZY_TICK) && event.kind == CombatLogKind::Energize
        })
        .collect();
    assert_eq!(gains.len(), 5);
    assert!(gains.iter().all(|event| {
        event.source == Some(h.caster.to_bits())
            && event.target == Some(h.caster.to_bits())
            && event.amount == 1
            && event.overflow == 0
    }));
    assert_eq!(h.power(PowerType::ComboPoints), 5.0);
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        FERAL_FRENZY
    ));
    h.advance(180);
    assert!(
        frenzy_damage(&h, true) > 0,
        "child must retain its native bleed"
    );
    assert!(
        h.inbox
            .log
            .iter()
            .filter(|event| {
                event.spell_id == Some(FERAL_FRENZY_TICK)
                    && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss)
            })
            .all(|event| {
                event.source == Some(h.caster.to_bits()) && event.target == Some(h.npc.to_bits())
            })
    );
    for unit in [h.caster, h.npc, other, ally] {
        assert!(!crate::auras::has_aura(
            h.app.world(),
            unit,
            FERAL_FRENZY_TICK
        ));
    }
    let damage = frenzy_damage(&h, false) + frenzy_damage(&h, true);
    h.advance(60);
    assert_eq!(frenzy_damage(&h, false) + frenzy_damage(&h, true), damage);
}

#[test]
fn feral_producers_frenzy_native_catalog_without_root_cast_grants_no_children() {
    let mut h = fixture(false);
    let npc = h.npc;
    h.cast_and_deliver(SHRED, Some(npc));
    h.advance(200);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.go.iter().any(|go| go.spell_id == SHRED));
    assert!(!h.inbox.log.iter().any(|event| {
        event.spell_id == Some(FERAL_FRENZY_TICK) || event.spell_id == Some(FERAL_FRENZY)
    }));
    assert!(!crate::auras::has_aura(
        h.app.world(),
        h.caster,
        FERAL_FRENZY
    ));
}
