//! Ordinary owned184364 resolves the native385174 self-heal only with actual383468.
use super::*;
use crate::class_progression::KnownSpells;

fn owned_heals(h: &Harness) -> Vec<&shared::protocol::CombatLogEvent> {
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(HEAL)
                && event.kind == CombatLogKind::Heal
                && event.source == Some(h.caster.to_bits())
        })
        .collect()
}

fn assert_owned_heal(h: &Harness, amount: i32, overflow: i32) {
    let heals = owned_heals(h);
    assert_eq!(heals.len(), 1, "exactly one owned native385174 heal");
    let heal = heals[0];
    assert_eq!(heal.target, Some(h.caster.to_bits()));
    assert_eq!(
        (heal.amount, heal.overflow, heal.crit),
        (amount, overflow, false)
    );
    assert!(h.inbox.log.iter().any(|event| {
        event.spell_id == Some(HEAL)
            && event.kind == CombatLogKind::CastSuccess
            && event.source == Some(h.caster.to_bits())
    }));
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|entry| { entry.spell_id == HEAL && entry.duration_ms > 0 })
    );
}

#[test]
#[ignore = "pushed native regression before Invigorating Fury AfterCast binding"]
fn native_invigorating_fury_wounded_root_cast_heals_once_with_native_payload() {
    let mut h = fixture(true);
    cast_native_attack(&mut h, WOUND);
    assert_eq!(h.health(h.caster), 200.0);
    cast_root(&mut h);
    assert_owned_heal(&h, 100, 0);
    assert_eq!(h.health(h.caster), 300.0);
    assert_native_recovery(&h);
    let root = aura(&h, ENRAGED_REGENERATION).unwrap();
    assert_eq!(root.duration_ms, 11000);
    expire_root_naturally(&mut h, root.expires_at.unwrap());
    assert_eq!(
        owned_heals(&h).len(),
        1,
        "expiry must not cast another heal"
    );
    // Ordinary idle regeneration is intentionally enabled; no frozen late HP oracle.
}

#[test]
#[ignore = "pushed native regression before Invigorating Fury AfterCast binding"]
fn native_invigorating_fury_full_health_still_casts_real_native_overheal() {
    let mut h = fixture(true);
    cast_root(&mut h);
    assert_owned_heal(&h, 100, 100);
    assert_eq!(h.health(h.caster), 1000.0);
}

#[test]
#[ignore = "pushed native regression before Invigorating Fury AfterCast binding"]
fn native_invigorating_fury_missing_talent_never_heals_wounded_owner() {
    let mut h = fixture(false);
    cast_native_attack(&mut h, WOUND);
    assert_eq!(h.health(h.caster), 200.0);
    cast_root(&mut h);
    assert!(owned_heals(&h).is_empty());
    assert_eq!(h.health(h.caster), 200.0);
    assert_eq!(aura(&h, ENRAGED_REGENERATION).unwrap().duration_ms, 8000);
}

#[test]
#[ignore = "pushed native regression before Invigorating Fury AfterCast binding"]
fn native_invigorating_fury_unlearned_actual_passive_no_longer_heals() {
    let mut h = fixture(true);
    assert_eq!(aura(&h, TALENT).unwrap().source, AuraSource::KnownSpell);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(KnownSpells::fixture(vec![
            ENRAGED_REGENERATION,
            HEAL,
            SHATTERING_THROW,
            WOUND,
            NON_BYPASSING_DAMAGE,
        ]));
    h.advance(3);
    assert!(aura(&h, TALENT).is_none());
    cast_native_attack(&mut h, WOUND);
    cast_root(&mut h);
    assert!(owned_heals(&h).is_empty());
    assert_eq!(h.health(h.caster), 200.0);
    assert_eq!(aura(&h, ENRAGED_REGENERATION).unwrap().duration_ms, 8000);
}

#[test]
#[ignore = "pushed native regression before Invigorating Fury AfterCast binding"]
fn native_invigorating_fury_other_warriors_known_talent_does_not_enable_owner() {
    let mut h = fixture(true);
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(KnownSpells::fixture(vec![
            ENRAGED_REGENERATION,
            HEAL,
            SHATTERING_THROW,
            WOUND,
            NON_BYPASSING_DAMAGE,
        ]));
    let peer = super::super::test_harness::spawn_player(&mut h.app, 1, 1.0);
    h.app.world_mut().entity_mut(peer).insert((
        KnownSpells::fixture(vec![TALENT]),
        crate::class_progression::ActiveSpec(72),
    ));
    h.advance(3);
    let peer_talent = h
        .app
        .world()
        .get::<ActiveAuras>(peer)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == TALENT)
        .unwrap();
    assert_eq!(
        (peer_talent.caster, peer_talent.source),
        (peer, AuraSource::KnownSpell)
    );
    assert!(aura(&h, TALENT).is_none());
    cast_native_attack(&mut h, WOUND);
    cast_root(&mut h);
    assert!(owned_heals(&h).is_empty());
    assert_eq!(h.health(h.caster), 200.0);
    assert_eq!(h.health(peer), 1000.0);
}

#[test]
#[ignore = "pushed native regression before Invigorating Fury AfterCast binding"]
fn native_invigorating_fury_refused_root_recast_cannot_issue_another_heal() {
    let mut h = fixture(true);
    cast_native_attack(&mut h, WOUND);
    cast_root(&mut h);
    assert_owned_heal(&h, 100, 0);
    let root = aura(&h, ENRAGED_REGENERATION).unwrap();
    h.cast_and_deliver(ENRAGED_REGENERATION, None);
    h.advance(3);
    assert!(
        !h.inbox.failed.is_empty(),
        "native120s recovery must reject recast"
    );
    assert_owned_heal(&h, 100, 0);
    assert_eq!(h.health(h.caster), 300.0);
    let after = aura(&h, ENRAGED_REGENERATION).unwrap();
    assert_eq!(
        (after.instance_id, after.expires_at),
        (root.instance_id, root.expires_at)
    );
}
