//! Native zero-mask proc versus pinned TDB family/type scope; no Ret dependency.
use crate::auras::{ActiveAuras, AuraApplication, AuraSource, apply_aura, has_aura};
use crate::spell_cast::test_harness::{Harness, harness};
use shared::components::Health;

const BODY_AND_SOUL: u32 = 64129;
const SPEED: u32 = 65081;
const SHIELD: u32 = 17;
const FLASH_HEAL: u32 = 2061;

fn priest() -> Harness {
    let mut h = harness(Vec::new(), &[BODY_AND_SOUL, SPEED, SHIELD, FLASH_HEAL], 5).unwrap();
    h.app.world_mut().entity_mut(h.caster).insert(Health {
        current: 1.0,
        max: 100.0,
    });
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster: h.caster,
            target: h.caster,
            spell_id: BODY_AND_SOUL,
            effect_mask: u32::MAX,
            source: AuraSource::Cast,
        },
    );
    let parent = h
        .app
        .world()
        .get::<ActiveAuras>(h.caster)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == BODY_AND_SOUL)
        .unwrap();
    assert!(
        parent
            .effects
            .iter()
            .all(|effect| effect.class_mask == [0; 4]),
        "native input really has no DB2 family restriction"
    );
    assert!(!has_aura(h.app.world(), h.caster, SPEED));
    h
}

fn saw_speed_after_cast(h: &mut Harness, spell: u32) -> bool {
    h.cast_and_deliver(spell, Some(h.caster));
    assert!(
        h.inbox.failed.is_empty(),
        "ordinary native cast: {:?}",
        h.inbox.failed
    );
    let mut seen = has_aura(h.app.world(), h.caster, SPEED);
    for _ in 0..200 {
        h.advance(1);
        seen |= has_aura(h.app.world(), h.caster, SPEED);
    }
    seen
}

#[test]
#[ignore = "native TDB scope RED: audited64129 permits Shield no-damage/heal event"]
fn tdb_proc_scope_native_body_and_soul_preserves_power_word_shield() {
    let _seed = crate::random::seed_scope(70448);
    let mut h = priest();
    let seen = saw_speed_after_cast(&mut h, SHIELD);
    assert!(
        has_aura(h.app.world(), h.caster, SHIELD),
        "actual native shield must apply"
    );
    assert!(
        seen,
        "audited family6/mask0=1/type4 permits native Shield17"
    );
}

#[test]
#[ignore = "native TDB scope RED: Flash Heal is outside64129 family/type scope"]
fn tdb_proc_scope_native_body_and_soul_rejects_flash_heal() {
    let _seed = crate::random::seed_scope(70448);
    let mut h = priest();
    let before = h.health(h.caster);
    let seen = saw_speed_after_cast(&mut h, FLASH_HEAL);
    assert!(
        h.health(h.caster) > before,
        "actual native Flash Heal must heal"
    );
    assert!(
        !seen,
        "TDB64129 family mask/type4 cannot proc speed65081 from FlashHeal2061"
    );
}
