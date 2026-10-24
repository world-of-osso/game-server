//! Transport-free qualification against the authoritative server library.
#![cfg(test)]

use server::{
    auras, class_progression, combat, combat_refs, conditions, creature_aggro, creature_ai,
    creature_scripts, equipment_effects, faction, followers, hunter_pet, item_proc_data,
    item_stats, item_templates, level_scaling, maps, melee, networking, npc_interaction,
    pet_control, random, scripted_ai, simc_apl_native, spell_cast, spell_info, stats, test_data,
    trait_config, unit_creature_type, world_data, zones,
};

mod dungeon_simulator;
