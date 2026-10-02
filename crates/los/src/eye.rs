//! Eye points per creature display and stand state (docs/specs/line-of-sight.md, Eye
//! point).
//!
//! The bake writes `eye_heights.tsv`: `display_id`, `stand_state`, `source`, then the eye
//! point in the display's model space (WoW axes: X forward, Y left, Z up), in yards, posed
//! at the first frame of the stand state's animation and scaled by the display's
//! `CreatureDisplayInfo.CreatureModelScale` and `CreatureModelData.ModelScale`. `source`
//! is `eyes` (the centroid of the eye geoset's skinned vertices) or `head` (the head
//! bone, for models without eye geometry). A state whose sequence the model lacks holds
//! the Stand eye. The caller multiplies by the unit's own scale and rotates by its facing.
//!
//! `player_displays.tsv` maps each playable race and sex to its character display.

use std::collections::HashMap;
use std::path::Path;

pub const EYE_TABLE_FILE: &str = "eye_heights.tsv";
pub const PLAYER_DISPLAYS_FILE: &str = "player_displays.tsv";

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

/// Where a model's eye point comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EyeSource {
    /// The centroid of the eye geoset (group 17) vertices.
    Eyes,
    /// The head bone (`key_bone_id` 6).
    Head,
}

impl EyeSource {
    fn name(self) -> &'static str {
        match self {
            Self::Eyes => "eyes",
            Self::Head => "head",
        }
    }

    fn parse(text: &str) -> Option<Self> {
        match text {
            "eyes" => Some(Self::Eyes),
            "head" => Some(Self::Head),
            _ => None,
        }
    }
}

/// One `eye_heights.tsv` row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EyeRow {
    pub display: u32,
    pub stand_state: u8,
    pub source: EyeSource,
    /// Model space, WoW axes, yards.
    pub eye: [f32; 3],
}

#[derive(Default)]
pub struct EyeTable {
    eyes: HashMap<(u32, u8), EyeRow>,
    /// Character display by `(race, sex)`.
    players: HashMap<(u8, u8), u32>,
}

impl EyeTable {
    /// `eye_heights.tsv` and `player_displays.tsv` from a bake's output directory.
    pub fn load(dir: &Path) -> Result<Self, String> {
        let read = |name: &str| {
            let path = dir.join(name);
            std::fs::read_to_string(&path).map_err(|err| format!("{}: {err}", path.display()))
        };
        let mut table = Self::parse(&read(EYE_TABLE_FILE)?)?;
        table.players = parse_players(&read(PLAYER_DISPLAYS_FILE)?)?;
        Ok(table)
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let rows = parse_lines(text, |fields| {
            let [display, state, source, x, y, z] = fields else {
                return None;
            };
            Some(EyeRow {
                display: display.parse().ok()?,
                stand_state: state.parse().ok()?,
                source: EyeSource::parse(source)?,
                eye: [x.parse().ok()?, y.parse().ok()?, z.parse().ok()?],
            })
        })?;
        Ok(Self {
            eyes: rows
                .into_iter()
                .map(|row| ((row.display, row.stand_state), row))
                .collect(),
            players: HashMap::new(),
        })
    }

    pub fn to_tsv(rows: &[EyeRow]) -> String {
        let mut out = String::from("display_id\tstand_state\tsource\tx\ty\tz\n");
        for row in rows {
            let [x, y, z] = row.eye;
            out.push_str(&format!(
                "{}\t{}\t{}\t{x:.3}\t{y:.3}\t{z:.3}\n",
                row.display,
                row.stand_state,
                row.source.name()
            ));
        }
        out
    }

    pub fn players_to_tsv(players: &[((u8, u8), u32)]) -> String {
        let mut out = String::from("race\tsex\tdisplay_id\n");
        for ((race, sex), display) in players {
            out.push_str(&format!("{race}\t{sex}\t{display}\n"));
        }
        out
    }

    /// The eye point of `display` in `stand_state`, model space (see the module).
    pub fn eye(&self, display: u32, stand_state: u8) -> Option<[f32; 3]> {
        self.row(display, stand_state).map(|row| row.eye)
    }

    pub fn row(&self, display: u32, stand_state: u8) -> Option<&EyeRow> {
        self.eyes.get(&(display, stand_state))
    }

    /// The character display of a player of `race` and `sex`.
    pub fn player_display(&self, race: u8, sex: u8) -> Option<u32> {
        self.players.get(&(race, sex)).copied()
    }

    pub fn len(&self) -> usize {
        self.eyes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.eyes.is_empty()
    }
}

fn parse_players(text: &str) -> Result<HashMap<(u8, u8), u32>, String> {
    Ok(parse_lines(text, |fields| {
        let [race, sex, display] = fields else {
            return None;
        };
        Some((
            (race.parse().ok()?, sex.parse().ok()?),
            display.parse().ok()?,
        ))
    })?
    .into_iter()
    .collect())
}

/// Each line after the header, split at tabs and read by `row`.
fn parse_lines<T>(text: &str, row: impl Fn(&[&str]) -> Option<T>) -> Result<Vec<T>, String> {
    text.lines()
        .enumerate()
        .skip(1)
        .map(|(number, line)| {
            let fields: Vec<&str> = line.split('\t').collect();
            row(&fields).ok_or_else(|| format!("line {}: bad row {line:?}", number + 1))
        })
        .collect()
}
