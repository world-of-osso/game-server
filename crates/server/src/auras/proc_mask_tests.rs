//! Native combat-event mask contract, sourced TC Spell.cpp2343–2400.
use super::procs::*;
use crate::spell_cast::test_harness::world_db;

#[test]
fn native_hit_proc_masks_follow_ability_periodic_and_weapon_attributes() {
    let db = world_db().unwrap();
    // Actual unchanged catalog rows, no attributes or proc chances overwritten.
    for (spell_id, harmful, expected) in [
        (
            85673,
            false,
            (PROC_DEAL_HELPFUL_SPELL, PROC_TAKE_HELPFUL_SPELL),
        ),
        (
            5857,
            true,
            (PROC_DEAL_HARMFUL_PERIODIC, PROC_TAKE_HARMFUL_PERIODIC),
        ),
        (
            5394,
            false,
            (PROC_DEAL_HELPFUL_ABILITY, PROC_TAKE_HELPFUL_ABILITY),
        ),
        (
            1464,
            true,
            (PROC_DEAL_MELEE_ABILITY, PROC_TAKE_MELEE_ABILITY),
        ),
        (
            19434,
            true,
            (PROC_DEAL_RANGED_ABILITY, PROC_TAKE_RANGED_ABILITY),
        ),
        (
            2061,
            false,
            (PROC_DEAL_HELPFUL_SPELL, PROC_TAKE_HELPFUL_SPELL),
        ),
    ] {
        let spell = db.store.get(spell_id).expect("native fixture spell");
        assert_eq!(
            spell_hit_masks(spell, harmful),
            expected,
            "native spell{spell_id} hit-event mask"
        );
    }
}
