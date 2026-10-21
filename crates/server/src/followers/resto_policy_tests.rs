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
fn resto_policy_barkskin_protects_actually_attacked_healer_before_throughput() {
    let (mut h, healer) = super::super::kit_tests::at_level(209072, 13, |_| {});
    let enemy = h.npc;
    h.app.world_mut().entity_mut(enemy).insert((
        shared::components::ThreatTable::default(),
        crate::creature_ai::CreatureState::Idle,
    ));
    crate::creature_aggro::add_threat(h.app.world_mut(), enemy, healer, 100.0);
    assert_eq!(
        h.app
            .world()
            .get::<crate::networking::CombatTarget>(enemy)
            .and_then(|target| target.0),
        Some(healer)
    );
    damage(&mut h, healer, 0.65);
    let owner = h.caster;
    decide(&mut h, healer, &[healer, owner]);
    assert!(
        own_aura(&h, healer, healer, 22812).is_some(),
        "actual learned Barkskin must protect the attacked low-health healer before throughput preparation"
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
    if !h
        .app
        .world()
        .get::<CastState>(healer)
        .is_some_and(|cast| cast.spell_id == 740 && cast.cast_type == CastType::Channel)
    {
        let world = h.app.world();
        let patients = party
            .iter()
            .filter_map(|unit| super::super::healer_triage::read_patient(world, *unit))
            .collect::<Vec<_>>();
        let entries = world
            .resource::<FollowerCatalog>()
            .kits
            .iter()
            .filter(|entry| entry.spec_id == 105 && entry.spell_id == 740)
            .cloned()
            .collect::<Vec<_>>();
        let spell = world.resource::<SpellStore>().get(740).unwrap();
        eprintln!(
            "Tranquility diagnostic: patients={patients:?} clusters={:?} recovery={:?} costs={:?} casts={:?}",
            super::super::healer_context::read_clusters(world, healer, &entries, &patients),
            crate::spell_cast::read_healer_recovery(world, healer, spell),
            crate::spell_cast::spell_costs(world, healer, spell),
            h.inbox
                .go
                .iter()
                .filter(|event| event.caster == healer.to_bits())
                .map(|event| event.spell_id)
                .collect::<Vec<_>>()
        );
    }
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
fn resto_policy_native_tree_form_boosts_make_real_regrowth_instant_and_end_with_form() {
    let (mut h, healer) = super::super::kit_tests::at_level(209072, 70, |_| {});
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 33891, Some(healer), false)
        .unwrap();
    assert!(own_aura(&h, healer, healer, 33891).is_some());
    assert!(
        own_aura(&h, healer, healer, 5420).is_some()
            && own_aura(&h, healer, healer, 81097).is_some(),
        "TC form2 adds its real5420/81097 boosts, not just the33891 visual/form aura"
    );
    super::super::kit_clearcasting_tests::wait_for_gcd(&mut h, healer, 8936);
    damage(&mut h, healer, 0.25);
    let before = h.health(healer);
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 8936, Some(healer), false).unwrap();
    assert!(h.app.world().get::<CastState>(healer).is_none());
    assert!(
        h.health(healer) > before,
        "boosted native Regrowth must heal immediately"
    );
    crate::auras::remove_auras(
        h.app.world_mut(),
        healer,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == 33891,
    );
    assert!(
        own_aura(&h, healer, healer, 5420).is_none()
            && own_aura(&h, healer, healer, 81097).is_none()
    );
    super::super::kit_clearcasting_tests::wait_for_gcd(&mut h, healer, 8936);
    damage(&mut h, healer, 0.1);
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 8936, Some(healer), false).unwrap();
    assert!(
        h.app
            .world()
            .get::<CastState>(healer)
            .is_some_and(|cast| cast.spell_id == 8936 && cast.cast_type == CastType::Normal),
        "leaving the native form must restore ordinary Regrowth cast time"
    );
}

