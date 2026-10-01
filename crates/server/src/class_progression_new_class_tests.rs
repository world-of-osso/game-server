//! Monk, Demon Hunter and Evoker spell grants from their class skill lines
//! (829, 1848, 2810) and specializations, on the real world.db.

use super::*;

const NIGHT_ELF: u8 = 4;
const DRACTHYR_ALLIANCE: u8 = 52;
const MONK: u8 = 10;
const DEMON_HUNTER: u8 = 12;
const EVOKER: u8 = 13;

const TIGER_PALM: u32 = 100_780;
const BLACKOUT_KICK: u32 = 100_784;
const DEMONS_BITE_INITIAL: u32 = 344_859;
const CHAOS_STRIKE_INITIAL: u32 = 344_862;
const FEL_RUSH_INITIAL: u32 = 344_865;
const TORMENT: u32 = 185_245;
const DEMONS_BITE: u32 = 162_243;
const CHAOS_STRIKE: u32 = 162_794;
const LIVING_FLAME: u32 = 361_469;
const AZURE_STRIKE: u32 = 362_969;
const DISINTEGRATE: u32 = 356_995;

const SPEC_BREWMASTER: u32 = 268;
const SPEC_MONK_INITIAL: u32 = 1450;
const SPEC_HAVOC: u32 = 577;
const SPEC_DH_INITIAL: u32 = 1456;
const SPEC_DEVASTATION: u32 = 1467;

#[test]
fn new_classes_load_their_skill_line_and_specs() {
    let Some(classes) = class_spells() else {
        return;
    };
    for (class_id, skill_line, specs) in [
        (MONK, 829, vec![268, 270, 269, SPEC_MONK_INITIAL]),
        (DEMON_HUNTER, 1848, vec![577, 581, 1480, SPEC_DH_INITIAL]),
        (EVOKER, 2810, vec![1467, 1468, 1473, 1465]),
    ] {
        let class = class_info(&classes, class_id).unwrap();
        assert_eq!(class.skill_line, skill_line);
        let ids: Vec<u32> = class.specs.iter().map(|spec| spec.spec_id).collect();
        assert_eq!(ids, specs, "class {class_id}");
    }
}

#[test]
fn level_1_monk_knows_tiger_palm_and_learns_blackout_kick_at_2() {
    let Some(classes) = class_spells() else {
        return;
    };
    let level_1 = known_ids(&classes, HUMAN, MONK, 1, 0);
    assert!(level_1.contains(&TIGER_PALM));
    assert!(!level_1.contains(&BLACKOUT_KICK));

    let mut app = setup_app(temp_database(), classes);
    let player = spawn_player(&mut app, 10, HUMAN, MONK, 1);
    advance(&mut app);
    assert_eq!(
        received::<SpecializationChanged>(&mut app, player),
        vec![SpecializationChanged {
            spec_id: SPEC_MONK_INITIAL
        }]
    );
    drain(&mut app, player);

    set_live_level(app.world_mut(), 10, 2);
    advance(&mut app);
    assert_eq!(
        received::<SpellsLearned>(&mut app, player),
        vec![SpellsLearned {
            spells: vec![BLACKOUT_KICK],
        }]
    );

    set_live_level(app.world_mut(), 10, 10);
    advance(&mut app);
    assert_eq!(
        received::<SpecializationChanged>(&mut app, player),
        vec![SpecializationChanged {
            spec_id: SPEC_BREWMASTER
        }]
    );
}

/// A new Demon Hunter is level 8 in the Initial spec with the baseline
/// Demon's Bite / Chaos Strike / Fel Rush; Torment arrives at 9 and Havoc's
/// versions at 10.
#[test]
fn demon_hunter_levels_8_to_10_learn_torment_then_havoc() {
    let Some(classes) = class_spells() else {
        return;
    };
    let level_8 = known_ids(&classes, NIGHT_ELF, DEMON_HUNTER, 8, 0);
    for spell_id in [DEMONS_BITE_INITIAL, CHAOS_STRIKE_INITIAL, FEL_RUSH_INITIAL] {
        assert!(level_8.contains(&spell_id), "missing {spell_id}");
    }
    assert!(!level_8.contains(&TORMENT));

    let mut app = setup_app(temp_database(), classes);
    let player = spawn_player(&mut app, 12, NIGHT_ELF, DEMON_HUNTER, 8);
    advance(&mut app);
    assert_eq!(
        received::<SpecializationChanged>(&mut app, player),
        vec![SpecializationChanged {
            spec_id: SPEC_DH_INITIAL
        }]
    );
    drain(&mut app, player);

    set_live_level(app.world_mut(), 12, 9);
    advance(&mut app);
    assert_eq!(
        received::<SpellsLearned>(&mut app, player),
        vec![SpellsLearned {
            spells: vec![TORMENT],
        }]
    );

    set_live_level(app.world_mut(), 12, 10);
    advance(&mut app);
    assert_eq!(
        received::<SpecializationChanged>(&mut app, player),
        vec![SpecializationChanged {
            spec_id: SPEC_HAVOC
        }]
    );
    let learned = received::<SpellsLearned>(&mut app, player);
    assert_eq!(learned.len(), 1);
    for spell_id in [DEMONS_BITE, CHAOS_STRIKE] {
        assert!(learned[0].spells.contains(&spell_id), "missing {spell_id}");
    }
    // Havoc's SpecializationSpells replace the baselines (OverridesSpellID
    // 344859/344862/344865): they are unlearned and their bar buttons move over.
    let unlearned = received::<SpellsUnlearned>(&mut app, player);
    assert_eq!(unlearned.len(), 1);
    for spell_id in [DEMONS_BITE_INITIAL, CHAOS_STRIKE_INITIAL, FEL_RUSH_INITIAL] {
        assert!(
            unlearned[0].spells.contains(&spell_id),
            "{spell_id} stays known"
        );
    }
    let known = app.world().get::<KnownSpells>(player).unwrap();
    assert!(!known.contains(DEMONS_BITE_INITIAL) && known.contains(DEMONS_BITE));
    let bar = &app.world().get::<ActionBar>(player).unwrap().0;
    let on_bar = |spell_id| {
        bar.values()
            .any(|action| *action == ActionRef::Spell(spell_id))
    };
    assert!(on_bar(DEMONS_BITE) && on_bar(CHAOS_STRIKE) && on_bar(195_072));
    assert!(!on_bar(DEMONS_BITE_INITIAL) && !on_bar(CHAOS_STRIKE_INITIAL));
}

