//! Class-follower casting through the DPS parity harness's spec APL drivers.
//!
//! The parity fixtures in `server-simulation` drive these same types; this crate
//! owns their production code so a dungeon follower and its measurement share one
//! implementation.

extern crate server_combat as server;

use server::{
    auras, class_progression, combat, creature_ai, equipment_effects, followers, hunter_pet,
    item_proc_data, item_stats, item_templates, networking, random, scripted_ai, simc_apl_native,
    spell_cast, spell_info, stats,
};

pub mod brewmaster_rotation;
mod follower;
pub mod holy_paladin_rotation;
pub mod literal_rotation;
pub mod profiles;
pub mod registry;
pub mod ret_rotation;

pub use follower::{
    FollowerApl, FollowerAplFailed, FollowerAplPlugin, SpecRotation, drive_followers, party_of,
};