#[test]
fn resto_policy_learned_tree_sustains_group_pressure_after_native_tranquility_cooldown() {
    let (mut h, healer, party) = super::super::kit_abundance_tests::native_party(70);
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 740, Some(healer), false).unwrap();
    for _ in 0..200 {
        h.advance(1);
        if h.app.world().get::<CastState>(healer).is_none() {
            break;
        }
    }
    assert!(h.app.world().get::<CastState>(healer).is_none());
    for target in &party[1..4] {
        damage(&mut h, *target, 0.45);
    }
    let until = aura_now(h.app.world()) + 3.5;
    while aura_now(h.app.world()) < until {
        h.advance(1);
    }
    for target in &party[1..4] {
        let patient = super::super::healer_triage::read_patient(h.app.world(), *target).unwrap();
        let health = h.app.world().get::<Health>(*target).unwrap();
        assert!(!patient.emergency && health.current < health.max * 0.7);
    }
    let spell = h.app.world().resource::<SpellStore>().get(740).unwrap();
    assert!(
        crate::spell_cast::read_healer_recovery(h.app.world(), healer, spell).is_err(),
        "the real previous native channel must have spent Tranquility's cooldown"
    );
    decide(&mut h, healer, &party);
    assert!(
        own_aura(&h, healer, healer, 33891).is_some(),
        "learned Tree of Life must use its native aura for sustained group pressure when Tranquility is unavailable"
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

#[test]
fn resto_policy_swiftmend_soul_is_spent_on_useful_empowered_regrowth() {
    let (mut h, healer, party) = super::super::kit_abundance_tests::native_party(32);
    let tank = party[2];
    super::super::kit_soul_of_forest_tests::cast_setup_swiftmend(&mut h, healer, tank);
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 33763, Some(tank), false).unwrap();
    super::super::kit_clearcasting_tests::wait_for_gcd(&mut h, healer, 8936);
    assert!(
        own_aura(&h, healer, healer, 114108).is_some(),
        "SotF must come from the real Swiftmend"
    );
    let health = h.app.world().get::<Health>(tank).unwrap();
    let fraction = health.current / health.max - 0.2;
    damage(&mut h, tank, fraction);
    let first = h.inbox.log.len();
    for _ in 0..100 {
        decide(&mut h, healer, &party);
        h.advance(1);
        if h.inbox.log[first..].iter().any(|event| {
            event.spell_id == Some(8936)
                && event.kind == CombatLogKind::Heal
                && event.source == Some(healer.to_bits())
        }) {
            break;
        }
    }
    assert!(
        h.inbox.log[first..]
            .iter()
            .any(|event| event.spell_id == Some(8936)
                && event.kind == CombatLogKind::Heal
                && event.target == Some(tank.to_bits())
                && event.source == Some(healer.to_bits())
                && event.overflow < event.amount),
        "AI must spend the earned native Soul buff on a useful supported Regrowth, not a topped patient or unsupported Wild Growth bonus"
    );
    assert!(own_aura(&h, healer, healer, 114108).is_none());
}

#[test]
fn resto_policy_native_abundance_fifth_primary_hot_enables_discount() {
    let (mut h, healer, party) = super::super::kit_abundance_tests::native_party(69);
    let threshold = own_aura(&h, healer, healer, 207383)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount;
    assert_eq!(threshold, party.len() as f32);
    for &target in &party[..4] {
        super::super::kit_abundance_tests::cast_real_rejuvenation(&mut h, healer, target);
    }
    assert!(
        own_aura(&h, healer, healer, 207640).is_none(),
        "four native primary HoTs must not fake old stacked Abundance"
    );
    let regular = super::super::kit_clearcasting_tests::mana_cost(&h, healer, 8936);
    damage(&mut h, party[4], 0.1);
    decide(&mut h, healer, &party);
    let buff = own_aura(&h, healer, healer, 207640)
        .expect("useful fifth primary Rejuvenation should activate the native threshold buff");
    assert_eq!(buff.stacks, 1);
    assert_eq!(
        party
            .iter()
            .filter(|target| own_aura(&h, healer, **target, 774).is_some())
            .count(),
        5
    );
    assert!(
        super::super::kit_clearcasting_tests::mana_cost(&h, healer, 8936) < regular,
        "discount must come from native207640, not a policy mana grant"
    );
}

