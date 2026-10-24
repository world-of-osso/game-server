//! Native melee behavior qualification against the authoritative server library.
#![cfg(test)]

use server::{auras, combat_refs, maps, random, spell_cast, stats, test_data, world_data, zones};

mod combat;
