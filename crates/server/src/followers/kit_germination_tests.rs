//! SimC76f8118 sc_druid.cpp6487–6518; native talent first learned at33.
use super::kit_tests::at_level;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::Harness;
use crate::spell_cast::{SpellHistory, creature_cast, history_now};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use shared::components::Health;
use shared::protocol::CombatLogKind;

const CRENNA: u32 = 209072;
const REJUVENATION: u32 = 774;
const GERMINATION_TALENT: u32 = 155675;
const GERMINATION_HOT: u32 = 155777;

fn own_expiry(h: &Harness, healer: Entity, spell: u32) -> Option<f64> {
    h.app
        .world()
        .get::<ActiveAuras>(healer)?
        .iter()
        .find(|aura| aura.caster == healer && aura.spell_id == spell)?
        .expires_at
}

fn cast_ordinary_rejuvenation(h: &mut Harness, healer: Entity) {
    creature_cast(h.app.world_mut(), healer, REJUVENATION, Some(healer), false).unwrap();
    for _ in 0..200 {
        h.advance(1);
        let world = h.app.world();
        let spell = world.resource::<SpellStore>().get(REJUVENATION).unwrap();
        let locked = world
            .get::<SpellHistory>(healer)
            .is_some_and(|history| history.on_gcd(spell, history_now(world)));
        if !locked {
            return;
        }
    }
    panic!("native Rejuvenation GCD did not clear");
}

fn prepare_two_native_hots() -> (Harness, Entity) {
    let (mut h, healer) = at_level(CRENNA, 33, |_| {});
    assert!(crate::auras::has_aura(
        h.app.world(),
        healer,
        GERMINATION_TALENT
    ));
    cast_ordinary_rejuvenation(&mut h, healer);
    assert!(own_expiry(&h, healer, REJUVENATION).is_some());
    cast_ordinary_rejuvenation(&mut h, healer);
    assert!(own_expiry(&h, healer, REJUVENATION).is_some());
    assert!(
        own_expiry(&h, healer, GERMINATION_HOT).is_some(),
        "second ordinary774 with actual learned155675 must create own155777"
    );
    (h, healer)
}

#[test]
#[ignore = "GERMINATION_NATIVE_ROUTING: level32 absent-talent refresh control"]
fn follower_germination_level32_without_talent_refreshes_one_rejuvenation() {
    let (mut h, healer) = at_level(CRENNA, 32, |_| {});
    assert!(!crate::auras::has_aura(
        h.app.world(),
        healer,
        GERMINATION_TALENT
    ));
    cast_ordinary_rejuvenation(&mut h, healer);
    let before = own_expiry(&h, healer, REJUVENATION).unwrap();
    cast_ordinary_rejuvenation(&mut h, healer);
    assert!(own_expiry(&h, healer, REJUVENATION).unwrap() > before);
    assert!(own_expiry(&h, healer, GERMINATION_HOT).is_none());
    let count = h
        .app
        .world()
        .get::<ActiveAuras>(healer)
        .unwrap()
        .iter()
        .filter(|aura| aura.caster == healer && aura.spell_id == REJUVENATION)
        .count();
    assert_eq!(count, 1);
}

#[test]
#[ignore = "GERMINATION_NATIVE_ROUTING: actual learned33 second774 creates periodic child"]
fn follower_germination_level33_second_rejuvenation_keeps_two_owned_hots() {
    let (mut h, healer) = prepare_two_native_hots();
    let mut health = h.app.world_mut().get_mut::<Health>(healer).unwrap();
    health.current = health.max * 0.3;
    let store = h.app.world().resource::<SpellStore>();
    let period_ms = [REJUVENATION, GERMINATION_HOT]
        .into_iter()
        .map(|spell| {
            store
                .get(spell)
                .unwrap()
                .effects
                .iter()
                .find(|effect| effect.aura == 8)
                .unwrap()
                .aura_period_ms
        })
        .max()
        .unwrap();
    assert!(period_ms > 0);
    let frame_ms = match h.app.world().resource::<TimeUpdateStrategy>() {
        TimeUpdateStrategy::ManualDuration(step) => step.as_millis(),
        _ => panic!("native qualification requires a fixed frame duration"),
    };
    h.advance(u128::from(period_ms).div_ceil(frame_ms) as usize + 1);
    for spell in [REJUVENATION, GERMINATION_HOT] {
        assert!(
            h.inbox
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Heal
                    && event.periodic
                    && event.spell_id == Some(spell)
                    && event.source == Some(healer.to_bits())
                    && event.target == Some(healer.to_bits())
                    && event.amount > 0),
            "actual periodic heal missing for {spell}"
        );
    }
}

#[test]
#[ignore = "GERMINATION_NATIVE_ROUTING: second-HoT prerequisite before shorter-remaining routing"]
fn follower_germination_level33_third_cast_refreshes_shorter_owned_hot() {
    let (mut h, healer) = prepare_two_native_hots();
    let base = own_expiry(&h, healer, REJUVENATION).unwrap();
    let germination = own_expiry(&h, healer, GERMINATION_HOT).unwrap();
    assert_ne!(
        base, germination,
        "ordinary staggered casts must have distinct deadlines"
    );
    let (shorter, longer, before_shorter, before_longer) = if base < germination {
        (REJUVENATION, GERMINATION_HOT, base, germination)
    } else {
        (GERMINATION_HOT, REJUVENATION, germination, base)
    };
    cast_ordinary_rejuvenation(&mut h, healer);
    assert!(own_expiry(&h, healer, shorter).unwrap() > before_shorter);
    assert_eq!(
        own_expiry(&h, healer, longer),
        Some(before_longer),
        "refreshing the shorter own HoT must not change the longer instance deadline"
    );
}