#[test]
fn resto_policy_learned_germination_adds_second_native_hot_to_pressured_tank() {
    let (mut h, healer, party) = super::super::kit_abundance_tests::native_party(33);
    assert!(own_aura(&h, healer, healer, 155675).is_some());
    let tank = party[2];
    super::super::kit_abundance_tests::cast_real_rejuvenation(&mut h, healer, tank);
    assert!(own_aura(&h, healer, tank, 774).is_some());
    assert!(own_aura(&h, healer, tank, 155777).is_none());
    damage(&mut h, tank, 0.45);
    let until = aura_now(h.app.world()) + 3.5;
    while aura_now(h.app.world()) < until {
        h.advance(1);
    }
    let patient = super::super::healer_triage::read_patient(h.app.world(), tank).unwrap();
    assert!(!patient.emergency && patient.score > 0.0 && patient.missing_health > 0.0);
    let rows = h
        .app
        .world()
        .resource::<FollowerCatalog>()
        .kits
        .iter()
        .filter(|entry| entry.available(105, 33) && entry.spell_id == 774)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    let follower = h.app.world().get::<Follower>(healer).unwrap().clone();
    super::super::healer_triage::update(h.app.world_mut(), healer, &follower, &party, &rows);
    h.advance(1);
    assert!(own_aura(&h, healer, tank, 774).is_some());
    assert!(
        own_aura(&h, healer, tank, 155777).is_some(),
        "actual learned Germination must add its second native HoT despite the generic primary missing-aura condition"
    );
}

#[test]
fn resto_policy_marks_every_living_native_party_member_before_safe_filler() {
    let (mut h, healer, party) = super::super::kit_abundance_tests::native_party(32);
    for _ in 0..200 {
        decide(&mut h, healer, &party);
        h.advance(1);
        if party
            .iter()
            .all(|target| crate::auras::has_aura(h.app.world(), *target, 1126))
        {
            return;
        }
    }
    let missing: Vec<_> = party
        .iter()
        .filter(|target| !crate::auras::has_aura(h.app.world(), **target, 1126))
        .collect();
    panic!(
        "native Mark of the Wild maintenance must cover the real party before damage filler; missing {missing:?}"
    );
}

#[test]
fn resto_policy_stops_admitted_damage_when_recent_pressure_is_temporarily_topped() {
    let (mut h, healer) = super::super::kit_tests::level32(209072);
    let target = h.caster;
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 1126, Some(healer), false).unwrap();
    super::super::kit_clearcasting_tests::wait_for_gcd(&mut h, healer, 5176);
    let heal_budget = [774, 8936]
        .into_iter()
        .map(|spell| super::super::kit_clearcasting_tests::mana_cost(&h, healer, spell))
        .fold(f32::INFINITY, f32::min)
        * 0.5;
    let enemy = h.npc;
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 5176, Some(enemy), false).unwrap();
    assert!(
        h.app
            .world()
            .get::<CastState>(healer)
            .is_some_and(|cast| cast.spell_id == 5176)
    );
    h.app.world_mut().get_mut::<Mana>(healer).unwrap().current = heal_budget;
    damage(&mut h, target, 0.2);
    let other = super::super::kit_abundance_tests::add_native_friend(&mut h, 209072);
    crate::spell_cast::creature_cast(h.app.world_mut(), other, 18562, Some(target), false).unwrap();
    let health = h.app.world().get::<Health>(target).unwrap();
    assert_eq!(
        health.current, health.max,
        "other native healer must top the pressured recipient"
    );
    let risk = super::super::healer_triage::read_patient(h.app.world(), target).unwrap();
    assert!(
        risk.score > 0.0,
        "recent actual damage remains unsafe despite the top-up"
    );
    decide(&mut h, healer, &[target, healer]);
    assert!(
        h.app
            .world()
            .get::<CastState>(healer)
            .is_none_or(|cast| cast.spell_id != 5176),
        "no affordable heal is not permission to continue an unsafe admitted damage cast"
    );
}

fn native_efflo_party() -> (Harness, Entity, [Entity; 5]) {
    let (mut h, healer, party) = super::super::kit_abundance_tests::native_party(40);
    let native = crate::spell_cast::test_harness::load_native_creature_world(&[47649]);
    assert!(
        native.templates.contains_key(&47649),
        "published native Efflo template required"
    );
    h.app
        .insert_resource(crate::test_data::world_data().clone());
    let center = shared::components::Position {
        x: 12.0,
        y: 0.0,
        z: 0.0,
    };
    for &target in &party[2..] {
        h.app.world_mut().entity_mut(target).insert(center);
        damage(&mut h, target, 0.35);
    }
    (h, healer, party)
}

