//! Native spell fixture qualification against the authoritative server.
#![cfg(test)]

use server::{
    auras, class_progression, combat, level_scaling, maps, melee, random, spell_info, stats,
    test_data,
};

mod spell_cast;
