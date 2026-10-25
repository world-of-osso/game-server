//! SimC76f8118 Swiftmend::impact order; native392410 disables consumption.
use super::kit_tests::at_level;
use crate::auras::ActiveAuras;
use crate::spell_cast::test_harness::Harness;
use crate::spell_cast::{SpellHistory, creature_cast, history_now};
use crate::spell_info::SpellStore;
use bevy::prelude::*;
use shared::casting::CastState;
use shared::components::Health;
use shared::protocol::CombatLogKind;

const CRENNA: u32 = 209072;
const SWIFTMEND: u32 = 18562;
const REJUVENATION: u32 = 774;
const REGROWTH: u32 = 8936;
const VERDANT_INFUSION: u32 = 392410;

fn owns_hot(h: &Harness, healer: Entity, spell: u32) -> bool {
    h.app
        .world()
        .get::<ActiveAuras>(healer)
        .is_some_and(|auras| {
            auras
                .iter()
                .any(|aura| aura.spell_id == spell && aura.caster == healer)
        })
}

fn wait_for_native_cast_and_gcd(h: &mut Harness, healer: Entity) {
    for _ in 0..200 {
        h.advance(1);
        let world = h.app.world();
        let spell = world.resource::<SpellStore>().get(SWIFTMEND).unwrap();
        let on_gcd = world
            .get::<SpellHistory>(healer)
            .is_some_and(|history| history.on_gcd(spell, history_now(world)));
        if world.get::<CastState>(healer).is_none() && !on_gcd {
            return;
        }
    }
    panic!("ordinary source cast/GCD did not finish");
}

fn apply_own_hot(h: &mut Harness, healer: Entity, spell: u32) {
    creature_cast(h.app.world_mut(), healer, spell, Some(healer), false).unwrap();
    wait_for_native_cast_and_gcd(h, healer);
    assert!(owns_hot(h, healer, spell));
}

fn cast_native_swiftmend_and_observe_heal(h: &mut Harness, healer: Entity) {
    let mut health = h.app.world_mut().get_mut::<Health>(healer).unwrap();
    health.current = health.max * 0.3;
    let before = h.health(healer);
    creature_cast(h.app.world_mut(), healer, SWIFTMEND, Some(healer), false).unwrap();
    h.advance(3);
    assert!(h.health(healer) > before);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(SWIFTMEND)
                && event.source == Some(healer.to_bits())
                && event.target == Some(healer.to_bits()))
    );
}

#[test]
fn follower_swiftmend_consumption_preserves_another_casters_rejuvenation() {
    let (mut h, healer) = at_level(CRENNA, 17, |_| {});
    let profile = h
        .app
        .world()
        .get::<super::Follower>(healer)
        .unwrap()
        .profile
        .clone();
    let other = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    h.app
        .world_mut()
        .entity_mut(other)
        .insert(shared::components::Npc {
            template_id: profile.entry,
            name: profile.name.clone(),
        });
    let owner = h.caster;
    super::spawning::initialize(h.app.world_mut(), owner, other, profile).unwrap();
    creature_cast(h.app.world_mut(), other, REJUVENATION, Some(healer), false).unwrap();
    wait_for_native_cast_and_gcd(&mut h, other);
    apply_own_hot(&mut h, healer, REJUVENATION);
    cast_native_swiftmend_and_observe_heal(&mut h, healer);
    assert!(!owns_hot(&h, healer, REJUVENATION));
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(healer)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == REJUVENATION && aura.caster == other),
        "consumption must leave the other caster's actual instance intact"
    );
}

#[test]
fn follower_swiftmend_consumption_level17_consumes_own_rejuvenation() {
    let (mut h, healer) = at_level(CRENNA, 17, |_| {});
    assert!(!crate::auras::has_aura(
        h.app.world(),
        healer,
        VERDANT_INFUSION
    ));
    apply_own_hot(&mut h, healer, REJUVENATION);
    cast_native_swiftmend_and_observe_heal(&mut h, healer);
    assert!(
        !owns_hot(&h, healer, REJUVENATION),
        "native Swiftmend must consume its caster's Rejuvenation without Verdant Infusion"
    );
}

#[test]
fn follower_swiftmend_consumption_level32_native_verdant_preserves_rejuvenation() {
    let (mut h, healer) = at_level(CRENNA, 32, |_| {});
    assert!(crate::auras::has_aura(
        h.app.world(),
        healer,
        VERDANT_INFUSION
    ));
    apply_own_hot(&mut h, healer, REJUVENATION);
    cast_native_swiftmend_and_observe_heal(&mut h, healer);
    assert!(
        owns_hot(&h, healer, REJUVENATION),
        "actual learned Verdant Infusion must preserve the HoT"
    );
}

#[test]
fn follower_swiftmend_consumption_level17_consumes_regrowth_before_rejuvenation() {
    let (mut h, healer) = at_level(CRENNA, 17, |_| {});
    assert!(!crate::auras::has_aura(
        h.app.world(),
        healer,
        VERDANT_INFUSION
    ));
    apply_own_hot(&mut h, healer, REJUVENATION);
    apply_own_hot(&mut h, healer, REGROWTH);
    assert!(owns_hot(&h, healer, REJUVENATION));
    cast_native_swiftmend_and_observe_heal(&mut h, healer);
    assert!(
        !owns_hot(&h, healer, REGROWTH),
        "source priority must consume Regrowth first"
    );
    assert!(
        owns_hot(&h, healer, REJUVENATION),
        "Swiftmend consumes only one qualifying own HoT"
    );
}
