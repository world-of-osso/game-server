//! TC a352b1fa Player::UpdateGatherSkill, Spell::EffectSkinning and default
//! worldserver.conf: gain1, orange100/yellow75/green25/grey0; mining/skinning steps0.
#[derive(Clone, Copy, Debug)]
pub(crate) struct GatherProgress {
    pub skill: u32,
    yellow: u16,
    grey: u16,
    pub multiplier: u32,
}

impl GatherProgress {
    pub fn node(skill: u32, required: u16, low: u16, high: u16) -> Self {
        let yellow = if low != 0 { low } else { required + 30 };
        let grey = if high != 0 { high } else { required + 90 };
        Self {
            skill,
            yellow,
            grey,
            multiplier: 1,
        }
    }

    pub fn skinning(skill: u32, level: u8) -> Self {
        let level = u16::from(level);
        let required = match level {
            0..=10 => 1,
            11..=19 => (level - 10) * 10,
            20..=73 => level * 5,
            74..=79 => level * 10 - 365,
            80..=84 => level * 5 + 35,
            85..=87 => level * 15 - 805,
            88..=92 => (level - 62) * 20,
            93..=104 => level * 5 + 175,
            105..=107 => level * 15 - 905,
            108..=112 => (level - 72) * 20,
            113..=122 => (level - 32) * 10,
            _ => 900,
        };
        Self::node(skill, required, 0, 0)
    }

    pub fn chance_permille(self, rank: u16) -> u32 {
        let green = (u32::from(self.yellow) + u32::from(self.grey)) / 2;
        let chance = if rank >= self.grey {
            0
        } else if u32::from(rank) >= green {
            250
        } else if rank >= self.yellow {
            750
        } else {
            1000
        };
        chance * self.multiplier
    }
}
