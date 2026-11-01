//! Observed US Midnight Season 2 weeks; no future rotation period is assumed.
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct CitedBands {
    value: Bands,
    citation: String,
}

#[derive(Debug, Deserialize)]
struct Bands {
    guidance_min: u32,
    guidance_max: u32,
    guidance_id: u32,
    bargain_min: u32,
    bargain_max: u32,
    defense_min: u32,
    both_min: u32,
    fortified_id: u32,
    tyrannical_id: u32,
    guile_min: u32,
    guile_id: u32,
}

#[derive(Debug, Deserialize)]
struct Week {
    start_unix: i64,
    end_unix: i64,
    bargain: u32,
    defense: u32,
    citation: String,
    defense_citation: String,
    qualification: String,
}

#[derive(Debug, Deserialize)]
pub struct AffixSchedule {
    season: String,
    pub season_id: u32,
    season_id_citation: String,
    region: String,
    reset_citation: String,
    bands: CitedBands,
    weeks: Vec<Week>,
}

impl AffixSchedule {
    pub fn read(path: &std::path::Path) -> Result<Self, String> {
        let contents = std::fs::read_to_string(path)
            .map_err(|error| format!("Read Mythic+ affix weeks {}: {error}", path.display()))?;
        Self::parse(&contents)
            .map_err(|error| format!("Invalid Mythic+ affix weeks {}: {error}", path.display()))
    }

    fn parse(contents: &str) -> Result<Self, String> {
        let schedule: Self = serde_json::from_str(contents).map_err(|error| error.to_string())?;
        if schedule.season != "Midnight Season 2" || schedule.region != "US" {
            return Err("expected Midnight Season 2 US policy".into());
        }
        if schedule.season_id == 0
            || schedule.season_id_citation.is_empty()
            || schedule.reset_citation.is_empty()
            || schedule.bands.citation.is_empty()
        {
            return Err("missing policy citation".into());
        }
        for week in &schedule.weeks {
            if week.citation.is_empty()
                || week.defense_citation.is_empty()
                || week.qualification.is_empty()
                || week.start_unix >= week.end_unix
            {
                return Err("invalid weekly interval or missing citation".into());
            }
            if ![9, 10].contains(&week.defense) || ![148, 158, 160, 162].contains(&week.bargain) {
                return Err("unknown weekly affix".into());
            }
        }
        if schedule
            .weeks
            .windows(2)
            .any(|weeks| weeks[0].end_unix > weeks[1].start_unix)
        {
            return Err("overlapping or unordered weeks".into());
        }
        Ok(schedule)
    }

    pub fn affixes_at(&self, unix: i64, level: u32) -> Option<Vec<u32>> {
        let bands = &self.bands.value;
        if level < bands.guidance_min {
            return None;
        }
        let week = self
            .weeks
            .iter()
            .find(|week| (week.start_unix..week.end_unix).contains(&unix))?;
        let mut affixes = Vec::new();
        if level <= bands.guidance_max {
            affixes.push(bands.guidance_id);
        }
        if (bands.bargain_min..=bands.bargain_max).contains(&level) {
            affixes.push(week.bargain);
        }
        if level >= bands.defense_min {
            affixes.push(week.defense);
            if level >= bands.both_min {
                affixes.push(if week.defense == bands.fortified_id {
                    bands.tyrannical_id
                } else {
                    bands.fortified_id
                });
            }
        }
        if level >= bands.guile_min {
            affixes.push(bands.guile_id);
        }
        Some(affixes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mythic_weeks_level_bands_observed_weeks_and_dst_end() {
        let schedule = AffixSchedule::parse(include_str!("mythic_plus_weeks.json")).unwrap();
        for (start, bargain, defense) in [
            (1_787_065_200, 162, 10),
            (1_787_670_000, 158, 9),
            (1_788_274_800, 160, 10),
            (1_788_879_600, 162, 9),
            (1_789_484_400, 148, 10),
            (1_790_089_200, 158, 9),
            (1_790_694_000, 160, 10),
            (1_791_298_800, 148, 9),
            (1_791_903_600, 162, 10),
            (1_792_508_400, 158, 9),
            (1_793_113_200, 160, 10),
        ] {
            assert_eq!(schedule.affixes_at(start, 7), Some(vec![bargain, defense]));
        }
        let oct27 = 1_793_113_200;
        assert_eq!(schedule.affixes_at(oct27, 2), Some(vec![165]));
        assert_eq!(schedule.affixes_at(oct27, 5), Some(vec![165, 160]));
        assert_eq!(schedule.affixes_at(oct27, 7), Some(vec![160, 10]));
        assert_eq!(schedule.affixes_at(oct27, 10), Some(vec![160, 10, 9]));
        assert_eq!(schedule.affixes_at(oct27, 12), Some(vec![10, 9, 147]));
        assert_eq!(schedule.affixes_at(oct27 - 1, 7), Some(vec![158, 9]));
        // The observed week crosses the Pacific fall DST transition.
        assert_eq!(schedule.affixes_at(1_793_721_599, 7), Some(vec![160, 10]));
        assert_eq!(schedule.affixes_at(1_793_721_600, 7), None);
        assert_eq!(schedule.affixes_at(oct27, 1), None);
    }
}
