//! Actual ordinary202168, native202166 and direct native32216 only.
//! VICTORIOUS_KILL_PRODUCER_UNIMPLEMENTED: no kill producer or forged aura.
use super::*;
use crate::class_progression::KnownSpells;

fn cast_root(h: &mut Harness) {
    h.cast_and_deliver(ROOT, Some(h.npc));
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(h.inbox.log.iter().any(|event| event.spell_id == Some(ROOT)
        && event.kind == CombatLogKind::CastSuccess
        && event.source == Some(h.caster.to_bits())));
}

fn cast_victorious(h: &mut Harness) -> AuraInstance {
    h.cast_and_deliver(VICTORIOUS, None);
    h.advance(3);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    let applied = aura(h, h.caster, VICTORIOUS).unwrap();
    assert_eq!((applied.caster, applied.duration_ms), (h.caster, 20000));
    applied
}

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

fn assert_native_heal(h: &Harness, index: usize, full: bool) -> i32 {
    let heals = owned_heals(h);
    let heal = heals
        .get(index)
        .expect("ordinary202168 must issue real owned202166 heal");
    let amount = if heal.crit { 600 } else { 300 };
    assert_eq!(heal.target, Some(h.caster.to_bits()));
    assert_eq!(heal.amount, amount);
    assert_eq!(heal.overflow, if full { amount } else { 0 });
    assert!(!heal.periodic);
    assert!(
        !h.inbox
            .cooldowns
            .iter()
            .any(|entry| entry.spell_id == HEAL && entry.duration_ms > 0)
    );
    amount
}

#[test]
fn native_impending_victory_wounded_owner_without_victorious_still_heals_once() {
    let mut h = fixture();
    wound(&mut h);
    charge_for_real_rage(&mut h);
    assert!(aura(&h, h.caster, VICTORIOUS).is_none());
    let before = h.power(PowerType::Rage);
    cast_root(&mut h);
    assert_eq!(owned_heals(&h).len(), 1);
    let amount = assert_native_heal(&h, 0, false);
    assert_eq!(h.health(h.caster), 200.0 + amount as f32);
    assert_eq!(h.power(PowerType::Rage), before - 100.0);
}

#[test]
fn native_impending_victory_full_health_casts_native_overheal() {
    let mut h = fixture();
    charge_for_real_rage(&mut h);
    cast_root(&mut h);
    assert_eq!(owned_heals(&h).len(), 1);
    assert_native_heal(&h, 0, true);
    assert_eq!(h.health(h.caster), 1000.0);
}

#[test]
fn native_impending_victory_consumes_only_casting_warriors_actual_victorious() {
    let mut h = fixture();
    let peer = super::super::test_harness::spawn_player(&mut h.app, 1, 1.0);
    h.app
        .world_mut()
        .entity_mut(peer)
        .insert(KnownSpells::fixture(vec![VICTORIOUS]));
    super::super::creature_cast(h.app.world_mut(), peer, VICTORIOUS, None, false).unwrap();
    h.advance(3);
    let foreign = aura(&h, peer, VICTORIOUS).expect("actual foreign native32216 application");
    cast_victorious(&mut h);
    assert_eq!(h.power(PowerType::Rage), 0.0);
    cast_root(&mut h);
    assert!(
        aura(&h, h.caster, VICTORIOUS).is_none(),
        "source AfterCast must remove caster32216"
    );
    let retained = aura(&h, peer, VICTORIOUS).expect("other Warrior aura remains");
    assert_eq!(
        (retained.instance_id, retained.expires_at, retained.caster),
        (foreign.instance_id, foreign.expires_at, peer)
    );
    assert_eq!(h.power(PowerType::Rage), 0.0, "native32216 made root free");
    assert_eq!(h.health(peer), 1000.0);
    assert_eq!(owned_heals(&h).len(), 1);
    assert_native_heal(&h, 0, true);
}

#[test]
fn native_impending_victory_refused_zero_rage_without_victorious_cannot_heal() {
    let mut h = fixture();
    wound(&mut h);
    assert!(aura(&h, h.caster, VICTORIOUS).is_none());
    assert_eq!(h.power(PowerType::Rage), 0.0);
    h.cast_and_deliver(ROOT, Some(h.npc));
    h.advance(3);
    assert!(!h.inbox.failed.is_empty());
    assert!(owned_heals(&h).is_empty());
    assert_eq!(h.health(h.caster), 200.0);
    assert_eq!(h.power(PowerType::Rage), 0.0);
}

#[test]
fn native_impending_victory_cooldown_refusal_neither_heals_nor_consumes_reapplied_aura() {
    let mut h = fixture();
    cast_victorious(&mut h);
    cast_root(&mut h);
    assert_eq!(owned_heals(&h).len(), 1);
    h.advance(40); // Native1500ms GCD ends; native25s cooldown does not.
    let reapplied = cast_victorious(&mut h);
    h.cast_and_deliver(ROOT, Some(h.npc));
    h.advance(3);
    assert!(!h.inbox.failed.is_empty());
    assert_eq!(owned_heals(&h).len(), 1);
    let retained =
        aura(&h, h.caster, VICTORIOUS).expect("rejected root cannot consume actual32216");
    assert_eq!(
        (retained.instance_id, retained.expires_at),
        (reapplied.instance_id, reapplied.expires_at)
    );
    assert_eq!(h.power(PowerType::Rage), 0.0);
}

#[test]
fn native_impending_victory_natural_recovery_allows_new_real_wound_heal() {
    let mut h = fixture();
    cast_victorious(&mut h);
    cast_root(&mut h);
    assert_eq!(owned_heals(&h).len(), 1);
    assert_native_heal(&h, 0, true);
    let recovery = h
        .inbox
        .cooldowns
        .iter()
        .find(|entry| entry.spell_id == ROOT && !entry.is_gcd)
        .unwrap()
        .duration_ms;
    assert_eq!(recovery, 25000);
    let dt = h.app.world().resource::<Time>().delta_secs();
    h.advance((recovery as f32 / 1000.0 / dt).ceil() as usize + 4);
    assert_eq!(
        owned_heals(&h).len(),
        1,
        "idle native cooldown cannot cause heal"
    );
    wound(&mut h); // Wound AFTER idle wait; ordinary regeneration remains enabled.
    cast_victorious(&mut h);
    cast_root(&mut h);
    assert_eq!(owned_heals(&h).len(), 2);
    let amount = assert_native_heal(&h, 1, false);
    assert_eq!(h.health(h.caster), 200.0 + amount as f32);
    assert!(aura(&h, h.caster, VICTORIOUS).is_none());
    assert_eq!(h.power(PowerType::Rage), 0.0);
}