#[test]
fn resto_policy_native_efflo_summon_pulses_useful_owner_healing_at_placement() {
    let (mut h, healer, party) = native_efflo_party();
    let center = *h
        .app
        .world()
        .get::<shared::components::Position>(party[2])
        .unwrap();
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 145205, Some(party[2]), false)
        .expect("ordinary learned native Efflo cast");
    h.advance(1);
    let field = {
        let world = h.app.world_mut();
        world
            .query::<(Entity, &crate::spell_cast::summon::SummonedCreature)>()
            .iter(world)
            .find(|(_, summon)| summon.owner == healer && summon.spell_id == 145205)
            .map(|(unit, _)| unit)
            .expect("native145205 must summon47649")
    };
    let at = h
        .app
        .world()
        .get::<shared::components::Position>(field)
        .unwrap();
    assert_eq!((at.x, at.y, at.z), (center.x, center.y, center.z));
    assert!(
        own_aura(&h, field, field, 81262).is_some(),
        "native summon must initialize its actual81262 periodic dummy aura"
    );
    let period_ms = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(81262)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .aura_period_ms;
    assert!(period_ms > 0);
    let until = aura_now(h.app.world()) + f64::from(period_ms) / 1000.0;
    let first = h.inbox.log.len();
    while aura_now(h.app.world()) < until {
        h.advance(1);
    }
    let useful = h.inbox.log[first..]
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Heal
                && event.spell_id == Some(81269)
                && event.source == Some(healer.to_bits())
                && event.overflow < event.amount
        })
        .collect::<Vec<_>>();
    assert_eq!(
        useful.len(),
        3,
        "first native pulse must heal the three injured allies at its field"
    );
    assert!(useful.iter().all(|event| {
        party[2..]
            .iter()
            .any(|target| event.target == Some(target.to_bits()))
    }));
}

#[test]
fn resto_policy_native_efflo_recast_removes_previous_source_area_trigger() {
    let (mut h, healer, party) = native_efflo_party();
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 145205, Some(party[2]), false)
        .unwrap();
    super::super::kit_clearcasting_tests::wait_for_gcd(&mut h, healer, 145205);
    let previous = {
        let world = h.app.world_mut();
        let fields = world
            .query::<(Entity, &crate::spell_cast::area_trigger::SpellAreaTrigger)>()
            .iter(world)
            .filter(|(_, field)| field.caster == healer && field.spell_id == 145205)
            .map(|(unit, _)| unit)
            .collect::<Vec<_>>();
        assert_eq!(fields.len(), 1);
        fields[0]
    };
    h.app
        .world_mut()
        .get_mut::<shared::components::Position>(party[2])
        .unwrap()
        .x = 18.0;
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 145205, Some(party[2]), false)
        .unwrap();
    h.advance(1);
    let world = h.app.world_mut();
    assert!(
        world.get_entity(previous).is_err(),
        "source145205 effect2 removes the old caster-owned area trigger before creating the new one"
    );
    let fields = world
        .query::<(
            &crate::spell_cast::area_trigger::SpellAreaTrigger,
            &shared::components::Position,
        )>()
        .iter(world)
        .filter(|(field, _)| field.caster == healer && field.spell_id == 145205)
        .map(|(_, at)| at.x)
        .collect::<Vec<_>>();
    assert_eq!(fields, vec![18.0]);
}

#[test]
fn resto_policy_admits_native_efflo_for_stable_cluster_without_direct_heal_effect() {
    let (mut h, healer, party) = native_efflo_party();
    let until = aura_now(h.app.world()) + 3.5;
    while aura_now(h.app.world()) < until {
        h.advance(1);
    }
    for target in &party[2..] {
        let patient = super::super::healer_triage::read_patient(h.app.world(), *target).unwrap();
        assert!(
            !patient.emergency && patient.missing_health > 0.0,
            "native stable damage must exercise ordinary field admission, not an emergency exception"
        );
    }
    let rows = h
        .app
        .world()
        .resource::<FollowerCatalog>()
        .kits
        .iter()
        .filter(|entry| entry.available(105, 40) && entry.spell_id == 145205)
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(rows.len(), 1);
    let follower = h.app.world().get::<Follower>(healer).unwrap().clone();
    super::super::healer_triage::update(h.app.world_mut(), healer, &follower, &party, &rows);
    h.advance(1);
    let world = h.app.world_mut();
    assert!(
        world
            .query::<&crate::spell_cast::summon::SummonedCreature>()
            .iter(world)
            .any(|summon| summon.owner == healer && summon.spell_id == 145205),
        "actual learned145205 must place its native healing field for stable clustered injuries; zero direct-heal metadata is not zero healing"
    );
}
