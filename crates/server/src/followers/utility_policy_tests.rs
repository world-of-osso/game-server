//! FOLLOWER_UTILITY_POLICY: real selected spellbooks and ordinary execution.
use super::{
    kit_tests::{at_level, level32},
    profile_behavior_tests::spawn_profile_fixture_with_plugins,
    update_ai, utility_tests,
};
use crate::spell_cast::test_harness::{Harness, spawn_npc};
use bevy::prelude::*;
use shared::components::{Health, Level, Npc, Position};
use shared::protocol::CombatLogKind;

fn add_follower(h: &mut Harness, entry: u32) -> Entity {
    let catalog = h.app.world().resource::<super::FollowerCatalog>();
    let profile = catalog
        .profiles
        .iter()
        .find(|p| p.entry == entry)
        .unwrap()
        .clone();
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[entry]);
    h.app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .extend(native.templates);
    let unit = spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().entity_mut(unit).insert(Npc {
        template_id: entry,
        name: profile.name.clone(),
    });
    super::spawning::initialize(h.app.world_mut(), h.caster, unit, profile).unwrap();
    unit
}

fn cast_count(h: &Harness, spell: u32) -> usize {
    h.inbox.go.iter().filter(|go| go.spell_id == spell).count()
}

#[test]
fn follower_utility_policy_native_resurrection_effect_restores_a_dead_follower() {
    let (mut h, unit) = level32(209072);
    let healer = add_follower(&mut h, 215011);
    h.app
        .world_mut()
        .entity_mut(healer)
        .insert(crate::combat::CorpseDelay(60.0));
    crate::combat::kill_unit(h.app.world_mut(), h.npc, healer);
    h.advance(2);
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(healer)
            .is_some()
    );
    crate::spell_cast::creature_cast(h.app.world_mut(), unit, 20484, Some(healer), false).unwrap();
    h.advance(80);
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(20484)
        .unwrap();
    let percent = spell
        .effects
        .iter()
        .find(|effect| effect.effect == 18)
        .unwrap()
        .base_points;
    let max = h.app.world().get::<Health>(healer).unwrap().max;
    assert!(h.health(healer) >= (max * percent / 100.0).floor());
    assert!(
        h.app
            .world()
            .get::<crate::combat::DeathState>(healer)
            .is_none()
    );
    assert!(h.app.world().get::<crate::combat::Corpse>(healer).is_none());
}

#[test]
fn follower_utility_policy_native_resurrection_from_follower_retains_player_acceptance() {
    use lightyear::prelude::{MessageReceiver, MessageSender};
    use shared::protocol::{ResurrectionOffer, ResurrectionResponse};
    let (mut h, unit) = level32(209072);
    h.stop_attack();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::death::PlayerDeathState {
            state: shared::death::DeathState::Dead,
            corpse: None,
            graveyard: None,
        });
    h.app
        .world_mut()
        .get_mut::<Health>(h.caster)
        .unwrap()
        .current = 0.0;
    h.app.world_mut().entity_mut(h.connection).insert((
        MessageSender::<ResurrectionOffer>::default(),
        MessageReceiver::<ResurrectionOffer>::default(),
    ));
    crate::spell_cast::creature_cast(h.app.world_mut(), unit, 20484, Some(h.caster), false)
        .unwrap();
    let mut offers = Vec::new();
    for _ in 0..80 {
        h.advance(1);
        offers.extend(crate::spell_cast::test_harness::drain::<ResurrectionOffer>(
            &mut h.app,
            h.connection,
        ));
        if !offers.is_empty() {
            break;
        }
    }
    assert_eq!(offers.len(), 1);
    assert_eq!(offers[0].caster_name, "Crenna Earth-Daughter");
    assert_eq!(h.health(h.caster), 0.0, "player must not auto-accept");
    crate::death::resurrection::respond(
        h.app.world_mut(),
        h.caster,
        ResurrectionResponse {
            caster: unit.to_bits(),
            spell_id: 20484,
            accept: true,
        },
    );
    assert!(h.health(h.caster) > 0.0);
}

