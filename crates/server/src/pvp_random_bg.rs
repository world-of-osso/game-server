//! RANDOM_BG_LEVEL_ELIGIBLE_MAPS: explicit Retail-based project policy, not TC RangeIndex copying.
use bevy::prelude::*;
use rusqlite::{Connection, OptionalExtension};
use shared::battleground::{
    BgMatch, BgQueue, BgTemplate, LevelBracket, source_level_bracket, template_by_id,
};

// Authenticated70448 BattlemasterListXMap links for queue32, including missing Deepwind1037.
const SOURCE_MAPS: &[(u32, u32)] = &[
    (7, 566),
    (108, 726),
    (708, 727),
    (120, 761),
    (699, 998),
    (894, 1803),
    (1014, 2106),
    (1018, 2107),
    (1037, 2245),
    (1110, 2656),
];

#[derive(Clone)]
struct ConfiguredMap {
    template: BgTemplate,
    weight: u32,
}

#[derive(Resource, Default)]
pub(crate) struct Catalog(Vec<ConfiguredMap>);

#[derive(Clone)]
struct EligibleMap {
    template: BgTemplate,
    bracket: LevelBracket,
    weight: u32,
}

impl Catalog {
    pub(crate) fn load(db: &Connection) -> rusqlite::Result<Self> {
        let mut query = db.prepare("SELECT Weight FROM tdb_battleground_template WHERE ID=?1")?;
        let mut maps = Vec::new();
        for &(id, map_id) in SOURCE_MAPS {
            let weight = query
                .query_row([id], |row| row.get::<_, u32>(0))
                .optional()?;
            let Some(weight) = weight.filter(|weight| *weight > 0) else {
                continue;
            };
            let template = template_by_id(id)
                .filter(|template| template.map_id == map_id)
                .ok_or_else(|| {
                    rusqlite::Error::InvalidParameterName(format!(
                        "configured Random32 template{id}/map{map_id} has no implemented lifecycle"
                    ))
                })?;
            maps.push(ConfiguredMap { template, weight });
        }
        Ok(Self(maps))
    }

    fn eligible(&self, levels: &[u8]) -> Vec<EligibleMap> {
        let Some(&first) = levels.first() else {
            return Vec::new();
        };
        self.0
            .iter()
            .filter_map(|configured| {
                let template = &configured.template;
                let bracket = source_level_bracket(template.map_id, first)?;
                let all_eligible = levels.iter().all(|&level| {
                    template.bracket.contains(level)
                        && source_level_bracket(template.map_id, level) == Some(bracket)
                });
                all_eligible.then(|| EligibleMap {
                    template: template.clone(),
                    bracket,
                    weight: configured.weight,
                })
            })
            .collect()
    }

    pub(super) fn validate_levels(&self, levels: &[u8]) -> Result<(), String> {
        if self.eligible(levels).is_empty() {
            return Err(
                "no configured Random Battleground map is eligible for this level range".into(),
            );
        }
        Ok(())
    }
}

fn select_ticket(maps: &[EligibleMap], mut ticket: u64) -> Option<&EligibleMap> {
    for map in maps {
        if ticket < u64::from(map.weight) {
            return Some(map);
        }
        ticket -= u64::from(map.weight);
    }
    None
}

/// Draw a configured eligible map before committing removal of the parent queue roster.
pub(super) fn draw_match(queue: &mut BgQueue, catalog: &Catalog) -> Option<BgMatch> {
    let mut pending = queue.clone();
    let mut matched = pending.try_match(shared::battleground::RANDOM_BATTLEGROUND_ID)?;
    let levels: Vec<_> = matched
        .team_a
        .iter()
        .chain(&matched.team_b)
        .map(|&player| {
            queue
                .player_level(player)
                .expect("matched queued player level")
        })
        .collect();
    let maps = catalog.eligible(&levels);
    let total: u64 = maps.iter().map(|map| u64::from(map.weight)).sum();
    if total == 0 {
        return None;
    }
    let ticket = rand::Rng::gen_range(&mut rand::thread_rng(), 0..total);
    let selected = select_ticket(&maps, ticket).expect("ticket within configured weights");
    matched.bg_id = selected.template.id;
    matched.bracket = selected.bracket;
    *queue = pending;
    Some(matched)
}

#[cfg(test)]
#[path = "pvp_random_bg_tests.rs"]
pub(crate) mod tests;
