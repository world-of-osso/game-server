//! Decisions over learned native Crenna spells under actual damage pressure.
use super::*;
use crate::auras::{ActiveAuras, AuraInstance, aura_now};
use crate::spell_cast::test_harness::Harness;
use crate::spell_info::SpellStore;
use shared::casting::CastType;
use shared::components::Mana;
use shared::protocol::CombatLogKind;

fn own_aura(h: &Harness, caster: Entity, target: Entity, spell: u32) -> Option<AuraInstance> {
    h.app
        .world()
        .get::<ActiveAuras>(target)?
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == caster)
        .cloned()
}

fn damage(h: &mut Harness, target: Entity, fraction: f32) {
    let health = h.app.world().get::<Health>(target).unwrap();
    let before = health.current;
    let amount = health.max * fraction;
    let source = crate::level_scaling::UnitScaling::of(h.app.world(), h.npc);
    let recipient = crate::level_scaling::UnitScaling::of(h.app.world(), target);
    let health_per_damage = crate::level_scaling::health_loss(&source, &recipient, 1.0);
    assert!(health_per_damage > 0.0);
    let enemy = h.npc;
    crate::combat::deal_direct_damage(h.app.world_mut(), enemy, target, amount / health_per_damage);
    let actual = before - h.health(target);
    assert!(
        h.health(target) > 0.0,
        "native pressure must not kill its fixture patient"
    );
    assert!(
        (actual - amount).abs() <= amount * 0.001 + 1.0,
        "native pressure must reach the requested recipient-health deficit: {actual} vs {amount}"
    );
}

fn decide(h: &mut Harness, healer: Entity, party: &[Entity]) {
    let follower = h.app.world().get::<Follower>(healer).unwrap().clone();
    update(h.app.world_mut(), healer, &follower, party, None);
}

#[test]
fn resto_policy_late_tank_lifebloom_refresh_uses_native_bloom() {
    let (mut h, healer, party) = super::super::kit_abundance_tests::native_party(32);
    let tank = party[2];
    assert!(super::is_tank(h.app.world(), tank));
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 33763, Some(tank), false).unwrap();
    h.advance(1);
    let old = own_aura(&h, healer, tank, 33763).unwrap();
    let refresh_at = old.expires_at.unwrap() - f64::from(old.duration_ms) / 1000.0 * 0.2;
    while aura_now(h.app.world()) < refresh_at {
        h.advance(1);
    }
    damage(&mut h, tank, 0.1);
    let first = h.inbox.log.len();
    decide(&mut h, healer, &[tank, healer]);
    h.advance(1);
    let refreshed = own_aura(&h, healer, tank, 33763).unwrap();
    assert!(
        refreshed.expires_at > old.expires_at,
        "AI must refresh own near-expiry Lifebloom despite generic missing-aura condition"
    );
    assert!(
        h.inbox.log[first..]
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(33778)
                && event.source == Some(healer.to_bits())
                && event.target == Some(tank.to_bits())),
        "refresh must invoke actual native bloom, not a synthetic heal"
    );
}

#[test]
fn resto_policy_ironbark_protects_actually_attacked_tank_before_filler() {
    let (mut h, healer, party) = super::super::kit_abundance_tests::native_party(32);
    let tank = party[2];
    assert!(super::is_tank(h.app.world(), tank));
    let enemy = h.npc;
    h.app.world_mut().entity_mut(enemy).insert((
        shared::components::ThreatTable::default(),
        crate::creature_ai::CreatureState::Idle,
    ));
    crate::creature_aggro::add_threat(h.app.world_mut(), enemy, tank, 100.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(enemy)
            .and_then(|target| target.0),
        Some(tank)
    );
    damage(&mut h, tank, 0.6);
    decide(&mut h, healer, &[tank, healer]);
    assert!(
        own_aura(&h, healer, tank, 102342).is_some(),
        "native Ironbark must protect the attacked tank under real pressure"
    );
}

#[test]
fn resto_policy_natures_cure_removes_native_magic_before_routine_healing() {
    let (mut h, healer) = super::super::kit_tests::level32(209072);
    let target = h.caster;
    let enemy = h.npc;
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        enemy,
        589,
        Some(target),
        1
    ));
    assert!(crate::auras::has_aura(h.app.world(), target, 589));
    damage(&mut h, target, 0.05);
    decide(&mut h, healer, &[target, healer]);
    h.advance(1);
    assert!(
        !crate::auras::has_aura(h.app.world(), target, 589),
        "routine healing must not starve Nature's Cure's actual native Magic dispel"
    );
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Dispel
                && event.source == Some(healer.to_bits()))
    );
}

#[test]
fn resto_policy_high_level_tranquility_channels_for_stacked_group_pressure() {
    let (mut h, healer, party) = super::super::kit_abundance_tests::native_party(70);
    for target in party.into_iter().filter(|target| *target != healer).take(3) {
        damage(&mut h, target, 0.45);
    }
    decide(&mut h, healer, &party);
    assert!(
        h.app
            .world()
            .get::<CastState>(healer)
            .is_some_and(|cast| cast.spell_id == 740 && cast.cast_type == CastType::Channel),
        "learned Tranquility must handle stacked multi-patient damage while its caster is safe"
    );
    h.advance(50);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.kind == CombatLogKind::Heal
                && event.spell_id == Some(157982)
                && event.source == Some(healer.to_bits())
                && event.overflow < event.amount),
        "channel must produce useful native Tranquility child healing"
    );
}

#[test]
fn resto_policy_high_level_low_mana_uses_native_innervate() {
    let (mut h, healer) = super::super::kit_tests::at_level(209072, 70, |_| {});
    let target = h.caster;
    let max = h.app.world().get::<Mana>(healer).unwrap().max;
    h.app.world_mut().get_mut::<Mana>(healer).unwrap().current = max * 0.15;
    damage(&mut h, target, 0.1);
    decide(&mut h, healer, &[target, healer]);
    assert!(
        own_aura(&h, healer, healer, 29166).is_some(),
        "learned70 native Innervate must sustain low-mana healing; do not fake zero-cost spells or grant mana"
    );
}

#[test]
fn resto_policy_native_channel_is_unsafe_when_caster_forecast_dies_before_finish() {
    let (mut h, healer, party) = super::super::kit_abundance_tests::native_party(40);
    for target in party {
        damage(&mut h, target, 0.45);
    }
    let risk = super::super::healer_triage::read_patient(h.app.world(), healer).unwrap();
    let spell = h.app.world().resource::<SpellStore>().get(740).unwrap();
    let duration = crate::spell_cast::duration_ms(h.app.world(), healer, spell) as f32 / 1000.0;
    assert!(
        risk.projected_death_seconds < duration,
        "native pressure must establish unsafe channel survival before testing admission"
    );
    let rows = h
        .app
        .world()
        .resource::<FollowerCatalog>()
        .kits
        .iter()
        .filter(|entry| entry.spec_id == 105 && entry.spell_id == 740 && entry.available(105, 40))
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    let follower = h.app.world().get::<Follower>(healer).unwrap().clone();
    super::super::healer_triage::update(h.app.world_mut(), healer, &follower, &party, &rows);
    assert!(
        h.app
            .world()
            .get::<CastState>(healer)
            .is_none_or(|cast| cast.spell_id != 740),
        "actual recent damage predicts caster death before native channel ends; do not start the channel"
    );
}
