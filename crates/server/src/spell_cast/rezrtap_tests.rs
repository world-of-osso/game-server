use super::test_harness::*;
use crate::death::{DeathPlugin, PlayerDeathState};
use crate::spell_info::{SpellEffect, SpellInfo, SpellMisc, SpellRange};
use bevy::prelude::*;
use shared::components::{Health, Mana, MovementControl, Position, UnitTap};
use shared::death::DeathState;
use shared::protocol::{ResurrectionOffer, ResurrectionResponse};

fn fixture() -> (Harness, Entity, Entity) {
    let spell = SpellInfo {
        id: 7328,
        name: "Redemption".into(),
        misc: Some(SpellMisc {
            range: Some(SpellRange {
                max_friendly: 40.0,
                ..default()
            }),
            ..default()
        }),
        effects: vec![SpellEffect {
            effect: 18,
            base_points: 35.0,
            implicit_targets: [21, 0],
            ..default()
        }],
        ..default()
    };
    let mut h = harness_with_plugins(vec![spell], &[], 2, false, |app| {
        app.add_plugins(DeathPlugin);
    })
    .expect("world fixture required");
    let target = spawn_player(&mut h.app, 5, 3.0);
    h.app.world_mut().entity_mut(target).insert((
        PlayerDeathState {
            state: DeathState::Dead,
            ..default()
        },
        MovementControl::default(),
        Health {
            current: 0.0,
            max: 1000.0,
        },
        Mana {
            current: 0.0,
            max: 2000.0,
        },
    ));
    let conn = spawn_peer_connection(&mut h.app, target, 99);
    (h, target, conn)
}

fn offer(h: &mut Harness, target: Entity, conn: Entity) -> ResurrectionOffer {
    h.cast(7328, Some(target));
    let mut offers = Vec::new();
    for _ in 0..6 {
        h.advance(1);
        offers.extend(drain::<ResurrectionOffer>(&mut h.app, conn));
    }
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert_eq!(offers.len(), 1);
    offers.remove(0)
}

#[test]
fn rezrtap_cast_offer_accept_percent_and_caster_position() {
    let (mut h, target, conn) = fixture();
    let offer = offer(&mut h, target, conn);
    assert_eq!(offer.caster, h.caster.to_bits());
    assert_eq!(offer.caster_name, "Caster");
    assert_eq!(offer.spell_id, 7328);
    assert_eq!(offer.time_left_ms, 60_000);
    assert_eq!(h.health(target), 0.0);
    crate::death::resurrection::respond(
        h.app.world_mut(),
        target,
        ResurrectionResponse {
            caster: offer.caster,
            spell_id: 7328,
            accept: true,
        },
    );
    assert_eq!(
        h.app.world().get::<PlayerDeathState>(target).unwrap().state,
        DeathState::Alive
    );
    assert_eq!(h.health(target), 350.0);
    assert_eq!(h.app.world().get::<Mana>(target).unwrap().current, 700.0);
    assert_eq!(
        h.app.world().get::<Position>(target),
        h.app.world().get::<Position>(h.caster)
    );
}

#[test]
fn rezrtap_decline_and_timeout_remain_dead() {
    for timeout in [false, true] {
        let (mut h, target, conn) = fixture();
        let offer = offer(&mut h, target, conn);
        if timeout {
            h.app
                .world_mut()
                .get_mut::<crate::death::resurrection::PendingResurrection>(target)
                .unwrap()
                .expires_at = -1.0;
        }
        crate::death::resurrection::respond(
            h.app.world_mut(),
            target,
            ResurrectionResponse {
                caster: offer.caster,
                spell_id: 7328,
                accept: timeout,
            },
        );
        assert_eq!(h.health(target), 0.0);
        assert_eq!(
            h.app.world().get::<PlayerDeathState>(target).unwrap().state,
            DeathState::Dead
        );
    }
}

#[test]
fn rezrtap_damage_tap_projection_and_clear() {
    let mut h = harness(Vec::new(), &[], 2).expect("world fixture required");
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::persistence::ClientId(42));
    let caster = h.caster;
    let npc = h.npc;
    crate::creature_tap::record_damage(h.app.world_mut(), npc, caster, 100.0, 10.0);
    let tap = h.app.world().get::<UnitTap>(npc).unwrap();
    assert!(!tap.denied(caster.to_bits(), &[]));
    assert!(!tap.denied(99, &[caster.to_bits()]));
    assert!(tap.denied(99, &[100]));
    crate::creature_tap::clear(h.app.world_mut(), npc);
    assert!(!h.app.world().get::<UnitTap>(npc).unwrap().denied(99, &[]));
}
