//! Ordinary caster-life admission honors native Passive and AllowCastWhileDead.
use super::test_harness::*;
use crate::auras::has_aura;

const DEATH_TOUCH: u32 = 5;
const FROSTBOLT: u32 = 116;
const FROSTBOLT_DAMAGE: u32 = 228597;
const RESURRECTION_SICKNESS: u32 = 15007;
const DIVINE_VISION: u32 = 199324;

fn native_fixture() -> Harness {
    let data = world_db().unwrap();
    let spells = [
        DEATH_TOUCH,
        FROSTBOLT,
        FROSTBOLT_DAMAGE,
        RESURRECTION_SICKNESS,
        DIVINE_VISION,
    ]
    .into_iter()
    .map(|id| {
        data.store
            .get(id)
            .expect("native ordinary-life spells")
            .clone()
    })
    .collect();
    harness(spells, &[], 2).unwrap()
}

fn kill_npc_caster(h: &mut Harness) {
    super::creature::creature_cast(h.app.world_mut(), h.caster, DEATH_TOUCH, Some(h.npc), false)
        .unwrap();
    assert_eq!(h.health(h.npc), 0.0);
    assert!(h.app.world().get_entity(h.npc).is_ok());
}

fn assert_dead_caster_has_aura(h: &Harness, spell: u32) {
    assert_eq!(h.health(h.npc), 0.0);
    assert!(has_aura(h.app.world(), h.npc, spell));
}

#[test]
fn native_ordinary_allow_cast_while_dead_applies_actual_sickness() {
    let mut h = native_fixture();
    kill_npc_caster(&mut h);
    super::creature::creature_cast(
        h.app.world_mut(),
        h.npc,
        RESURRECTION_SICKNESS,
        Some(h.npc),
        false,
    )
    .unwrap();
    assert_dead_caster_has_aura(&h, RESURRECTION_SICKNESS);
}

#[test]
fn native_ordinary_passive_without_cast_while_dead_flag_applies_divine_vision() {
    let mut h = native_fixture();
    kill_npc_caster(&mut h);
    super::creature::creature_cast(h.app.world_mut(), h.npc, DIVINE_VISION, Some(h.npc), false)
        .unwrap();
    assert_dead_caster_has_aura(&h, DIVINE_VISION);
}

#[test]
fn native_ordinary_execution_recheck_honors_cast_while_dead_flag() {
    let mut h = native_fixture();
    kill_npc_caster(&mut h);
    let spell = world_db()
        .unwrap()
        .store
        .get(RESURRECTION_SICKNESS)
        .unwrap()
        .clone();
    super::execute_cast(h.app.world_mut(), h.npc, &spell, Some(h.npc)).unwrap();
    assert_dead_caster_has_aura(&h, RESURRECTION_SICKNESS);
}

#[test]
fn native_ordinary_unexempt_dead_caster_still_refuses_frostbolt() {
    let mut h = native_fixture();
    kill_npc_caster(&mut h);
    assert!(matches!(
        super::creature::creature_cast(h.app.world_mut(), h.npc, FROSTBOLT, Some(h.caster), false,),
        Err(super::CastFailure::CasterDead)
    ));
}

#[test]
fn native_ordinary_living_caster_still_applies_sickness() {
    let mut h = native_fixture();
    super::creature::creature_cast(
        h.app.world_mut(),
        h.npc,
        RESURRECTION_SICKNESS,
        Some(h.npc),
        false,
    )
    .unwrap();
    assert!(has_aura(h.app.world(), h.npc, RESURRECTION_SICKNESS));
    assert!(h.health(h.npc) > 0.0);
}
