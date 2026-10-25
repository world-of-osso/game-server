//! TC spell_rog_stealth (spell_rogue.cpp, 1784): Stealth's own DB2 rows carry
//! no MOD_STEALTH; its effect1 dummy casts the 158185 stealth and 158188
//! shapeshift children, which leave with the parent.
use super::test_harness::*;
use crate::auras::{ActiveAuras, RemoveReason, has_aura, remove_auras};

const STEALTH: u32 = 1784;
const STEALTH_AURA: u32 = 158_185;
const STEALTH_SHAPESHIFT: u32 = 158_188;
const SANCTUARY: u32 = 98_877;

fn stealth_amount(h: &Harness) -> Option<f32> {
    h.app
        .world()
        .get::<ActiveAuras>(h.caster)?
        .iter()
        .filter(|aura| aura.spell_id == STEALTH_AURA)
        .flat_map(|aura| &aura.effects)
        .find(|effect| effect.aura_type == crate::auras::handlers::MOD_STEALTH)
        .map(|effect| effect.amount)
}

#[test]
fn arena_rogue_stealth_applies_source_children_and_removes_them_with_parent() {
    let mut h = harness(
        Vec::new(),
        &[STEALTH, STEALTH_AURA, STEALTH_SHAPESHIFT, SANCTUARY],
        4,
    )
    .expect("real world.db");
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(shared::components::Level(20));
    h.app
        .world_mut()
        .get_mut::<shared::components::CombatStatus>(h.caster)
        .unwrap()
        .0 = false;
    h.cast_and_deliver(STEALTH, Some(h.caster));
    h.advance(1);
    assert!(h.inbox.failed.is_empty(), "{:?}", h.inbox.failed);
    assert!(has_aura(h.app.world(), h.caster, STEALTH));
    assert!(has_aura(h.app.world(), h.caster, STEALTH_SHAPESHIFT));
    // 158185 s0: 20 + (20 - BaseLevel 1) * RealPointsPerLevel 5 type0 stealth.
    assert_eq!(stealth_amount(&h), Some(115.0));
    remove_auras(
        h.app.world_mut(),
        h.caster,
        RemoveReason::Cancelled,
        |aura| aura.spell_id == STEALTH,
    );
    assert_eq!(stealth_amount(&h), None);
    assert!(!has_aura(h.app.world(), h.caster, STEALTH_SHAPESHIFT));
}
