use super::*;

include!("skyborne_faction_fixture.rs");

/// FactionTemplate IDs (Retail DB2, AzerothCore `creature_template.faction`).
const HUMAN: u32 = 1;
const ORC: u32 = 2;
/// Stormwind City Guard (creature 68).
const STORMWIND_GUARD: u32 = 11;
/// Kobold Vermin (creature 6, Northshire).
const KOBOLD: u32 = 25;
/// Rabbit (creature 721).
const CRITTER: u32 = 31;

fn player(template: u32) -> UnitFaction {
    UnitFaction {
        template,
        is_player: true,
    }
}

fn npc(template: u32) -> UnitFaction {
    UnitFaction {
        template,
        is_player: false,
    }
}

#[test]
fn maps_alliance_race_to_alliance_faction() {
    assert_eq!(faction_for_race(1), 1);
}

#[test]
fn maps_horde_race_to_horde_faction() {
    assert_eq!(faction_for_race(2), 2);
}

#[test]
fn skyborne_factions_follow_forever_chr_races() {
    for (race, faction, group, template) in [(95, 1, 3, 3629), (96, 2, 5, 3628)] {
        assert_eq!(faction_for_race(race), faction);
        assert_eq!(faction_group_for_race(race), group);
        assert_eq!(faction_template_for_race(race), template);
    }
}

#[test]
fn toyfx20_racial_factions_include_modern_chr_races() {
    // Local 12.1.0.69933 ChrRaces.FactionID -> FactionTemplate.FactionGroup.
    for (race, faction, template) in [
        (52, 1, 1),
        (70, 2, 2),
        (84, 2, 2),
        (85, 1, 3),
        (86, 1, 4),
        (91, 2, 116),
    ] {
        assert_eq!(faction_for_race(race), faction, "race {race}");
        assert_eq!(faction_template_for_race(race), template);
    }
    assert_eq!(faction_for_race(24), 0);
}

#[test]
fn maps_unknown_race_to_neutral_faction() {
    assert_eq!(faction_for_race(0), 0);
}

#[test]
fn race_faction_templates_come_from_chr_races() {
    assert_eq!(faction_template_for_race(1), HUMAN);
    assert_eq!(faction_template_for_race(2), ORC);
    assert_eq!(faction_template_for_race(10), 1610);
    assert_eq!(faction_template_for_race(22), 2203);
}

#[test]
fn skyborne_imported_templates_have_authored_reactions() {
    let mut templates = world_db_faction_templates().expect("world.db factions");
    let scratch = Connection::open_in_memory().expect("scratch database");
    scratch
        .execute_batch(SKYBORNE_FACTION_SQL)
        .expect("source-decoded faction fixture");
    templates.0.extend(
        load_faction_templates(&scratch)
            .expect("imported templates")
            .0,
    );
    for (skyborne, ally, enemy) in [(3629, HUMAN, ORC), (3628, ORC, HUMAN)] {
        assert_eq!(templates.reaction(skyborne, ally), Reaction::Friendly);
        assert_eq!(templates.reaction(ally, skyborne), Reaction::Friendly);
        assert_eq!(templates.reaction(skyborne, enemy), Reaction::Hostile);
        assert_eq!(templates.reaction(enemy, skyborne), Reaction::Hostile);
        assert!(templates.can_assist(player(skyborne), player(ally)));
        assert!(!templates.can_assist(player(skyborne), player(enemy)));
        assert!(!templates.can_attack(player(skyborne), player(enemy)));
    }
}

#[test]
fn retail_vendor_is_neutral_with_authored_hostile_friendly_and_critter_controls() {
    let templates = world_db_faction_templates().expect("authentic Retail faction templates");
    assert_eq!(templates.reaction(HUMAN, 2007), Reaction::Neutral);
    assert_eq!(templates.reaction(2007, HUMAN), Reaction::Neutral);
    assert!(templates.can_attack(player(HUMAN), npc(CRITTER)));
    assert!(templates.can_attack(player(HUMAN), npc(KOBOLD)));
    assert!(!templates.can_attack(player(HUMAN), npc(STORMWIND_GUARD)));
}

#[test]
fn stormwind_guard_is_friendly_with_human_player() {
    let Some(templates) = world_db_faction_templates() else {
        return;
    };
    assert_eq!(
        templates.reaction(STORMWIND_GUARD, HUMAN),
        Reaction::Friendly
    );
    assert_eq!(
        templates.reaction(HUMAN, STORMWIND_GUARD),
        Reaction::Friendly
    );
    assert!(!templates.can_attack(player(HUMAN), npc(STORMWIND_GUARD)));
    assert!(templates.can_assist(player(HUMAN), npc(STORMWIND_GUARD)));
}

#[test]
fn stormwind_guard_is_hostile_to_orc_player() {
    let Some(templates) = world_db_faction_templates() else {
        return;
    };
    assert_eq!(templates.reaction(STORMWIND_GUARD, ORC), Reaction::Hostile);
    assert_eq!(templates.reaction(ORC, STORMWIND_GUARD), Reaction::Hostile);
    assert!(templates.can_attack(player(ORC), npc(STORMWIND_GUARD)));
    assert!(!templates.can_assist(player(ORC), npc(STORMWIND_GUARD)));
}

#[test]
fn northshire_kobold_is_hostile_to_human_player() {
    let Some(templates) = world_db_faction_templates() else {
        return;
    };
    assert_eq!(templates.reaction(HUMAN, KOBOLD), Reaction::Hostile);
    // The kobold template has no enemy mask: its own view is neutral.
    assert_eq!(templates.reaction(KOBOLD, HUMAN), Reaction::Neutral);
    assert!(templates.can_attack(player(HUMAN), npc(KOBOLD)));
    assert!(templates.can_attack(npc(KOBOLD), player(HUMAN)));
    assert!(!templates.can_assist(player(HUMAN), npc(KOBOLD)));
}

#[test]
fn critter_is_neutral_and_attackable_by_players_only() {
    let Some(templates) = world_db_faction_templates() else {
        return;
    };
    assert_eq!(templates.reaction(HUMAN, CRITTER), Reaction::Neutral);
    assert_eq!(templates.reaction(CRITTER, HUMAN), Reaction::Neutral);
    assert!(templates.can_attack(player(HUMAN), npc(CRITTER)));
    // Creature vs creature needs a hostile side; kobold and critter are neutral.
    assert!(!templates.can_attack(npc(KOBOLD), npc(CRITTER)));
}

#[test]
fn opposing_players_are_hostile_but_not_attackable_without_pvp() {
    let Some(templates) = world_db_faction_templates() else {
        return;
    };
    assert_eq!(templates.reaction(HUMAN, ORC), Reaction::Hostile);
    assert!(!templates.can_attack(player(HUMAN), player(ORC)));
    assert!(templates.can_assist(player(HUMAN), player(HUMAN)));
}

#[test]
fn unknown_template_is_neutral() {
    let Some(templates) = world_db_faction_templates() else {
        return;
    };
    assert_eq!(templates.reaction(0, HUMAN), Reaction::Neutral);
    assert_eq!(templates.reaction(HUMAN, 0), Reaction::Neutral);
}
