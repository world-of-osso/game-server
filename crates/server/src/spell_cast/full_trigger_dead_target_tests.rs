//! Full-mask admission keeps native dead-target eligibility separate from relation bypass.
use shared::components::Mana;

use super::test_harness::*;
use crate::auras::has_aura;

const DEATH_TOUCH: u32 = 5;
const FLAME_SHOCK: u32 = 188389;
const FREEZE: u32 = 9454;
const NATURAL_FIXTURE_MANA: f32 = 10_000.0;

fn native_fixture() -> Harness {
    let data = world_db().unwrap();
    let spells = [DEATH_TOUCH, FLAME_SHOCK, FREEZE]
        .into_iter()
        .map(|id| {
            data.store
                .get(id)
                .expect("native death/recipient spells")
                .clone()
        })
        .collect();
    harness(spells, &[], 7).unwrap()
}

fn kill_target(h: &mut Harness) {
    super::creature::creature_cast(h.app.world_mut(), h.caster, DEATH_TOUCH, Some(h.npc), false)
        .unwrap();
    assert_eq!(h.health(h.npc), 0.0);
    assert!(h.app.world().get_entity(h.npc).is_ok());
}

#[test]
fn native_full_trigger_refuses_present_dead_flame_shock_recipient() {
    let mut h = native_fixture();
    kill_target(&mut h);
    assert!(!super::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        FLAME_SHOCK,
        Some(h.npc),
        1,
    ));
}

#[test]
fn native_full_trigger_does_not_apply_flame_shock_to_present_corpse() {
    let mut h = native_fixture();
    kill_target(&mut h);
    super::cast_triggered(h.app.world_mut(), h.caster, FLAME_SHOCK, Some(h.npc), 1);
    assert!(!has_aura(h.app.world(), h.npc, FLAME_SHOCK));
}

#[test]
fn native_full_trigger_living_recipient_keeps_cost_exemption() {
    let mut h = native_fixture();
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        FLAME_SHOCK,
        Some(h.npc),
        1,
    ));
    assert!(has_aura(h.app.world(), h.npc, FLAME_SHOCK));
    assert_eq!(
        h.app.world().get::<Mana>(h.caster).unwrap().current,
        NATURAL_FIXTURE_MANA,
    );
}

#[test]
fn native_full_trigger_allow_dead_target_freeze_remains_admissible() {
    let mut h = native_fixture();
    kill_target(&mut h);
    assert!(super::try_cast_triggered(
        h.app.world_mut(),
        h.caster,
        FREEZE,
        Some(h.npc),
        1,
    ));
    assert!(has_aura(h.app.world(), h.npc, FREEZE));
    assert_eq!(h.health(h.npc), 0.0);
}
