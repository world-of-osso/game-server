//! Natural acquisition bounds are fixture confidence, never gameplay timers.
use super::kit_tests::at_level;
use crate::auras::{ActiveAuras, AuraInstance, aura_now, has_aura};
use crate::spell_cast::test_harness::Harness;
use crate::spell_cast::{CostPool, SpellHistory, creature_cast, history_now, spell_costs};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use bevy::time::TimeUpdateStrategy;
use shared::components::{Mana, PowerType};
use shared::protocol::CombatLogKind;

const CRENNA: u32 = 209072;
const OMEN: u32 = 113043;
const CLEARCASTING: u32 = 16870;
const LIFEBLOOM: u32 = 33763;
const REGROWTH: u32 = 8936;
const REJUVENATION: u32 = 774;
const ACQUISITION_MISS_BUDGET: f64 = 1e-6;

pub(super) fn own_aura(h: &Harness, caster: Entity, spell: u32) -> Option<AuraInstance> {
    h.app
        .world()
        .get::<ActiveAuras>(caster)?
        .iter()
        .find(|aura| aura.caster == caster && aura.spell_id == spell)
        .cloned()
}

pub(super) fn frame_seconds(h: &Harness) -> f64 {
    match h.app.world().resource::<TimeUpdateStrategy>() {
        TimeUpdateStrategy::ManualDuration(frame) => frame.as_secs_f64(),
        _ => panic!("natural proc qualification needs deterministic frame duration"),
    }
}

pub(super) fn mana_cost(h: &Harness, caster: Entity, spell: u32) -> f32 {
    let world = h.app.world();
    let spell = world.resource::<SpellStore>().get(spell).unwrap();
    spell_costs(world, caster, spell)
        .unwrap()
        .into_iter()
        .filter_map(|(pool, amount)| {
            matches!(pool, CostPool::Power(PowerType::Mana)).then_some(amount)
        })
        .sum()
}

pub(super) fn wait_for_gcd(h: &mut Harness, caster: Entity, spell: u32) {
    loop {
        let world = h.app.world();
        let spell = world.resource::<SpellStore>().get(spell).unwrap();
        let locked = world
            .get::<SpellHistory>(caster)
            .is_some_and(|history| history.on_gcd(spell, history_now(world)));
        if !locked {
            return;
        }
        h.advance(1);
    }
}

pub(super) fn complete_real_cast(h: &mut Harness, caster: Entity, spell: u32) {
    wait_for_gcd(h, caster, spell);
    let start = h.inbox.go.len();
    creature_cast(h.app.world_mut(), caster, spell, Some(caster), false).unwrap();
    for _ in 0..200 {
        h.advance(1);
        if h.inbox.go[start..]
            .iter()
            .any(|go| go.caster == caster.to_bits() && go.spell_id == spell)
        {
            return;
        }
    }
    panic!("ordinary native cast{spell} did not complete");
}

/// Count actual late-enough Lifebloom ticks, then allow that HoT to expire.
/// No further eligible periodic source can regrant the buff during Regrowth.
fn finish_native_lifebloom(
    h: &mut Harness,
    caster: Entity,
    needed_window: f64,
    buff_duration: f64,
) -> usize {
    complete_real_cast(h, caster, LIFEBLOOM);
    let hot = own_aura(h, caster, LIFEBLOOM).unwrap();
    let expiry = hot.expires_at.unwrap();
    let mut eligible = 0;
    let mut native_ticks = 0;
    let mut buff_deadline_changes = 0;
    while own_aura(h, caster, LIFEBLOOM).is_some() {
        let start = h.inbox.log.len();
        let previous_buff = own_aura(h, caster, CLEARCASTING).and_then(|aura| aura.expires_at);
        h.advance(1);
        let current_buff = own_aura(h, caster, CLEARCASTING).and_then(|aura| aura.expires_at);
        if current_buff.is_some() && current_buff != previous_buff {
            buff_deadline_changes += 1;
        }
        native_ticks += h.inbox.log[start..]
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Heal
                    && event.periodic
                    && event.spell_id == Some(LIFEBLOOM)
            })
            .count();
        let now = aura_now(h.app.world());
        if now + buff_duration - expiry <= needed_window + frame_seconds(h) {
            continue;
        }
        eligible += h.inbox.log[start..]
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Heal
                    && event.periodic
                    && event.spell_id == Some(LIFEBLOOM)
                    && event.source == Some(caster.to_bits())
                    && event.target == Some(caster.to_bits())
            })
            .count();
    }
    println!(
        "OMEN life native_ticks={native_ticks}, buff_deadline_changes={buff_deadline_changes}"
    );
    eligible
}

