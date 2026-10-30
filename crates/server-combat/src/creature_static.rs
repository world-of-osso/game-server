//! TC 15e0ed76 Creature.cpp:575–576, 1300–1309, 2923–2942, 2972–3000;
//! ObjectMgr.cpp:1615–1650: source difficulty static flags and initial effects.
use bevy::prelude::*;

pub const UNKILLABLE: u32 = 0x0000_0008;
pub const SESSILE: u32 = 0x0000_0100;
pub const NO_MELEE_FLEE: u32 = 0x0010_0000;
pub const IGNORE_COMBAT: u32 = 0x0200_0000;
pub const CAN_SWIM: u32 = 0x1000_0000;
pub const FLOATING: u32 = 0x2000_0000;
pub const CANT_SWIM: u32 = 0x0000_0100;
pub const ALLOW_INTERACTION_IN_COMBAT: u32 = 0x0004_0000;
pub const CANNOT_TURN: u32 = 0x0200_0000;
pub const NO_MELEE_APPROACH: u32 = 0x0000_0200;
pub const SWIM_IF_TARGET_SWIMS: u32 = 0x0100_0000;
pub const INTERACT_WHILE_HOSTILE: u32 = 0x0000_0100;

#[derive(Component, Debug, Clone, Copy)]
pub struct CreatureStaticFlags(pub [u32; 8]);

impl CreatureStaticFlags {
    pub fn sessile(self) -> bool {
        self.0[0] & SESSILE != 0
    }
    pub fn can_melee(self) -> bool {
        self.0[0] & NO_MELEE_FLEE == 0 && self.0[3] & NO_MELEE_APPROACH == 0
    }
}

/// TC ObjectMgr::ChooseCreatureFlags applies these static movement bits to UnitFlags.
pub fn unit_flags(authored: u32, flags: [u32; 8]) -> u32 {
    let mut value = authored;
    if flags[0] & CAN_SWIM != 0 {
        value |= 0x0000_8000;
    }
    if flags[2] & CANT_SWIM != 0 {
        value |= 0x0000_4000;
    }
    value
}

/// TC ObjectMgr::ChooseCreatureFlags applies static capability bits to UnitFlags2.
pub fn unit_flags2(authored: u32, flags: [u32; 8]) -> u32 {
    let mut value = authored;
    if flags[2] & CANNOT_TURN != 0 {
        value |= 0x0000_8000;
    }
    if flags[3] & SWIM_IF_TARGET_SWIMS != 0 {
        value |= 0x0100_0000;
    }
    if flags[4] & INTERACT_WHILE_HOSTILE != 0 {
        value |= 0x0000_4000;
    }
    value
}
