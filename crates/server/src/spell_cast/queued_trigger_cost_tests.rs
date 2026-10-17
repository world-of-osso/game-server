//! Native470411 checks but retains mana; queued188389 pays its own cost.
use shared::components::Mana;

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
