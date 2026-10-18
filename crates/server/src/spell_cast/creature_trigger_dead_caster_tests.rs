//! Bool-triggered Unit::CastSpell has no triggering aura and permits a dead caster.
use super::test_harness::*;
use shared::spell_data::CastFailReason;

const DEATH_TOUCH: u32 = 5;
const FROSTBOLT: u32 = 116;
const FROSTBOLT_DAMAGE: u32 = 228597;
const SHEILUN_GIFT: u32 = 205406;
const SHEILUN_CLOUD: u32 = 214502;

fn native_fixture() -> Harness {
    let data = world_db().unwrap();
    let spells = [
        DEATH_TOUCH,
        FROSTBOLT,
        FROSTBOLT_DAMAGE,
        SHEILUN_GIFT,
        SHEILUN_CLOUD,
    ]
    .into_iter()
    .map(|id| {
        data.store
            .get(id)
            .expect("native caster-life spells")
            .clone()
    })
    .collect();
    harness(spells, &[], 8).unwrap()
}

fn kill_npc_caster(h: &mut Harness) {
    super::creature::creature_cast(h.app.world_mut(), h.caster, DEATH_TOUCH, Some(h.npc), false)
        .unwrap();
    assert_eq!(h.health(h.npc), 0.0);
    assert!(h.app.world().get_entity(h.npc).is_ok());
}

#[test]
fn native_dead_creature_bool_trigger_accepts_non_aura_origin_frostbolt() {
    let mut h = native_fixture();
    kill_npc_caster(&mut h);
    super::creature::creature_cast(h.app.world_mut(), h.npc, FROSTBOLT, Some(h.caster), true)
        .unwrap();
    assert_eq!(h.health(h.npc), 0.0);
    assert!(h.health(h.caster) > 0.0);
}

#[test]
fn native_dead_creature_bool_trigger_still_checks_sheilun_caster_requirement() {
    let mut h = native_fixture();
    kill_npc_caster(&mut h);
    let (reason, detail) = super::creature::creature_cast(
        h.app.world_mut(),
        h.npc,
        SHEILUN_GIFT,
        Some(h.caster),
        true,
    )
    .expect_err("dead non-aura triggered caster still needs its own cloud")
    .to_contract();
    assert_eq!(reason, CastFailReason::InvalidTarget);
    assert_eq!(detail.as_deref(), Some("SPELL_FAILED_CASTER_AURASTATE"));
}

#[test]
fn native_dead_creature_ordinary_frostbolt_still_refuses_caster_dead() {
    let mut h = native_fixture();
    kill_npc_caster(&mut h);
    assert!(matches!(
        super::creature::creature_cast(h.app.world_mut(), h.npc, FROSTBOLT, Some(h.caster), false,),
        Err(super::CastFailure::CasterDead)
    ));
}

#[test]
fn native_living_creature_bool_trigger_still_accepts_frostbolt() {
    let mut h = native_fixture();
    super::creature::creature_cast(h.app.world_mut(), h.npc, FROSTBOLT, Some(h.caster), true)
        .unwrap();
    assert!(h.health(h.npc) > 0.0);
}
