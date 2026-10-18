//! Native470411 checks but retains mana; queued188389 pays its own cost.
use shared::components::{Mana, PowerType};

use super::test_harness::*;
use crate::auras::has_aura;

const FLAME_SHOCK_PARENT: u32 = 470411;
const FLAME_SHOCK_CHILD: u32 = 188389;
const NATURAL_FIXTURE_MANA: f32 = 10_000.0;
const NATIVE_CHILD_MANA_COST: f32 = 30.0;

fn native_fixture() -> Harness {
    let data = world_db().unwrap();
    let spells = [FLAME_SHOCK_PARENT, FLAME_SHOCK_CHILD]
        .into_iter()
        .map(|id| {
            data.store
                .get(id)
                .expect("native Flame Shock chain")
                .clone()
        })
        .collect();
    let h = harness(spells, &[], 7).unwrap();
    assert_eq!(mana(&h), NATURAL_FIXTURE_MANA);
    h
}

fn mana(h: &Harness) -> f32 {
    h.app.world().get::<Mana>(h.caster).unwrap().current
}

fn cast_parent(h: &mut Harness) {
    super::creature::creature_cast(
        h.app.world_mut(),
        h.caster,
        FLAME_SHOCK_PARENT,
        Some(h.npc),
        false,
    )
    .unwrap();
}

#[test]
fn native_trigger64_no_consume_parent_does_not_spend_before_child_dispatch() {
    let mut h = native_fixture();
    cast_parent(&mut h);
    assert_eq!(mana(&h), NATURAL_FIXTURE_MANA);
    assert!(!has_aura(h.app.world(), h.npc, FLAME_SHOCK_CHILD));
}

#[test]
fn native_trigger64_child_pays_native_mana_after_parent_without_double_charge() {
    let mut h = native_fixture();
    cast_parent(&mut h);
    h.advance(1);
    assert!(has_aura(h.app.world(), h.npc, FLAME_SHOCK_CHILD));
    assert_eq!(mana(&h), NATURAL_FIXTURE_MANA - NATIVE_CHILD_MANA_COST);
}

#[test]
fn native_trigger64_missing_target_does_not_spend_child_mana() {
    let mut h = native_fixture();
    cast_parent(&mut h);
    h.app.world_mut().despawn(h.npc);
    h.advance(1);
    assert_eq!(mana(&h), NATURAL_FIXTURE_MANA);
    assert!(h.app.world().get_entity(h.npc).is_err());
}

#[test]
fn native_trigger64_child_refuses_unaffordable_dispatch_after_native_enemy_mana_drain() {
    const SOUL_REND: u32 = 3405;
    const SOUL_REND_CASTS: usize = 1000;
    const SKUL: u32 = 10393;
    let mut h = harness(
        Vec::new(),
        &[FLAME_SHOCK_PARENT, FLAME_SHOCK_CHILD, SOUL_REND],
        7,
    )
    .unwrap();
    let db_path = &world_db().unwrap().powers_db;
    let source =
        rusqlite::Connection::open_with_flags(db_path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)
            .unwrap();
    h.app
        .insert_resource(crate::world_data::load_creature_spell_stats(&source));
    h.app
        .insert_resource(crate::world_data::load_world_data(db_path));
    // Normal spawning supplies Skul's native UnitLevel, not an invented fixture level.
    let caster = h.caster;
    let drainer =
        crate::zones::summon_creature(h.app.world_mut(), caster, SKUL, [3.0, 0.0, 0.0, 0.0])
            .unwrap();
    let control = spawn_player(&mut h.app, 7, 0.0);
    h.advance(1); // Ordinary initialization of the same-class regen control.
    assert_eq!(mana(&h), NATURAL_FIXTURE_MANA);
    assert_eq!(
        h.app.world().get::<Mana>(control).unwrap().current,
        NATURAL_FIXTURE_MANA
    );
    let caster_health = h.health(h.caster);
    let target_health = h.health(h.npc);
    cast_parent(&mut h);
    assert_eq!(mana(&h), NATURAL_FIXTURE_MANA);
    assert!(!has_aura(h.app.world(), h.npc, FLAME_SHOCK_CHILD));
    // Actual instant, level-scaled native enemy casts between enqueue and dispatch;
    // no frame/regen suppression, raw pool edit or changed imported effect.
    for target in [h.caster, control] {
        for _ in 0..SOUL_REND_CASTS {
            super::creature::creature_cast(
                h.app.world_mut(),
                drainer,
                SOUL_REND,
                Some(target),
                true,
            )
            .unwrap();
        }
    }
    assert_eq!(mana(&h), 0.0);
    assert_eq!(h.app.world().get::<Mana>(control).unwrap().current, 0.0);
    assert_eq!(h.health(h.caster), caster_health);
    assert_eq!(h.health(h.npc), target_health);
    let child = world_db().unwrap().store.get(FLAME_SHOCK_CHILD).unwrap();
    assert!(matches!(
        super::spell_costs(h.app.world(), h.caster, child),
        Err(super::CastFailure::NotEnoughPower(PowerType::Mana))
    ));
    h.advance(1);
    let remaining = mana(&h);
    let control_remaining = h.app.world().get::<Mana>(control).unwrap().current;
    assert_eq!(
        remaining, control_remaining,
        "refused child spends no power beyond normal regen"
    );
    assert!((0.0..NATIVE_CHILD_MANA_COST).contains(&remaining));
    assert!(!has_aura(h.app.world(), h.npc, FLAME_SHOCK_CHILD));
    assert_eq!(h.health(h.caster), caster_health);
    assert_eq!(h.health(h.npc), target_health);
    assert!(
        h.inbox
            .failed
            .iter()
            .all(|failure| failure.spell_id != FLAME_SHOCK_CHILD)
    );
    // The same living recipient remains eligible through the distinct FULL_MASK
    // helper, which does not pay power; low mana did not manufacture a target failure.
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        FLAME_SHOCK_CHILD,
        Some(h.npc),
        0,
    ));
    assert!(has_aura(h.app.world(), h.npc, FLAME_SHOCK_CHILD));
    assert_eq!(mana(&h), remaining);
}

#[test]
fn native_full_trigger_helper_keeps_its_distinct_power_cost_exemption() {
    let mut h = native_fixture();
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        FLAME_SHOCK_CHILD,
        Some(h.npc),
        0,
    ));
    assert!(has_aura(h.app.world(), h.npc, FLAME_SHOCK_CHILD));
    assert_eq!(mana(&h), NATURAL_FIXTURE_MANA);
}
