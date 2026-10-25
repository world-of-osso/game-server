//! LIFEBLOOM_REFRESH_BLOOM_THRESHOLD_SIMC: inclusive remaining <= 30%.
use super::kit_tests::at_level;
use crate::auras::{ActiveAuras, AuraInstance, aura_now};
use crate::spell_cast::{creature_cast, test_harness::Harness};
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use shared::components::Health;
use shared::protocol::CombatLogKind;

const CRENNA: u32 = 209072;
const LIFEBLOOM: u32 = 33763;
const BLOOM: u32 = 33778;

fn own_lifebloom(h: &Harness, caster: Entity) -> AuraInstance {
    h.app
        .world()
        .get::<ActiveAuras>(caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == LIFEBLOOM && aura.caster == caster)
        .expect("native own Lifebloom must remain active")
        .clone()
}

fn bloom_count(h: &Harness, caster: Entity) -> usize {
    h.inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(BLOOM)
                && event.kind == CombatLogKind::Heal
                && event.source == Some(caster.to_bits())
                && event.target == Some(caster.to_bits())
        })
        .count()
}

fn prepare_refresh(remaining_fraction: f64) -> (Harness, Entity, AuraInstance) {
    let (mut h, caster) = at_level(CRENNA, 11, |_| {});
    let mut health = h.app.world_mut().get_mut::<Health>(caster).unwrap();
    health.current = health.max * 0.1;
    creature_cast(h.app.world_mut(), caster, LIFEBLOOM, Some(caster), false).unwrap();
    h.advance(1);
    let aura = own_lifebloom(&h, caster);
    let duration = f64::from(aura.duration_ms) / 1000.0;
    let desired = aura.expires_at.unwrap() - duration * remaining_fraction;
    let frame = match h.app.world().resource::<TimeUpdateStrategy>() {
        TimeUpdateStrategy::ManualDuration(step) => step.as_secs_f64(),
        _ => panic!("native refresh qualification requires a deterministic frame"),
    };
    h.advance(((desired - aura_now(h.app.world())) / frame).ceil() as usize);
    assert_eq!(bloom_count(&h, caster), 0, "no bloom before actual refresh");
    let remaining = aura.expires_at.unwrap() - aura_now(h.app.world());
    assert!(remaining > 0.0);
    assert!((remaining / duration - remaining_fraction).abs() <= frame / duration + 1e-6);
    (h, caster, aura)
}

fn refresh_and_observe(h: &mut Harness, caster: Entity, old: &AuraInstance) {
    creature_cast(h.app.world_mut(), caster, LIFEBLOOM, Some(caster), false).unwrap();
    h.advance(1);
    let refreshed = own_lifebloom(h, caster);
    assert_eq!(refreshed.duration_ms, old.duration_ms);
    assert!(refreshed.expires_at.unwrap() > old.expires_at.unwrap());
    let casts = h
        .inbox
        .go
        .iter()
        .filter(|go| go.caster == caster.to_bits() && go.spell_id == LIFEBLOOM)
        .count();
    assert_eq!(casts, 2, "both applications must use ordinary native casts");
}

#[test]
fn follower_lifebloom_early_native_refresh_does_not_bloom() {
    let (mut h, caster, old) = prepare_refresh(0.6);
    refresh_and_observe(&mut h, caster, &old);
    assert_eq!(bloom_count(&h, caster), 0);
}

#[test]
fn follower_lifebloom_late_native_refresh_blooms_once_and_retains_hot() {
    let (mut h, caster, old) = prepare_refresh(0.2);
    refresh_and_observe(&mut h, caster, &old);
    assert_eq!(
        bloom_count(&h, caster),
        1,
        "late native refresh needs one real33778 heal"
    );
    let bloom = h
        .inbox
        .log
        .iter()
        .find(|event| event.spell_id == Some(BLOOM) && event.kind == CombatLogKind::Heal)
        .unwrap();
    assert!(bloom.amount > 0);
    assert!(!bloom.periodic);
    h.advance(1);
    assert_eq!(
        bloom_count(&h, caster),
        1,
        "refresh must not register duplicate blooms"
    );
    own_lifebloom(&h, caster);
}