#[test]
fn follower_utility_policy_delayed_shield_does_not_spend_a_second_kick() {
    let (mut h, first) = at_level(209057, 26, |_| {});
    let second = add_follower(&mut h, 215240);
    utility_tests::start_enemy_fireball(&mut h, first);
    h.app.world_mut().get_mut::<Position>(first).unwrap().x = -15.0;
    let target = h.npc;
    let profile = h
        .app
        .world()
        .get::<super::Follower>(first)
        .unwrap()
        .profile
        .clone();
    assert!(super::kit_interrupt::try_interrupt_candidates(
        h.app.world_mut(),
        first,
        &profile,
        &[target]
    ));
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(target)
            .is_some(),
        "native shield must still be in flight"
    );
    let profile = h
        .app
        .world()
        .get::<super::Follower>(second)
        .unwrap()
        .profile
        .clone();
    assert!(!super::kit_interrupt::try_interrupt_candidates(
        h.app.world_mut(),
        second,
        &profile,
        &[target]
    ));
    h.advance(30);
    assert_eq!(cast_count(&h, 351338), 0);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.kind == CombatLogKind::Interrupt && e.spell_id == Some(31935))
    );
}

#[test]
fn follower_utility_policy_rotates_ready_party_kicks() {
    let (mut h, first) = level32(209059);
    let second = add_follower(&mut h, 215240);
    utility_tests::start_enemy_fireball(&mut h, first);
    let target = h.npc;
    let profile = h
        .app
        .world()
        .get::<super::Follower>(first)
        .unwrap()
        .profile
        .clone();
    assert!(super::kit_interrupt::try_interrupt_candidates(
        h.app.world_mut(),
        first,
        &profile,
        &[target]
    ));
    h.advance(600);
    utility_tests::start_enemy_fireball(&mut h, first);
    let party = [h.caster, first, second];
    assert!(super::utility_interrupt::try_party(
        h.app.world_mut(),
        h.caster,
        &party,
        &[target]
    ));
    h.advance(3);
    assert_eq!(cast_count(&h, 351338), 1);
    assert_eq!(cast_count(&h, 2139), 1);
}

#[test]
fn follower_utility_policy_interrupts_unselected_engaged_caster() {
    let (mut h, unit) = level32(209059);
    let caster = spawn_npc(&mut h.app, 3.0);
    utility_tests::prepare_enemy_spec(&mut h, caster, 63);
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::networking::CombatTarget(Some(unit)));
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 133, Some(unit), false).unwrap();
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.kind == CombatLogKind::Interrupt && e.target == Some(caster.to_bits()))
    );
}

#[test]
fn follower_utility_policy_no_double_kick_before_impact() {
    let (mut h, first) = level32(209059);
    let second = add_follower(&mut h, 215240);
    utility_tests::start_enemy_fireball(&mut h, first);
    let target = h.npc;
    for unit in [first, second] {
        let profile = h
            .app
            .world()
            .get::<super::Follower>(unit)
            .unwrap()
            .profile
            .clone();
        super::kit_interrupt::try_interrupt_candidates(
            h.app.world_mut(),
            unit,
            &profile,
            &[target],
        );
    }
    h.advance(3);
    assert_eq!(cast_count(&h, 2139) + cast_count(&h, 351338), 1);
    assert_eq!(
        h.inbox
            .log
            .iter()
            .filter(|e| e.kind == CombatLogKind::Interrupt)
            .count(),
        1
    );
}

#[test]
fn follower_utility_policy_control_danger_precedes_selected_damage_cast() {
    let (mut h, unit) = level32(209059);
    utility_tests::start_enemy_fireball(&mut h, unit);
    let dangerous = spawn_npc(&mut h.app, 3.0);
    utility_tests::prepare_enemy_spec(&mut h, dangerous, 63);
    h.app
        .world_mut()
        .entity_mut(dangerous)
        .insert(crate::networking::CombatTarget(Some(unit)));
    crate::spell_cast::creature_cast(h.app.world_mut(), dangerous, 118, Some(unit), false).unwrap();
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|e| e.kind == CombatLogKind::Interrupt
                && e.target == Some(dangerous.to_bits())
                && e.extra_spell_id == Some(118))
    );
}

