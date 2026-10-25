//! Transport-free qualification against the authoritative server library.
#![cfg(test)]

extern crate server_combat as server;

use server::{
    auras, class_progression, combat, combat_refs, conditions, creature_aggro, creature_ai,
    creature_scripts, equipment_effects, faction, followers, hunter_pet, item_proc_data,
    item_stats, item_templates, level_scaling, maps, melee, networking, npc_interaction,
    pet_control, random, scripted_ai, simc_apl_native, spell_info, stats, test_data, trait_config,
    unit_creature_type, world_data, zones,
};

mod spell_cast {
    pub use server::spell_cast::*;
    pub use server_spell_scripts::test_support as test_harness;
}

mod add_up_tests;
mod dungeon_simulator;
