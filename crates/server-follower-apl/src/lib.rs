//! Class-follower casting through the DPS parity harness's spec APL drivers.
//!
//! The parity fixtures in `server-simulation` drive these same types; this crate
//! owns their production code so a dungeon follower and its measurement share one
//! implementation.

extern crate server_combat as server;

use server::{
    auras, class_progression, combat, combat_refs, creature_ai, equipment_effects, followers,
    hunter_pet, item_proc_data, item_stats, item_templates, networking, pet_control, random,
    scripted_ai, simc_apl_native, spell_cast, spell_info, stats,
};

mod follower;
pub mod hunter_facts;
pub mod literal_rotation;
pub mod profiles;
pub mod registry;

pub mod affliction_rotation;
pub mod arcane_rotation;
pub mod arms_rotation;
pub mod assassination_rotation;
pub mod balance_rotation;
pub mod beast_mastery_rotation;
pub mod blood_tank_rotation;
pub mod brewmaster_rotation;
pub mod demonology_rotation;
pub mod destruction_rotation;
pub mod devastation_rotation;
pub mod devourer_rotation;
pub mod discipline_priest_rotation;
pub mod elemental_rotation;
pub mod enhancement_rotation;
pub mod feral_rotation;
pub mod fire_rotation;
pub mod frost_mage_rotation;
pub mod frost_rotation;
pub mod fury_rotation;
pub mod guardian_tank_rotation;
pub mod havoc_rotation;
pub mod holy_paladin_rotation;
pub mod holy_priest_rotation;
pub mod marksmanship_rotation;
pub mod mistweaver_rotation;
pub mod outlaw_rotation;
pub mod preservation_evoker_rotation;
pub mod protection_paladin_rotation;
pub mod protection_warrior_rotation;
pub mod restoration_druid_rotation;
pub mod restoration_shaman_rotation;
pub mod ret_rotation;
pub mod shadow_rotation;
pub mod subtlety_rotation;
pub mod survival_rotation;
pub mod unholy_rotation;
pub mod vengeance_tank_rotation;
pub mod windwalker_rotation;

pub use follower::{
    FollowerApl, FollowerAplFailed, FollowerAplPlugin, SpecRotation, drive_followers, party_of,
    party_tank,
};
