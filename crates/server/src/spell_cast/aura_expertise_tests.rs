use super::*;
use crate::auras::{AuraApplication, AuraSource, apply_aura};
use crate::spell_cast::test_harness::{harness_with, world_db};

#[test]
#[ignore = "aura-stat-mods RED: native player expertise240"]
fn aura_stat_native_240_reduces_actual_player_weapon_spell_dodge_chance() {
    let mut h = harness_with(Vec::new(), &[105805], 1, false).unwrap();
    h.app
        .world_mut()
        .entity_mut(h.caster)
        .insert(crate::class_progression::KnownSpells::fixture(Vec::new()));
    h.advance(3);
    let caster = h.caster;
    crate::auras::remove_auras(
        h.app.world_mut(),
        caster,
        crate::auras::RemoveReason::Cancelled,
        |_| true,
    );
    h.advance(2);
    let spell = world_db().unwrap().store.get(12294).unwrap();
    let before = weapon_spell_dodge_chance(h.app.world(), h.caster, spell, h.npc, DEFENSE_MELEE);
    assert!(before >= 0.75, "native fixture dodge {before}");
    let caster = h.caster;
    apply_aura(
        h.app.world_mut(),
        &AuraApplication {
            caster,
            target: caster,
            spell_id: 105805,
            effect_mask: 1 << 3,
            source: AuraSource::Cast,
        },
    );
    h.advance(2);
    assert_eq!(
        weapon_spell_dodge_chance(h.app.world(), caster, spell, h.npc, DEFENSE_MELEE),
        before - 0.75
    );
}
