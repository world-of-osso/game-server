//! Behavioral policy regressions against the actual 70448 follower catalogs/executor.
use super::*;
use crate::spell_cast::test_harness::Harness;
use shared::components::Health;

fn fixture() -> (Harness, Entity) {
    super::super::kit_tests::level32(209072)
}

fn entry(h: &Harness, spell: u32) -> Entry {
    h.app
        .world()
        .resource::<FollowerCatalog>()
        .kits
        .iter()
        .find(|entry| entry.spec_id == 105 && entry.spell_id == spell)
        .unwrap()
        .clone()
}

fn patient(h: &mut Harness, health: f32, tank: bool) -> Entity {
    let map = *h.app.world().get::<InMap>(h.caster).unwrap();
    let mut entity = h.app.world_mut().spawn((
        map,
        Health {
            current: health,
            max: 1000.0,
        },
    ));
    if tank {
        entity.insert(crate::class_progression::ActiveSpec(66));
    }
    entity.id()
}

#[test]
fn healer_triage_tank_risk_outweighs_small_health_fraction_difference() {
    let (mut h, healer) = fixture();
    let dps = patient(&mut h, 600.0, false);
    let tank = patient(&mut h, 650.0, true);
    assert_eq!(
        choose_patient(h.app.world(), healer, &[dps, tank], &entry(&h, 8936)),
        Some(tank)
    );
}

#[test]
fn healer_triage_recent_incoming_damage_beats_idle_missing_health() {
    let (mut h, healer) = fixture();
    let idle = patient(&mut h, 500.0, false);
    let threatened = patient(&mut h, 700.0, false);
    let event = shared::protocol::CombatLogEvent {
        kind: shared::protocol::CombatLogKind::Damage,
        source: Some(h.npc.to_bits()),
        target: Some(threatened.to_bits()),
        amount: 900,
        overflow: 0,
        spell_id: None,
        school_mask: 1,
        absorbed: 0,
        resisted: 0,
        blocked: 0,
        crit: false,
        glancing: false,
        periodic: false,
        extra_spell_id: None,
        timestamp_unix_ms: 0,
    };
    crate::followers::pull_history::record_event(h.app.world_mut(), &event);
    assert_eq!(
        choose_patient(h.app.world(), healer, &[idle, threatened], &entry(&h, 8936)),
        Some(threatened)
    );
}

#[test]
fn healer_triage_remaining_native_hot_avoids_duplicate_healing() {
    let (mut h, healer) = fixture();
    let protected = h.caster;
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        healer,
        774,
        Some(protected),
        1
    ));
    let tick = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(protected)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == 774)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.aura_type == 8)
        .unwrap()
        .amount;
    assert!(tick > 0.0);
    let other = patient(&mut h, 700.0, false);
    h.app.world_mut().entity_mut(protected).insert(Health {
        current: tick,
        max: tick * 2.0,
    });
    h.app.world_mut().entity_mut(other).insert(Health {
        current: tick * 1.4,
        max: tick * 2.0,
    });
    assert_eq!(
        choose_patient(h.app.world(), healer, &[protected, other], &entry(&h, 8936)),
        Some(other)
    );
}

#[test]
fn healer_triage_low_mana_prefers_available_efficient_native_swiftmend() {
    let (mut h, healer) = fixture();
    let target = h.caster;
    assert!(crate::spell_cast::try_cast_triggered(
        h.app.world_mut(),
        healer,
        774,
        Some(target),
        1
    ));
    h.app.world_mut().entity_mut(target).insert(Health {
        current: 70_000.0,
        max: 100_000.0,
    });
    let cost = super::super::kit_clearcasting_tests::mana_cost(&h, healer, 18562);
    let mut mana = h
        .app
        .world_mut()
        .get_mut::<shared::components::Mana>(healer)
        .unwrap();
    assert!(cost * 2.0 < mana.max * 0.3);
    mana.current = cost * 2.0;
    let follower = h.app.world().get::<Follower>(healer).unwrap().clone();
    let first = h.inbox.go.len();
    update(
        h.app.world_mut(),
        healer,
        &follower,
        &[target, healer],
        None,
    );
    h.advance(1);
    assert!(
        h.inbox.go[first..]
            .iter()
            .any(|cast| cast.caster == healer.to_bits() && cast.spell_id == 18562),
        "available efficient Swiftmend must not be gated behind old40% HP ladder"
    );
}

#[test]
fn healer_triage_resto_uses_natures_swiftness_before_emergency_regrowth() {
    let (mut h, healer) = super::super::kit_tests::at_level(209072, 13, |_| {});
    let target = h.caster;
    let mut health = h.app.world_mut().get_mut::<Health>(target).unwrap();
    health.current = health.max * 0.1;
    let follower = h.app.world().get::<Follower>(healer).unwrap().clone();
    update(
        h.app.world_mut(),
        healer,
        &follower,
        &[target, healer],
        None,
    );
    assert!(
        crate::auras::has_aura(h.app.world(), healer, 132158),
        "learned native Nature's Swiftness must precede a life-saving cast-time heal"
    );
    super::super::kit_tests::uses_spell(&mut h, healer, 8936);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(healer.to_bits())
                && event.target == Some(target.to_bits())
                && event.spell_id == Some(8936)
                && event.kind == shared::protocol::CombatLogKind::Heal)
    );
}