/// Dracthyr start at 10, so a new Evoker is Devastation with its baseline
/// Living Flame, Azure Strike and Disintegrate; its create actions put Living
/// Flame and Azure Strike on the bar.
#[test]
fn new_evoker_enters_as_devastation_with_core_spells_on_the_bar() {
    let Some(classes) = class_spells() else {
        return;
    };
    let db = temp_database();
    let Some(character_id) = create_with_kit(&db, "Kalecgos", DRACTHYR_ALLIANCE, EVOKER) else {
        return;
    };
    let mut app = setup_app(db, classes);
    let player = spawn_player(&mut app, character_id, DRACTHYR_ALLIANCE, EVOKER, 10);
    advance(&mut app);
    assert_eq!(
        received::<SpecializationChanged>(&mut app, player),
        vec![SpecializationChanged {
            spec_id: SPEC_DEVASTATION
        }]
    );
    let known = app.world().get::<KnownSpells>(player).unwrap();
    for spell_id in [LIVING_FLAME, AZURE_STRIKE, DISINTEGRATE] {
        assert!(known.contains(spell_id), "missing {spell_id}");
    }
    let bar = received::<ActionBarSnapshot>(&mut app, player);
    let on_bar = |spell_id| {
        bar[0]
            .slots
            .iter()
            .any(|(_, action)| *action == ActionRef::Spell(spell_id))
    };
    assert!(on_bar(LIVING_FLAME) && on_bar(AZURE_STRIKE));
    assert!(
        !known_ids(
            app.world().resource::<ClassSpells>(),
            DRACTHYR_ALLIANCE,
            EVOKER,
            10,
            0
        )
        .contains(&361_227)
    );
    drain(&mut app, player);
    set_live_level(app.world_mut(), character_id, 12);
    advance(&mut app);
    assert_eq!(
        received::<SpellsLearned>(&mut app, player),
        vec![SpellsLearned {
            spells: vec![361_227]
        }],
    );
}

#[test]
fn new_classes_switch_specs_and_persist_their_authored_spell_grants() {
    let classes = class_spells().expect("new-class progression requires local world.db");
    let mut app = setup_app(temp_database(), classes);
    for (id, race, class, spec_id, passive, spec_spell) in [
        (101, HUMAN, MONK, 269, 137_025, 115_636),
        (102, NIGHT_ELF, DEMON_HUNTER, 581, 212_613, 203_783),
        (103, DRACTHYR_ALLIANCE, EVOKER, 1468, 356_810, 360_823),
    ] {
        let player = spawn_player(&mut app, id, race, class, 10);
        advance(&mut app);
        drain(&mut app, player);
        send_request(&mut app, player, SetSpecialization { spec_id });
        assert_eq!(
            received::<SpecializationChanged>(&mut app, player),
            vec![SpecializationChanged { spec_id }],
        );
        let known = app.world().get::<KnownSpells>(player).unwrap();
        assert!(
            known.contains(passive),
            "class {class} spec passive {passive}"
        );
        assert!(
            known.contains(spec_spell),
            "class {class} spec spell {spec_spell}"
        );
        assert_eq!(stored(&app, id).spec_id, spec_id);
        let learned = received::<SpellsLearned>(&mut app, player);
        assert_eq!(learned.len(), 1);
        assert!(learned[0].spells.contains(&passive));
        assert!(
            learned[0].spells.contains(&spec_spell),
            "class {class} delta lacks {spec_spell}"
        );
    }
}
