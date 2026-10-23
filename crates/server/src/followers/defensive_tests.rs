//! Wounded, actively targeted companions use ordinary defensive spells.
use super::profile_behavior_tests::spawn_profile_fixture;
use super::update_ai;
use crate::spell_cast::effects::{EffectCtx, deal_damage};
use crate::spell_cast::test_harness::Harness;
use crate::spell_info::SpellStore;
use bevy::prelude::Entity;
use shared::components::Health;

fn assert_follower_defensive(
    entry: u32,
    defensive: u32,
    expected_damage: impl FnOnce(&Harness, Entity) -> f32,
) {
    let (mut h, unit, _) = spawn_profile_fixture(entry);
    h.app.world_mut().get_mut::<Health>(unit).unwrap().current -= 200.0;
    let wounded = h.health(unit);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::networking::CombatTarget(Some(unit)));
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(
        h.inbox
            .go
            .iter()
            .any(|cast| cast.caster == unit.to_bits() && cast.spell_id == defensive),
        "companion did not cast defensive{defensive}"
    );
    let before = h.health(unit);
    let damage = expected_damage(&h, unit).round();
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(133)
        .unwrap()
        .clone();
    let context = EffectCtx {
        caster: h.npc,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    deal_damage(h.app.world_mut(), &context, unit, 100.0, false, false);
    assert_eq!(
        before - h.health(unit),
        damage,
        "native defensive mitigation"
    );
    if defensive == 19236 {
        assert!(
            h.health(unit) > wounded,
            "Desperate Prayer must heal the injured priest"
        );
    }
}

macro_rules! defensive_case {
    ($test:ident, $entry:literal, $spell:literal, $damage:literal) => {
        #[test]
        fn $test() {
            assert_follower_defensive($entry, $spell, |_, _| $damage);
        }
    };
}
defensive_case!(follower_garrick_divine_protection, 209057, 498, 80.0);
defensive_case!(follower_breka_shield_wall, 215517, 871, 60.0);
defensive_case!(follower_crenna_barkskin, 209072, 22812, 80.0);
defensive_case!(follower_henry_desperate_prayer, 215011, 19236, 100.0);
defensive_case!(follower_meredy_ice_block, 209059, 45438, 0.0);
defensive_case!(follower_shuja_astral_shift, 209053, 108271, 60.0);
defensive_case!(follower_austin_turtle, 209065, 186265, 70.0);
defensive_case!(follower_kybel_obsidian_scales, 215240, 363916, 70.0);
defensive_case!(follower_bo_fortifying_brew, 227734, 115203, 80.0);
fn lantha_source_damage(h: &Harness, unit: Entity) -> f32 {
    const FEL_ARMOR: u32 = 386124;
    const UNENDING_RESOLVE: u32 = 104773;
    let world = h.app.world();
    let known = world
        .get::<crate::class_progression::KnownSpells>(unit)
        .unwrap();
    assert!(known.contains(FEL_ARMOR));
    assert!(crate::auras::has_aura(world, unit, FEL_ARMOR));
    assert!(crate::auras::has_aura(world, unit, UNENDING_RESOLVE));
    let store = world.resource::<SpellStore>();
    let multiplier: f32 = [FEL_ARMOR, UNENDING_RESOLVE]
        .into_iter()
        .map(|spell| {
            let spell = store.get(spell).unwrap();
            let effect = spell
                .effects
                .iter()
                .find(|effect| effect.aura == 87)
                .unwrap();
            1.0 + effect.base_points / 100.0
        })
        .product();
    //70448 e1020181=-1.5 and e114371=-25:100*.985*.75=73.875 before integer loss.
    100.0 * multiplier
}

#[test]
fn follower_lantha_unending_resolve() {
    assert_follower_defensive(230861, 104773, lantha_source_damage);
}
defensive_case!(follower_ralia_barkskin, 232800, 22812, 80.0);

#[test]
fn follower_does_not_use_defensive_while_healthy_or_not_under_attack() {
    let (mut h, unit, _) = spawn_profile_fixture(209059);
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::networking::CombatTarget(Some(unit)));
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|cast| cast.caster == unit.to_bits() && cast.spell_id == 45438)
    );
    let (mut h, unit, _) = spawn_profile_fixture(209059);
    h.app.world_mut().get_mut::<Health>(unit).unwrap().current -= 200.0;
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::networking::CombatTarget(Some(h.caster)));
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(
        !h.inbox
            .go
            .iter()
            .any(|cast| cast.caster == unit.to_bits() && cast.spell_id == 45438)
    );
}