#[test]
fn follower_utility_policy_stun_unselected_cast_when_kick_cannot_cover() {
    let (mut h, unit) = at_level(209057, 5, |_| {});
    let caster = spawn_npc(&mut h.app, 1.0);
    utility_tests::prepare_enemy_spec(&mut h, caster, 63);
    h.app
        .world_mut()
        .entity_mut(caster)
        .insert(crate::networking::CombatTarget(Some(unit)));
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 133, Some(unit), false).unwrap();
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), caster, 853));
    assert!(
        h.app
            .world()
            .get::<shared::casting::CastState>(caster)
            .is_none()
    );
}

#[test]
fn follower_utility_policy_dispel_control_before_lower_health_periodic_damage() {
    let (mut h, unit) = at_level(215011, 70, |_| {});
    let controlled = add_follower(&mut h, 209059);
    let patient = h.caster;
    let max = h.app.world().get::<Health>(patient).unwrap().max;
    h.app
        .world_mut()
        .get_mut::<Health>(patient)
        .unwrap()
        .current = max * 0.75;
    for (target, spell_id) in [(patient, 589), (controlled, 118)] {
        let spell = h
            .app
            .world()
            .resource::<crate::spell_info::SpellStore>()
            .get(spell_id)
            .unwrap();
        let effect_mask = crate::auras::aura_effect_mask(spell);
        crate::auras::apply_aura(
            h.app.world_mut(),
            &crate::auras::AuraApplication {
                caster: h.npc,
                target,
                spell_id,
                effect_mask,
                source: crate::auras::AuraSource::Cast,
            },
        );
    }
    update_ai(h.app.world_mut());
    h.advance(3);
    let dispelled = h
        .inbox
        .log
        .iter()
        .find(|e| e.kind == CombatLogKind::Dispel && e.source == Some(unit.to_bits()))
        .expect("native Purify dispel");
    assert_eq!(dispelled.target, Some(controlled.to_bits()));
    assert_eq!(dispelled.extra_spell_id, Some(118));
}

#[test]
fn follower_utility_policy_raid_buff_rejects_combat_status_without_an_enemy() {
    let (mut h, unit) = level32(209059);
    let entry = h
        .app
        .world()
        .resource::<super::FollowerCatalog>()
        .kits
        .iter()
        .find(|entry| entry.spec_id == 63 && entry.spell_id == 1459)
        .unwrap()
        .clone();
    h.app
        .world_mut()
        .entity_mut(unit)
        .insert(shared::components::CombatStatus(true));
    let party = [h.caster, unit];
    assert!(
        !super::kit_ai::conditions_met(h.app.world_mut(), unit, unit, &party, &[], &entry),
        "an empty candidate list is not proof that a party member left combat"
    );
}

#[test]
fn follower_utility_policy_raid_buff_waits_for_out_of_combat() {
    let (mut h, unit) = level32(209059);
    for _ in 0..5 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert_eq!(
        cast_count(&h, 1459),
        0,
        "raid buffs must not replace combat actions"
    );
    h.stop_attack();
    crate::combat::combat_stop(h.app.world_mut(), unit);
    crate::combat::combat_stop(h.app.world_mut(), h.npc);
    for _ in 0..100 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(crate::auras::has_aura(h.app.world(), unit, 1459));
}

#[test]
fn follower_utility_policy_projected_death_uses_defensive_above_half_health() {
    let (mut h, unit) = at_level(215517, 70, |_| {});
    update_ai(h.app.world_mut());
    h.advance(3);
    // Observed net loss over the real manual clock, not an invented spell coefficient.
    let max = h.app.world().get::<Health>(unit).unwrap().max;
    h.app.world_mut().get_mut::<Health>(unit).unwrap().current = max * 0.60;
    update_ai(h.app.world_mut());
    h.advance(3);
    assert_eq!(
        cast_count(&h, 871),
        1,
        "projected death inside3s must trigger at60%HP"
    );
}

