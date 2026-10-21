//! FOLLOWER_UTILITY_POLICY: real selected spellbooks and ordinary execution.
use bevy::prelude::*;
use shared::components::{Health, Level, Npc, Position};
use shared::protocol::CombatLogKind;
use super::{kit_tests::{at_level, level32}, profile_behavior_tests::spawn_profile_fixture_with_plugins, utility_tests, update_ai};
use crate::spell_cast::test_harness::{Harness, spawn_npc};

fn add_follower(h: &mut Harness, entry: u32) -> Entity {
    let catalog = h.app.world().resource::<super::FollowerCatalog>();
    let profile = catalog.profiles.iter().find(|p| p.entry == entry).unwrap().clone();
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[entry]);
    h.app.world_mut().resource_mut::<crate::world_data::WorldData>().templates.extend(native.templates);
    let unit = spawn_npc(&mut h.app, 0.0);
    h.app.world_mut().entity_mut(unit).insert(Npc { template_id: entry, name: profile.name.clone() });
    super::spawning::initialize(h.app.world_mut(), h.caster, unit, profile).unwrap();
    unit
}

fn cast_count(h: &Harness, spell: u32) -> usize {
    h.inbox.go.iter().filter(|go| go.spell_id == spell).count()
}

#[test]
fn follower_utility_policy_interrupts_unselected_engaged_caster() {
    let (mut h, unit) = level32(209059);
    let caster = spawn_npc(&mut h.app, 3.0);
    utility_tests::prepare_enemy_spec(&mut h, caster, 63);
    h.app.world_mut().entity_mut(caster).insert(crate::networking::CombatTarget(Some(unit)));
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 133, Some(unit), false).unwrap();
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(h.inbox.log.iter().any(|e| e.kind == CombatLogKind::Interrupt && e.target == Some(caster.to_bits())));
}

#[test]
fn follower_utility_policy_no_double_kick_before_impact() {
    let (mut h, first) = level32(209059);
    let second = add_follower(&mut h, 215240);
    utility_tests::start_enemy_fireball(&mut h, first);
    let target = h.npc;
    for unit in [first, second] {
        let profile = h.app.world().get::<super::Follower>(unit).unwrap().profile.clone();
        super::kit_interrupt::try_interrupt_candidates(h.app.world_mut(), unit, &profile, &[target]);
    }
    h.advance(3);
    assert_eq!(cast_count(&h, 2139) + cast_count(&h, 351338), 1);
    assert_eq!(h.inbox.log.iter().filter(|e| e.kind == CombatLogKind::Interrupt).count(), 1);
}

#[test]
fn follower_utility_policy_control_danger_precedes_selected_damage_cast() {
    let (mut h, unit) = level32(209059);
    utility_tests::start_enemy_fireball(&mut h, unit);
    let dangerous = spawn_npc(&mut h.app, 3.0);
    utility_tests::prepare_enemy_spec(&mut h, dangerous, 266);
    h.app.world_mut().entity_mut(dangerous).insert(crate::networking::CombatTarget(Some(unit)));
    crate::spell_cast::creature_cast(h.app.world_mut(), dangerous, 5782, Some(unit), false).unwrap();
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(h.inbox.log.iter().any(|e| e.kind == CombatLogKind::Interrupt && e.target == Some(dangerous.to_bits()) && e.extra_spell_id == Some(5782)));
}

#[test]
fn follower_utility_policy_stun_unselected_cast_when_kick_cannot_cover() {
    let (mut h, unit) = at_level(209057, 5, |_| {});
    let caster = spawn_npc(&mut h.app, 1.0);
    utility_tests::prepare_enemy_spec(&mut h, caster, 63);
    h.app.world_mut().entity_mut(caster).insert(crate::networking::CombatTarget(Some(unit)));
    crate::spell_cast::creature_cast(h.app.world_mut(), caster, 133, Some(unit), false).unwrap();
    update_ai(h.app.world_mut());
    h.advance(3);
    assert!(crate::auras::has_aura(h.app.world(), caster, 853));
    assert!(h.app.world().get::<shared::casting::CastState>(caster).is_none());
}

