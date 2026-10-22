//! Source48629's native332 Cat Swipe override, without inventing a boost aura.
use super::rotation_refresh_tests::{cast_native, native_dummy};
use super::{Follower, FollowerCatalog, ai, update_ai};
use crate::spell_cast::test_harness::spawn_npc;
use shared::components::{Health, ThreatTable};

#[test]
fn rotation_fidelity_feral_uses_native_cat_swipe_only_while_in_cat_form() {
    let (mut h, unit) = native_dummy(232800);
    assert_eq!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(unit)
            .and_then(crate::auras::shapeshift::current_form)
            .map(|(_, form)| form),
        Some(1)
    );
    // Verify the actual starting state: the source override record is metadata,
    // not an applied48629 aura or a hand-granted106785 learned spell.
    assert!(!crate::auras::has_aura(h.app.world(), unit, 48629));
    let swipe = h
        .app
        .world()
        .resource::<FollowerCatalog>()
        .kits
        .iter()
        .find(|entry| entry.spec_id == 103 && entry.spell_id == 213764)
        .unwrap()
        .clone();
    h.app.world_mut().resource_mut::<FollowerCatalog>().kits = vec![swipe];
    for _ in 0..2 {
        let extra = spawn_npc(&mut h.app, 1.0);
        let mut threat = ThreatTable::default();
        threat.add_threat(h.caster.to_bits(), 1.0);
        h.app.world_mut().entity_mut(extra).insert((
            Health {
                current: 100_000_000.0,
                max: 100_000_000.0,
            },
            threat,
        ));
    }
    h.advance(3);
    let follower = h.app.world().get::<Follower>(unit).unwrap().clone();
    let party = ai::party_units(h.app.world_mut(), follower.owner);
    assert_eq!(
        ai::engaged_enemies(h.app.world_mut(), &follower, unit, &party).len(),
        3,
        "three actually engaged native dummies, not merely three spawned actors"
    );
    for _ in 0..200 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    let cat_casts = h
        .inbox
        .go
        .iter()
        .filter(|event| event.caster == unit.to_bits() && event.spell_id == 106785)
        .count();
    assert!(
        cat_casts > 0,
        "native Cat332 override must resolve before area/cast checks; actual casts={:?}",
        h.inbox
            .go
            .iter()
            .filter(|event| event.caster == unit.to_bits())
            .map(|event| event.spell_id)
            .collect::<Vec<_>>()
    );
    assert!(
        h.inbox.log.iter().any(|event| {
            event.source == Some(unit.to_bits())
                && event.spell_id == Some(106785)
                && event.kind == shared::protocol::CombatLogKind::Damage
                && event.amount > 0
        }),
        "the native replacement must actually deal damage"
    );
    cast_native(&mut h, unit, 5487, unit);
    for _ in 0..40 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert_eq!(
        h.inbox
            .go
            .iter()
            .filter(|event| { event.caster == unit.to_bits() && event.spell_id == 106785 })
            .count(),
        cat_casts,
        "leaving Cat Form must remove its cast-action override"
    );
}