#[test]
fn healer_triage_resto_consumes_naturally_earned_clearcasting_on_useful_regrowth() {
    let (mut h, healer) =
        super::super::kit_clearcasting_tests::earn_native_clearcasting_at_level(32);
    let buff = super::super::kit_clearcasting_tests::own_aura(&h, healer, 16870).unwrap();
    assert_eq!(
        super::super::kit_clearcasting_tests::mana_cost(&h, healer, 8936),
        0.0
    );
    let mut health = h.app.world_mut().get_mut::<Health>(healer).unwrap();
    health.current = health.max * 0.7;
    let target = h.caster;
    let follower = h.app.world().get::<Follower>(healer).unwrap().clone();
    update(
        h.app.world_mut(),
        healer,
        &follower,
        &[target, healer],
        None,
    );
    assert!(
        h.app
            .world()
            .get::<CastState>(healer)
            .is_some_and(|cast| cast.spell_id == 8936),
        "free useful Regrowth must precede paid Rejuvenation and damage"
    );
    h.advance(60);
    let remaining = super::super::kit_clearcasting_tests::own_aura(&h, healer, 16870)
        .map_or(0, |aura| aura.stacks.max(1));
    assert_eq!(remaining, buff.stacks.max(1) - 1);
    assert!(
        h.inbox
            .log
            .iter()
            .any(|event| event.source == Some(healer.to_bits())
                && event.target == Some(healer.to_bits())
                && event.spell_id == Some(8936)
                && event.kind == shared::protocol::CombatLogKind::Heal)
    );
}

#[test]
fn healer_triage_topped_patient_cancels_native_direct_cast_before_mana_spend() {
    let (mut h, healer) = fixture();
    let target = h.caster;
    h.app.world_mut().get_mut::<Health>(target).unwrap().current *= 0.5;
    crate::spell_cast::creature_cast(h.app.world_mut(), healer, 8936, Some(target), false).unwrap();
    assert!(h.app.world().get::<CastState>(healer).is_some());
    let max = h.app.world().get::<Health>(target).unwrap().max;
    h.app.world_mut().get_mut::<Health>(target).unwrap().current = max;
    let follower = h.app.world().get::<Follower>(healer).unwrap().clone();
    update(
        h.app.world_mut(),
        healer,
        &follower,
        &[target, healer],
        None,
    );
    assert!(
        h.app
            .world()
            .get::<CastState>(healer)
            .is_none_or(|cast| cast.spell_id != 8936),
        "topped heal must not finish wasting native mana"
    );
}