#[test]
fn follower_utility_policy_raid_buff_waits_for_out_of_combat() {
    let (mut h, unit) = level32(209059);
    for _ in 0..5 { update_ai(h.app.world_mut()); h.advance(1); }
    assert_eq!(cast_count(&h, 1459), 0, "raid buffs must not replace combat actions");
    h.stop_attack();
    crate::combat::combat_stop(h.app.world_mut(), unit);
    crate::combat::combat_stop(h.app.world_mut(), h.npc);
    for _ in 0..100 { update_ai(h.app.world_mut()); h.advance(1); }
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
    assert_eq!(cast_count(&h, 871), 1, "projected death inside3s must trigger at60%HP");
}

#[test]
fn follower_utility_policy_lust_only_on_native_boss_pull() {
    let (mut h, unit) = at_level(209059, 70, |_| {});
    update_ai(h.app.world_mut()); h.advance(3);
    assert_eq!(cast_count(&h, 80353), 0);
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[46264]);
    assert!(native.templates[&46264].flags_extra & 0x1000_0000 != 0);
    h.app.world_mut().resource_mut::<crate::world_data::WorldData>().templates.extend(native.templates);
    h.app.world_mut().entity_mut(h.npc).insert(Npc { template_id: 46264, name: "Hogger".into() });
    for _ in 0..10 { update_ai(h.app.world_mut()); h.advance(1); }
    assert_eq!(cast_count(&h, 80353), 1);
    assert!(crate::auras::has_aura(h.app.world(), unit, 80353));
}

#[test]
fn follower_utility_policy_battle_res_prefers_dead_healer() {
    let (mut h, unit) = at_level(209072, 32, |_| {});
    let healer = add_follower(&mut h, 215011);
    let tank = add_follower(&mut h, 209057);
    for dead in [healer, tank] {
        h.app.world_mut().get_mut::<Health>(dead).unwrap().current = 0.0;
        h.app.world_mut().entity_mut(dead).insert(crate::combat::CorpseDelay(60.0));
    }
    for _ in 0..80 { update_ai(h.app.world_mut()); h.advance(1); }
    assert!(h.health(healer) > 0.0, "Rebirth must actually resurrect the healer");
    assert!(h.inbox.go.iter().any(|go| go.caster == unit.to_bits() && go.spell_id == 20484));
    assert_eq!(h.health(tank), 0.0, "native cooldown must prevent second res");
}

#[test]
fn follower_utility_policy_movement_buff_for_distant_pursuit() {
    let (mut h, unit) = at_level(215240, 32, |_| {});
    h.app.world_mut().get_mut::<Position>(h.npc).unwrap().x = 80.0;
    for _ in 0..10 { update_ai(h.app.world_mut()); h.advance(1); }
    assert_eq!(cast_count(&h, 358267), 1);
    assert!(crate::auras::has_aura(h.app.world(), unit, 358267));
}

#[test]
fn follower_utility_policy_never_taunts_as_dps() {
    let (mut h, unit) = at_level(227734, 70, |_| {});
    for _ in 0..40 { update_ai(h.app.world_mut()); h.advance(1); }
    assert_eq!(cast_count(&h, 115546), 0);
    assert!(h.app.world().get::<Level>(unit).is_some());
}

#[test]
fn follower_utility_policy_no_damage_into_someone_elses_cc() {
    let (mut h, unit) = spawn_profile_fixture_with_plugins(209059, |_| {});
    h.app.world_mut().get_mut::<Level>(h.caster).unwrap().0 = 32;
    let db = rusqlite::Connection::open_with_flags(crate::test_data::world_db_path(), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    h.app.insert_resource(super::catalog::load_catalog(&db).unwrap());
    let spell = h.app.world().resource::<crate::spell_info::SpellStore>().get(118).unwrap();
    let mask = crate::auras::aura_effect_mask(spell);
    crate::auras::apply_aura(h.app.world_mut(), &crate::auras::AuraApplication { caster: h.caster, target: h.npc, spell_id: 118, effect_mask: mask, source: crate::auras::AuraSource::Cast });
    h.attack(h.npc); h.advance(3);
    let before = h.health(h.npc);
    for _ in 0..40 { update_ai(h.app.world_mut()); h.advance(1); }
    assert_eq!(h.health(h.npc), before);
    assert!(crate::auras::has_aura(h.app.world(), h.npc, 118));
    assert!(!h.inbox.log.iter().any(|e| e.kind == CombatLogKind::Damage && e.source == Some(unit.to_bits())));
}