#[test]
fn follower_utility_policy_lust_only_on_native_boss_pull() {
    let (mut h, unit) = at_level(209059, 70, |_| {});
    update_ai(h.app.world_mut());
    h.advance(3);
    assert_eq!(cast_count(&h, 80353), 0);
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[46264]);
    assert!(native.templates[&46264].flags_extra & 0x1000_0000 != 0);
    h.app
        .world_mut()
        .resource_mut::<crate::world_data::WorldData>()
        .templates
        .extend(native.templates);
    h.app.world_mut().entity_mut(h.npc).insert(Npc {
        template_id: 46264,
        name: "Hogger".into(),
    });
    for _ in 0..10 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert_eq!(cast_count(&h, 80353), 1);
    assert!(crate::auras::has_aura(h.app.world(), unit, 80353));
}

#[test]
fn follower_utility_policy_battle_res_prefers_dead_healer() {
    let (mut h, unit) = at_level(209072, 32, |_| {});
    let healer = add_follower(&mut h, 215011);
    let tank = add_follower(&mut h, 209057);
    for dead in [healer, tank] {
        h.app
            .world_mut()
            .entity_mut(dead)
            .insert(crate::combat::CorpseDelay(60.0));
        crate::combat::kill_unit(h.app.world_mut(), h.npc, dead);
    }
    for _ in 0..80 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert!(
        h.health(healer) > 0.0,
        "Rebirth must actually resurrect the healer"
    );
    assert!(
        h.inbox
            .go
            .iter()
            .any(|go| go.caster == unit.to_bits() && go.spell_id == 20484)
    );
    assert_eq!(
        h.health(tank),
        0.0,
        "native cooldown must prevent second res"
    );
}

#[test]
fn follower_utility_policy_movement_buff_for_distant_pursuit() {
    let (mut h, unit) = at_level(215240, 32, |_| {});
    h.app.world_mut().get_mut::<Position>(h.npc).unwrap().x = 80.0;
    for _ in 0..10 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert_eq!(cast_count(&h, 358267), 1);
    assert!(crate::auras::has_aura(h.app.world(), unit, 358267));
}

#[test]
fn follower_utility_policy_never_taunts_as_dps() {
    let (mut h, unit) = at_level(227734, 70, |_| {});
    for _ in 0..40 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert_eq!(cast_count(&h, 115546), 0);
    assert!(h.app.world().get::<Level>(unit).is_some());
}

#[test]
fn follower_utility_policy_no_damage_into_someone_elses_cc() {
    let (mut h, unit, _) = spawn_profile_fixture_with_plugins(209059, |_| {});
    h.app.world_mut().get_mut::<Level>(h.caster).unwrap().0 = 32;
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(super::catalog::load_catalog(&db).unwrap());
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(118)
        .unwrap();
    let mask = crate::auras::aura_effect_mask(spell);
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster: h.caster,
            target: h.npc,
            spell_id: 118,
            effect_mask: mask,
            source: crate::auras::AuraSource::Cast,
        },
    );
    h.app
        .world_mut()
        .entity_mut(h.npc)
        .insert(crate::networking::CombatTarget(Some(h.caster)));
    // Only follower behavior is under test; the owner must not white-swing the sheep.
    h.stop_attack();
    h.advance(3);
    let before = h.health(h.npc);
    for _ in 0..40 {
        update_ai(h.app.world_mut());
        h.advance(1);
    }
    assert_eq!(h.health(h.npc), before);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 118));
    assert!(
        !h.inbox
            .log
            .iter()
            .any(|e| e.kind == CombatLogKind::Damage && e.source == Some(unit.to_bits()))
    );
}
