//! Existing public-behavior melee cases against canonical server resources.
pub use server::combat::*;

#[path = "combat_facing_tests.rs"]
mod facing_tests;
#[path = "combat_npc_cadence_tests.rs"]
mod npc_cadence_tests;
#[path = "combat_npc_expertise_tests.rs"]
mod npc_expertise_tests;
#[path = "combat_npc_flags_tests.rs"]
mod npc_flags_tests;
#[path = "combat_npc_no_crit_tests.rs"]
mod npc_no_crit_tests;
#[path = "combat_npc_victim_dodge_tests.rs"]
mod npc_victim_dodge_tests;
#[path = "combat_npc_victim_parry_tests.rs"]
mod npc_victim_parry_tests;
#[path = "combat_npc_weapon_crit_tests.rs"]
mod npc_weapon_crit_tests;
#[path = "combat_pet_engagement_tests.rs"]
mod pet_engagement_tests;
#[path = "combat_victim_caster_crit_tests.rs"]
mod victim_caster_crit_tests;
#[path = "combat_victim_general_crit_tests.rs"]
mod victim_general_crit_tests;
#[path = "combat_victim_summoner_crit_tests.rs"]
mod victim_summoner_crit_tests;
