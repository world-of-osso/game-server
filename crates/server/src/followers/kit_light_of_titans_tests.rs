//! Selected native Protection talent producer, no power/aura/stat grants.
use super::kit_clearcasting_tests::{frame_seconds, own_aura, wait_for_gcd};
use super::kit_tests::at_level;
use crate::auras::{ActiveAuras, AuraInstance, aura_now};
use crate::spell_cast::powers::power_value;
use crate::spell_cast::test_harness::{Harness, ObservedCombatEvents};
use crate::spell_cast::{CastFailure, creature_cast};
use bevy::prelude::*;
use shared::components::PowerType;
use shared::protocol::{CombatLogEvent, CombatLogKind};

const GARRICK: u32 = 209_057;
const LIGHT_OF_TITANS: u32 = 378_405;
const TITANS_HOT: u32 = 378_412;
const WORD_OF_GLORY: u32 = 85_673;
const JUDGMENT: u32 = 275_779;
const BURST_OF_HATRED: u32 = 124_257;

fn fixture(level: u8) -> (Harness, Entity) {
    let (mut h, paladin) = at_level(GARRICK, level, |app| {
        app.init_resource::<ObservedCombatEvents>();
    });
    h.stop_attack();
    h.advance(3);
    (h, paladin)
}

fn earn_word(h: &mut Harness, paladin: Entity) {
    let world = h.app.world();
    let spell = world
        .resource::<crate::spell_info::SpellStore>()
        .get(WORD_OF_GLORY)
        .unwrap();
    let required: f32 = crate::spell_cast::spell_costs(world, paladin, spell)
        .unwrap()
        .iter()
        .filter_map(|(pool, amount)| {
            (*pool == crate::spell_cast::CostPool::Power(PowerType::HolyPower)).then_some(*amount)
        })
        .sum();
    assert!(
        required > 0.0,
        "fixture must earn the actual native Word cost"
    );
    for _ in 0..1_200 {
        if power_value(h.app.world(), paladin, PowerType::HolyPower)
            .unwrap()
            .0
            >= required
        {
            return;
        }
        let enemy = h.npc;
        match creature_cast(h.app.world_mut(), paladin, JUDGMENT, Some(enemy), false) {
            Ok(()) => {}
            Err(
                CastFailure::OnGlobalCooldown
                | CastFailure::OnCooldown
                | CastFailure::NoCharges
                | CastFailure::InProgress,
            ) => {}
            Err(failure) => panic!("ordinary Judgment generator failed: {failure:?}"),
        }
        h.advance(1);
    }
    panic!("ordinary native Judgment did not earn Word of Glory");
}

fn pressure(h: &mut Harness) {
    let enemy = h.npc;
    for _ in 0..4 {
        creature_cast(h.app.world_mut(), enemy, BURST_OF_HATRED, None, false).unwrap();
        h.advance(100);
    }
}

fn cast_heal(h: &mut Harness, paladin: Entity, target: Entity, spell: u32) -> CombatLogEvent {
    wait_for_gcd(h, paladin, spell);
    let start = h.app.world().resource::<ObservedCombatEvents>().0.len();
    creature_cast(h.app.world_mut(), paladin, spell, Some(target), false).unwrap();
    for _ in 0..200 {
        h.advance(1);
        let direct = h.app.world().resource::<ObservedCombatEvents>().0[start..]
            .iter()
            .find(|event| {
                event.kind == CombatLogKind::Heal
                    && event.spell_id == Some(spell)
                    && event.source == Some(paladin.to_bits())
                    && event.target == Some(target.to_bits())
            })
            .cloned();
        if let Some(direct) = direct {
            h.advance(2);
            return direct;
        }
    }
    panic!("ordinary native healing{spell} did not land");
}

fn owned_hot(h: &Harness, paladin: Entity, target: Entity) -> Option<AuraInstance> {
    h.app
        .world()
        .get::<ActiveAuras>(target)?
        .iter()
        .find(|aura| aura.spell_id == TITANS_HOT && aura.caster == paladin)
        .cloned()
}

fn qualify_word_target(self_target: bool) {
    let (mut h, paladin) = fixture(41);
    let talent = own_aura(&h, paladin, LIGHT_OF_TITANS).expect("native41 learned talent");
    let percent = talent
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert!(percent > 0.0);
    earn_word(&mut h, paladin);
    pressure(&mut h);
    let target = if self_target { paladin } else { h.caster };
    let start = h.app.world().resource::<ObservedCombatEvents>().0.len();
    let direct = cast_heal(&mut h, paladin, target, WORD_OF_GLORY);
    assert!(direct.amount > 0);
    let hot = owned_hot(&h, paladin, target)
        .expect("Word of Glory must produce native378412 on its real patient");
    assert_eq!(hot.duration_ms, 10_000);
    let period = hot
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .period_ms;
    assert_eq!(period, 2_000);
    let count = hot.duration_ms / period;
    let end = hot.expires_at.unwrap();
    let frames = ((end - aura_now(h.app.world())) / frame_seconds(&h)).ceil() as usize + 2;
    h.advance(frames);
    let ticks: Vec<_> = h.app.world().resource::<ObservedCombatEvents>().0[start..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(TITANS_HOT)
                && event.source == Some(paladin.to_bits())
                && event.target == Some(target.to_bits())
        })
        .collect();
    assert_eq!(ticks.len(), count as usize);
    let expected = (direct.amount as f32 * percent / 100.0 / count as f32).round() as i32;
    for tick in ticks {
        assert!(tick.periodic);
        assert!(
            (tick.amount - expected).abs() <= 1,
            "native talent percentage/clock: expected{expected}, {tick:?}"
        );
    }
    assert!(owned_hot(&h, paladin, target).is_none());
}

#[test]
fn light_of_titans_native41_word_on_other_patient_produces_owned_periodic_heal() {
    qualify_word_target(false);
}

#[test]
fn light_of_titans_native41_self_word_without_hostile_dot_keeps_base_amount() {
    qualify_word_target(true);
}

#[test]
fn light_of_titans_native40_unlearned_talent_does_not_produce_hot() {
    let (mut h, paladin) = fixture(40);
    assert!(own_aura(&h, paladin, LIGHT_OF_TITANS).is_none());
    earn_word(&mut h, paladin);
    pressure(&mut h);
    let direct = cast_heal(&mut h, paladin, paladin, WORD_OF_GLORY);
    assert!(direct.amount > 0);
    assert!(owned_hot(&h, paladin, paladin).is_none());
}

#[test]
fn light_of_titans_native41_flash_of_light_does_not_produce_word_hot() {
    let (mut h, paladin) = fixture(41);
    assert!(own_aura(&h, paladin, LIGHT_OF_TITANS).is_some());
    pressure(&mut h);
    let direct = cast_heal(&mut h, paladin, paladin, 19750);
    assert!(direct.amount > 0);
    assert!(owned_hot(&h, paladin, paladin).is_none());
}
