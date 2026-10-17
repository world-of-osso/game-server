//! TC boss_leymor.cpp npc_ley_line_sprouts::JustDied: one owner stack per actual death.
use super::*;

const ARCANE_POWER: u32 = 374736;
const SPROUT: u32 = 190509;

#[test]
#[ignore = "RED: native sprout death does not decrement summoner Arcane Power"]
fn azure_leymor_sprout_death_removes_one_owner_stack_once_until_final_aura_removal() {
    let (mut h, leymor) = super::azure_sprouts::prepare_arcane_power_prerequisites();
    assert_eq!(
        crate::auras::aura_count(h.app.world(), leymor, ARCANE_POWER),
        14
    );
    let original_caster = h
        .app
        .world()
        .get::<crate::auras::ActiveAuras>(leymor)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == ARCANE_POWER)
        .unwrap()
        .caster;
    let mut children = h.creatures_of(SPROUT);
    assert_eq!(children.len(), 14);
    assert!(children.contains(&original_caster));
    // Kill the actual original aura caster first, not a synthetic replacement.
    children.sort_by_key(|child| *child != original_caster);
    let player = h.player();
    for (index, child) in children.into_iter().enumerate() {
        crate::combat::kill_unit(h.app.world_mut(), player, child);
        h.advance(0.1);
        assert!(!crate::spell_cast::targets::is_alive(h.app.world(), child));
        let remaining = 13 - index as u32;
        assert_eq!(
            crate::auras::aura_count(h.app.world(), leymor, ARCANE_POWER),
            remaining,
            "one decrement for native sprout death {index}"
        );
        let view = h
            .app
            .world()
            .get::<shared::components::UnitAuras>(leymor)
            .unwrap();
        let powers: Vec<_> = view
            .auras
            .iter()
            .filter(|aura| aura.spell_id == ARCANE_POWER)
            .collect();
        if remaining == 0 {
            assert!(powers.is_empty(), "last stack removes replicated aura");
        } else {
            assert_eq!(powers.len(), 1);
            assert_eq!(u32::from(powers[0].stacks), remaining);
            assert_eq!(powers[0].caster, Some(original_caster.to_bits()));
        }
        crate::combat::kill_unit(h.app.world_mut(), player, child);
        h.advance(0.1);
        assert_eq!(
            crate::auras::aura_count(h.app.world(), leymor, ARCANE_POWER),
            remaining,
            "corpse is not a second death"
        );
    }
}