fn earn_native_clearcasting() -> (Harness, Entity) {
    earn_native_clearcasting_at_level(15)
}

pub(super) fn earn_native_clearcasting_at_level(level: u8) -> (Harness, Entity) {
    let (mut h, caster) = at_level(CRENNA, level, |_| {});
    assert!(has_aura(h.app.world(), caster, OMEN));
    let store = h.app.world().resource::<SpellStore>();
    let omen = store.get(OMEN).unwrap();
    let chance = omen
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .base_points;
    assert_eq!(chance, 4.0);
    let tick_bound =
        (ACQUISITION_MISS_BUDGET.ln() / (-f64::from(chance) / 100.0).ln_1p()).ceil() as usize;
    let buff_duration = f64::from(
        store
            .get(CLEARCASTING)
            .unwrap()
            .misc
            .as_ref()
            .unwrap()
            .duration
            .unwrap()
            .duration_ms,
    ) / 1000.0;
    let cast_time = f64::from(
        store
            .get(REGROWTH)
            .unwrap()
            .misc
            .as_ref()
            .unwrap()
            .cast_time
            .unwrap()
            .base_ms,
    ) / 1000.0;
    let needed_window = 2.0 * cast_time + frame_seconds(&h);
    let mut attempts = 0;
    while attempts < tick_bound {
        attempts += finish_native_lifebloom(&mut h, caster, needed_window, buff_duration);
        let sufficient = own_aura(&h, caster, CLEARCASTING)
            .is_some_and(|buff| buff.expires_at.unwrap() - aura_now(h.app.world()) > needed_window);
        if sufficient {
            println!(
                "CLEARCASTING acquired after{attempts} eligible natural ticks; native{chance}%, bound{tick_bound}, miss budget{ACQUISITION_MISS_BUDGET}"
            );
            assert!(own_aura(&h, caster, LIFEBLOOM).is_none());
            return (h, caster);
        }
    }
    panic!(
        "CLEARCASTING_ACQUISITION_PREREQUISITE: no usable native buff after{attempts} eligible ticks, bound{tick_bound}"
    );
}

#[test]
fn follower_clearcasting_level14_native_lifebloom_never_grants_buff() {
    let (mut h, caster) = at_level(CRENNA, 14, |_| {});
    assert!(!has_aura(h.app.world(), caster, OMEN));
    finish_native_lifebloom(&mut h, caster, 0.0, 0.0);
    assert!(!has_aura(h.app.world(), caster, CLEARCASTING));
    assert!(mana_cost(&h, caster, REGROWTH) > 0.0);
}

#[test]
fn follower_clearcasting_native_regrowth_is_free_once_then_consumes_buff() {
    let (mut h, caster) = earn_native_clearcasting();
    assert_eq!(mana_cost(&h, caster, REGROWTH), 0.0);
    let before = h.app.world().get::<Mana>(caster).unwrap().current;
    complete_real_cast(&mut h, caster, REGROWTH);
    assert!(
        h.app.world().get::<Mana>(caster).unwrap().current >= before,
        "free completed Regrowth must not debit mana; natural regeneration remains enabled"
    );
    assert!(
        !has_aura(h.app.world(), caster, CLEARCASTING),
        "one completed native Regrowth must consume16870"
    );
    assert!(mana_cost(&h, caster, REGROWTH) > 0.0);
    wait_for_gcd(&mut h, caster, REGROWTH);
    let before_paid = h.app.world().get::<Mana>(caster).unwrap().current;
    complete_real_cast(&mut h, caster, REGROWTH);
    assert!(
        h.app.world().get::<Mana>(caster).unwrap().current < before_paid,
        "next ordinary Regrowth must pay real native mana despite regeneration"
    );
}

#[test]
fn follower_clearcasting_native_rejuvenation_preserves_earned_buff() {
    let (mut h, caster) = earn_native_clearcasting();
    let buff = own_aura(&h, caster, CLEARCASTING).unwrap();
    assert!(mana_cost(&h, caster, REJUVENATION) > 0.0);
    complete_real_cast(&mut h, caster, REJUVENATION);
    let retained = own_aura(&h, caster, CLEARCASTING).expect("Rejuvenation must not consume16870");
    assert_eq!(retained.instance_id, buff.instance_id);
    assert_eq!(retained.expires_at, buff.expires_at);
    assert_eq!(mana_cost(&h, caster, REGROWTH), 0.0);
}
