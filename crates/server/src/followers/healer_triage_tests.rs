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
    crate::spell_cast::try_cast_triggered(h.app.world_mut(), healer, 774, Some(protected)).unwrap();
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