#[test]
fn healer_triage_scripted_deadline_prepares_full_health_tank() {
    let (mut h, healer) = fixture();
    let tank = h.caster;
    let boss = h.npc;
    let map = InMap {
        map_id: 34,
        instance_id: 1,
    };
    for unit in [tank, boss, healer] {
        h.app.world_mut().entity_mut(unit).insert(map);
    }
    h.app
        .world_mut()
        .entity_mut(tank)
        .insert(crate::class_progression::ActiveSpec(66));
    let max = h.app.world().get::<Health>(tank).unwrap().max;
    h.app.world_mut().get_mut::<Health>(tank).unwrap().current = max;
    let follower = h.app.world().get::<Follower>(healer).unwrap().clone();
    let rows = [entry(&h, 33763)];
    super::super::healer_triage::update(
        h.app.world_mut(),
        healer,
        &follower,
        &[tank, healer],
        &rows,
    );
    assert!(
        !crate::auras::has_aura(h.app.world(), tank, 33763),
        "no invented preparation timer"
    );
    let db = rusqlite::Connection::open_with_flags(
        crate::test_data::world_db_path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    h.app
        .insert_resource(crate::maps::store::load_map_store(&db).unwrap());
    let ai =
        crate::scripted_ai::the_stockade::creature_ai(h.app.world(), boss, "boss_hogger").unwrap();
    crate::scripted_ai::tests::install_fixture_ai(h.app.world_mut(), boss, ai);
    crate::scripted_ai::dispatch(
        h.app.world_mut(),
        boss,
        crate::scripted_ai::Hook::JustEngagedWith(tank),
    );
    let deadlines = |h: &Harness| {
        crate::scripted_ai::read_scheduled_damage(h.app.world(), boss)
            .into_iter()
            .map(|event| (event.spell_id, event.request_in_ms))
            .collect::<Vec<_>>()
    };
    let before = deadlines(&h);
    assert!(
        before
            .iter()
            .any(|(spell, time)| *spell == 86604 && (3000..=4000).contains(time))
    );
    assert!(
        before
            .iter()
            .any(|(spell, time)| *spell == 86620 && (1000..=2000).contains(time))
    );
    super::super::healer_triage::update(
        h.app.world_mut(),
        healer,
        &follower,
        &[tank, healer],
        &rows,
    );
    assert!(
        crate::auras::has_aura(h.app.world(), tank, 33763),
        "actual Hogger1–4s EventMap damage deadlines must permit full-health tank preparation"
    );
    assert_eq!(
        deadlines(&h),
        before,
        "triage observations never advance native timers"
    );
    crate::scripted_ai::dispatch(h.app.world_mut(), boss, crate::scripted_ai::Hook::Reset);
    assert!(
        deadlines(&h).is_empty(),
        "real Reset removes preparation deadlines"
    );
}

#[test]
fn healer_triage_native_radius_rejects_scattered_injury_then_accepts_stack() {
    let (mut h, healer) = super::super::kit_tests::at_level(209072, 40, |_| {});
    let anchor = h.caster;
    let p1 = patient(&mut h, 500.0, false);
    let p2 = patient(&mut h, 500.0, false);
    let position = *h
        .app
        .world()
        .get::<shared::components::Position>(anchor)
        .unwrap();
    h.app.world_mut().get_mut::<Health>(anchor).unwrap().current *= 0.5;
    for (unit, offset) in [(p1, 10000.0), (p2, -10000.0)] {
        h.app
            .world_mut()
            .entity_mut(unit)
            .insert(shared::components::Position {
                x: position.x + offset,
                ..position
            });
    }
    let rows = [entry(&h, 48438)];
    let follower = h.app.world().get::<Follower>(healer).unwrap().clone();
    super::super::healer_triage::update(
        h.app.world_mut(),
        healer,
        &follower,
        &[anchor, p1, p2, healer],
        &rows,
    );
    assert!(
        h.app.world().get::<CastState>(healer).is_none(),
        "global injury count must not admit Wild Growth when no native-radius cluster has three injured patients"
    );
    for unit in [p1, p2] {
        h.app.world_mut().entity_mut(unit).insert(position);
    }
    super::super::healer_triage::update(
        h.app.world_mut(),
        healer,
        &follower,
        &[anchor, p1, p2, healer],
        &rows,
    );
    assert!(
        h.app
            .world()
            .get::<CastState>(healer)
            .is_some_and(|cast| cast.spell_id == 48438),
        "same injuries stacked inside native radius must admit Wild Growth"
    );
}

#[test]
fn healer_triage_recent_damage_does_not_restart_direct_heal_on_topped_patient() {
    use crate::spell_cast::effects::{EffectCtx, apply_heal, deal_damage};
    let (mut h, healer) = fixture();
    let target = h.caster;
    let store = h.app.world().resource::<crate::spell_info::SpellStore>();
    let damage_spell = store.get(133).unwrap().clone();
    let heal_spell = store.get(8936).unwrap().clone();
    let damage = EffectCtx {
        caster: h.npc,
        spell: &damage_spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    let before = h.health(target);
    deal_damage(
        h.app.world_mut(),
        &damage,
        target,
        before * 0.1,
        false,
        false,
    );
    assert!(
        h.health(target) < before,
        "native damage must produce recent triage pressure"
    );
    let heal = EffectCtx {
        caster: healer,
        spell: &heal_spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    apply_heal(h.app.world_mut(), &heal, target, before, false, false);
    assert_eq!(
        h.health(target),
        before,
        "another native heal topped the damaged patient"
    );
    let risk = super::super::healer_triage::read_patient(h.app.world(), target).unwrap();
    assert!(risk.incoming_per_second > 0.0 && risk.score > 0.0);
    let follower = h.app.world().get::<Follower>(healer).unwrap().clone();
    let rows = [entry(&h, 8936)];
    for _ in 0..3 {
        let outcome = super::super::healer_triage::update(
            h.app.world_mut(),
            healer,
            &follower,
            &[target, healer],
            &rows,
        );
        assert!(
            h.app.world().get::<CastState>(healer).is_none(),
            "a topped target must not enter a normal-direct start→cancel→restart loop from recent damage alone"
        );
        assert!(
            !outcome.acted,
            "triage must release execution to utilities rather than starve them"
        );
    }
    h.app.world_mut().get_mut::<Health>(target).unwrap().current *= 0.8;
    super::super::healer_triage::update(
        h.app.world_mut(),
        healer,
        &follower,
        &[target, healer],
        &rows,
    );
    assert!(
        h.app
            .world()
            .get::<CastState>(healer)
            .is_some_and(|cast| cast.spell_id == 8936),
        "fresh real injury still admits the useful direct heal"
    );
}
