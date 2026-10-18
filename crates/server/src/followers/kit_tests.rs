//! Native 70448 level-32 followers must use more than their four legacy slots.
use super::profile_behavior_tests::spawn_profile_fixture_with_plugins;
use super::*;
use crate::spell_cast::test_harness::Harness;
use shared::components::{Health, Level};

fn level32(entry: u32) -> (Harness, Entity) {
    let (mut h, unit, profile) = spawn_profile_fixture_with_plugins(entry, |_| {});
    h.app.world_mut().get_mut::<Level>(h.caster).unwrap().0 = 32;
    spawning::initialize(h.app.world_mut(), h.caster, unit, profile).unwrap();
    h.attack(h.npc);
    h.advance(3);
    (h, unit)
}

fn wounded(h: &mut Harness, unit: Entity, fraction: f32) {
    let mut health = h.app.world_mut().get_mut::<Health>(unit).unwrap();
    health.current = health.max * fraction;
}

fn uses_spell(h: &mut Harness, unit: Entity, spell: u32) {
    for _ in 0..240 {
        update_ai(h.app.world_mut());
        h.advance(1);
        if h.inbox
            .go
            .iter()
            .any(|go| go.caster == unit.to_bits() && go.spell_id == spell)
        {
            return;
        }
    }
    panic!(
        "follower did not cast native spell {spell}; casts: {:?}",
        h.inbox
            .go
            .iter()
            .filter(|go| go.caster == unit.to_bits())
            .map(|go| go.spell_id)
            .collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "RED: four-slot Garrick never selects Lay on Hands"]
fn follower_kit_garrick32_emergency_heal() {
    let (mut h, unit) = level32(209057);
    wounded(&mut h, unit, 0.15);
    uses_spell(&mut h, unit, 633);
    assert!(h.health(unit) > h.app.world().get::<Health>(unit).unwrap().max * 0.15);
}

#[test]
#[ignore = "RED: four-slot Crenna never maintains Rejuvenation"]
fn follower_kit_crenna32_rejuvenation() {
    let (mut h, unit) = level32(209072);
    let patient = h.caster;
    wounded(&mut h, patient, 0.7);
    uses_spell(&mut h, unit, 774);
    assert!(crate::auras::has_aura(h.app.world(), patient, 774));
}

#[test]
#[ignore = "RED: Breka has no multi-enemy Thunder Clap priority"]
fn follower_kit_breka32_area_attack() {
    let (mut h, unit) = level32(215517);
    uses_spell(&mut h, unit, 6343);
}

#[test]
#[ignore = "RED: Henry never maintains Renew"]
fn follower_kit_henry32_renew() {
    let (mut h, unit) = level32(215011);
    let patient = h.caster;
    wounded(&mut h, patient, 0.7);
    uses_spell(&mut h, unit, 139);
}

#[test]
#[ignore = "RED: Shuja has no Flame Shock maintenance priority"]
fn follower_kit_shuja32_damage_over_time() {
    let (mut h, unit) = level32(209053);
    uses_spell(&mut h, unit, 470411);
}
