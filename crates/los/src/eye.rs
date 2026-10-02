//! Eye points: the Head attachment per creature display and stand state.
//!
//! The bake writes `eye_heights.tsv`: `display_id`, `stand_state`, then the Head
//! attachment's position in the display's model space (WoW axes: X forward, Y left, Z up),
//! in yards, posed in the stand state's animation and scaled by the display's
//! `CreatureDisplayInfo.CreatureModelScale` and `CreatureModelData.ModelScale`. The caller
//! multiplies by the unit's own scale and rotates by its facing.

use std::collections::HashMap;

pub const EYE_TABLE_FILE: &str = "eye_heights.tsv";

/// TrinityCore `UnitStandStateType` and the looping animation (`AnimationData` ID) each
/// poses the model in, as the client picks it (game-engine
/// `src/game/creatures/npc_gear_data.rs` `unit_pose_anim_id`). `SIT_CHAIR` (2) has no
/// established Retail animation there, so it has no row.
pub const STAND_STATE_ANIMS: [(u8, u16); 9] = [
    (0, 0),   // STAND: Stand
    (1, 97),  // SIT: SitGround
    (3, 100), // SLEEP: Sleep
    (4, 102), // SIT_LOW_CHAIR: SitChairLow
    (5, 103), // SIT_MEDIUM_CHAIR: SitChairMed
    (6, 104), // SIT_HIGH_CHAIR: SitChairHigh
    (7, 6),   // DEAD: Dead
    (8, 115), // KNEEL: KneelLoop
    (9, 202), // SUBMERGED: Submerged
];

#[derive(Default)]
pub struct EyeTable {
    eyes: HashMap<(u32, u8), [f32; 3]>,
}

impl EyeTable {
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut eyes = HashMap::new();
        for (number, line) in text.lines().enumerate().skip(1) {
            let fields: Vec<&str> = line.split('\t').collect();
            let parsed = (|| -> Option<((u32, u8), [f32; 3])> {
                let [display, state, x, y, z] = fields.as_slice() else {
                    return None;
                };
                Some((
                    (display.parse().ok()?, state.parse().ok()?),
                    [x.parse().ok()?, y.parse().ok()?, z.parse().ok()?],
                ))
            })();
            let (key, eye) =
                parsed.ok_or_else(|| format!("line {}: bad row {line:?}", number + 1))?;
            eyes.insert(key, eye);
        }
        Ok(Self { eyes })
    }

    pub fn to_tsv(rows: &[((u32, u8), [f32; 3])]) -> String {
        let mut out = String::from("display_id\tstand_state\tx\ty\tz\n");
        for ((display, state), [x, y, z]) in rows {
            out.push_str(&format!("{display}\t{state}\t{x:.3}\t{y:.3}\t{z:.3}\n"));
        }
        out
    }

    /// The Head attachment of `display` in `stand_state`, model space (see the module).
    pub fn eye(&self, display: u32, stand_state: u8) -> Option<[f32; 3]> {
        self.eyes.get(&(display, stand_state)).copied()
    }

    pub fn len(&self) -> usize {
        self.eyes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.eyes.is_empty()
    }
}
