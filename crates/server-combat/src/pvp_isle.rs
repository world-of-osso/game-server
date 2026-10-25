//! TC a352b1fa battleground_isle_of_conquest.cpp OnCreatureCreate: source
//! creature StringIds select the two generic Wintergrasp faction templates.
use crate::zones::CreatureStringIds;

const MAP_ID: u16 = 628;
const HORDE_GENERIC_WG: u32 = 1735;
const ALLIANCE_GENERIC_WG: u32 = 1732;

pub(crate) fn creature_faction(map_id: u16, string_ids: &CreatureStringIds) -> Option<u32> {
    if map_id != MAP_ID {
        return None;
    }
    if string_ids.has("bg_ioc_faction_1735") {
        Some(HORDE_GENERIC_WG)
    } else if string_ids.has("bg_ioc_faction_1732") {
        Some(ALLIANCE_GENERIC_WG)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn isle_string_ids_select_both_source_factions_without_changing_other_maps() {
        for (tag, faction) in [("bg_ioc_faction_1732", 1732), ("bg_ioc_faction_1735", 1735)] {
            let ids = CreatureStringIds {
                spawn: Some(tag.into()),
                ..Default::default()
            };
            assert_eq!(creature_faction(628, &ids), Some(faction));
            assert_eq!(creature_faction(30, &ids), None);
            assert_eq!(creature_faction(0, &ids), None);
        }
        assert_eq!(creature_faction(628, &CreatureStringIds::default()), None);
    }
}
