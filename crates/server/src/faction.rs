pub(crate) fn faction_for_race(race: u8) -> u8 {
    match race {
        1 | 3 | 4 | 7 | 11 | 22 | 25 | 29 | 30 | 32 | 34 | 37 => 1,
        2 | 5 | 6 | 8 | 9 | 10 | 26 | 27 | 28 | 31 | 35 | 36 => 2,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::faction_for_race;

    #[test]
    fn maps_alliance_race_to_alliance_faction() {
        assert_eq!(faction_for_race(1), 1);
    }

    #[test]
    fn maps_horde_race_to_horde_faction() {
        assert_eq!(faction_for_race(2), 2);
    }

    #[test]
    fn maps_unknown_race_to_neutral_faction() {
        assert_eq!(faction_for_race(0), 0);
    }
}
