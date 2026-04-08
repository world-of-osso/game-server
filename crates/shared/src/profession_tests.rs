use super::*;

#[test]
fn profession_categories() {
    assert_eq!(Profession::Mining.category(), ProfessionCategory::Gathering);
    assert_eq!(
        Profession::Blacksmithing.category(),
        ProfessionCategory::Crafting
    );
    assert_eq!(
        Profession::Cooking.category(),
        ProfessionCategory::Secondary
    );
    assert!(Profession::Mining.is_primary());
    assert!(!Profession::Cooking.is_primary());
}

#[test]
fn profession_all_count() {
    assert_eq!(Profession::all().len(), 13);
}

#[test]
fn learn_profession() {
    let mut profs = PlayerProfessions::default();
    assert!(profs.learn(Profession::Mining).is_ok());
    assert!(profs.knows(Profession::Mining));
    assert_eq!(profs.get(Profession::Mining).unwrap().current, 1);
}

#[test]
fn learn_duplicate_rejected() {
    let mut profs = PlayerProfessions::default();
    profs.learn(Profession::Mining).unwrap();
    assert_eq!(
        profs.learn(Profession::Mining),
        Err(ProfessionError::AlreadyKnown)
    );
}

#[test]
fn max_two_primary() {
    let mut profs = PlayerProfessions::default();
    profs.learn(Profession::Mining).unwrap();
    profs.learn(Profession::Herbalism).unwrap();
    assert_eq!(
        profs.learn(Profession::Skinning),
        Err(ProfessionError::TooManyPrimary)
    );
}

#[test]
fn secondary_unlimited() {
    let mut profs = PlayerProfessions::default();
    profs.learn(Profession::Mining).unwrap();
    profs.learn(Profession::Herbalism).unwrap();
    // Secondary professions don't count toward the 2 limit
    assert!(profs.learn(Profession::Cooking).is_ok());
    assert!(profs.learn(Profession::Fishing).is_ok());
}

#[test]
fn skill_up() {
    let mut profs = PlayerProfessions::default();
    profs.learn(Profession::Mining).unwrap();
    let skill = profs.get_mut(Profession::Mining).unwrap();
    assert!(skill.skill_up());
    assert_eq!(skill.current, 2);
}

#[test]
fn skill_up_capped() {
    let mut skill = ProfessionSkill::new(Profession::Mining);
    skill.current = 75; // at max
    assert!(!skill.skill_up());
}

#[test]
fn train_tier() {
    let mut skill = ProfessionSkill::new(Profession::Mining);
    skill.train_tier(ProfessionTier::Journeyman);
    assert_eq!(skill.max, 150);
    skill.train_tier(ProfessionTier::GrandMaster);
    assert_eq!(skill.max, 450);
}

// --- Tier system tests ---

#[test]
fn tier_skill_caps() {
    assert_eq!(ProfessionTier::Apprentice.max_skill(), 75);
    assert_eq!(ProfessionTier::Journeyman.max_skill(), 150);
    assert_eq!(ProfessionTier::Expert.max_skill(), 225);
    assert_eq!(ProfessionTier::Artisan.max_skill(), 300);
    assert_eq!(ProfessionTier::Master.max_skill(), 375);
    assert_eq!(ProfessionTier::GrandMaster.max_skill(), 450);
}

#[test]
fn tier_required_levels() {
    assert_eq!(ProfessionTier::Apprentice.required_level(), 5);
    assert_eq!(ProfessionTier::Expert.required_level(), 20);
    assert_eq!(ProfessionTier::GrandMaster.required_level(), 65);
}

#[test]
fn tier_next() {
    assert_eq!(
        ProfessionTier::Apprentice.next(),
        Some(ProfessionTier::Journeyman)
    );
    assert_eq!(ProfessionTier::GrandMaster.next(), None);
}

#[test]
fn next_trainable_apprentice_to_journeyman() {
    // Apprentice (max=75), skill=50, level=10 → can train Journeyman
    let tier = next_trainable_tier(75, 50, 10);
    assert_eq!(tier, Some(ProfessionTier::Journeyman));
}

#[test]
fn next_trainable_skill_too_low() {
    // Max=75, skill=30, level=10 → need 50 skill for Journeyman
    let tier = next_trainable_tier(75, 30, 10);
    assert_eq!(tier, None);
}

#[test]
fn next_trainable_level_too_low() {
    // Max=75, skill=50, level=5 → need level 10 for Journeyman
    let tier = next_trainable_tier(75, 50, 5);
    assert_eq!(tier, None);
}

#[test]
fn next_trainable_at_grand_master() {
    let tier = next_trainable_tier(450, 450, 80);
    assert_eq!(tier, None); // already max
}

#[test]
fn unlearn_profession() {
    let mut profs = PlayerProfessions::default();
    profs.learn(Profession::Mining).unwrap();
    profs.unlearn(Profession::Mining);
    assert!(!profs.knows(Profession::Mining));
    assert!(profs.learn(Profession::Skinning).is_ok());
}

// --- Gathering tests ---

fn copper_vein() -> GatherNode {
    GatherNode {
        profession: Profession::Mining,
        required_skill: 1,
        trivial_skill: 50,
        yields: vec![(2770, 1, 3)], // Copper Ore
        respawn_secs: 300,
    }
}

fn mining_profs(skill: u16) -> PlayerProfessions {
    let mut profs = PlayerProfessions::default();
    profs.learn(Profession::Mining).unwrap();
    let s = profs.get_mut(Profession::Mining).unwrap();
    s.current = skill;
    s.max = 75;
    profs
}

#[test]
fn gather_success() {
    let node = copper_vein();
    let mut profs = mining_profs(10);
    let result = attempt_gather(&node, &mut profs, 0.5, &[0.5]);
    match result {
        GatherResult::Success { items, .. } => {
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].0, 2770);
            assert!(items[0].1 >= 1 && items[0].1 <= 3);
        }
        _ => panic!("expected success"),
    }
}

#[test]
fn gather_skill_too_low() {
    let mut node = copper_vein();
    node.required_skill = 50;
    let mut profs = mining_profs(10);
    assert_eq!(
        attempt_gather(&node, &mut profs, 0.0, &[]),
        GatherResult::SkillTooLow
    );
}

#[test]
fn gather_wrong_profession() {
    let node = copper_vein();
    let mut profs = PlayerProfessions::default();
    profs.learn(Profession::Herbalism).unwrap();
    assert_eq!(
        attempt_gather(&node, &mut profs, 0.0, &[]),
        GatherResult::WrongProfession
    );
}

#[test]
fn gather_skill_up_at_low_skill() {
    let node = copper_vein(); // required=1, trivial=50
    let mut profs = mining_profs(5);
    // roll=0.0 → always skill up (chance is high at skill 5)
    let result = attempt_gather(&node, &mut profs, 0.0, &[0.0]);
    match result {
        GatherResult::Success { skilled_up, .. } => assert!(skilled_up),
        _ => panic!("expected success"),
    }
    assert_eq!(profs.get(Profession::Mining).unwrap().current, 6);
}

#[test]
fn gather_no_skill_up_at_trivial() {
    let node = copper_vein();
    let mut profs = mining_profs(50); // at trivial
    let result = attempt_gather(&node, &mut profs, 0.0, &[0.0]);
    match result {
        GatherResult::Success { skilled_up, .. } => assert!(!skilled_up),
        _ => panic!("expected success"),
    }
}

#[test]
fn skill_up_chance_values() {
    assert_eq!(skill_up_chance(1, 1, 50), 1.0);
    assert_eq!(skill_up_chance(50, 1, 50), 0.0);
    let mid = skill_up_chance(25, 1, 50);
    assert!(mid > 0.0 && mid < 1.0);
}

// --- Crafting tests ---

fn copper_bar_recipe() -> Recipe {
    Recipe {
        id: 2657,
        profession: Profession::Mining,
        required_skill: 1,
        trivial_skill: 25,
        materials: vec![CraftMaterial {
            item_id: 2770,
            count: 2,
        }],
        output_item_id: 2840,
        output_count: 1,
        source: Some(RecipeSource::Trainer),
    }
}

#[test]
fn craft_success() {
    let recipe = copper_bar_recipe();
    let mut profs = mining_profs(10);
    let inv = |id: u32| if id == 2770 { 10 } else { 0 };
    let result = attempt_craft(&recipe, &mut profs, &inv, 0.5);
    match result {
        CraftResult::Success {
            output_item_id,
            output_count,
            ..
        } => {
            assert_eq!(output_item_id, 2840);
            assert_eq!(output_count, 1);
        }
        other => panic!("expected success, got {other:?}"),
    }
}

#[test]
fn craft_wrong_profession() {
    let recipe = copper_bar_recipe();
    let mut profs = PlayerProfessions::default();
    profs.learn(Profession::Herbalism).unwrap();
    let inv = |_: u32| 99;
    assert!(matches!(
        attempt_craft(&recipe, &mut profs, &inv, 0.0),
        CraftResult::WrongProfession
    ));
}

#[test]
fn craft_skill_too_low() {
    let mut recipe = copper_bar_recipe();
    recipe.required_skill = 50;
    let mut profs = mining_profs(10);
    let inv = |_: u32| 99;
    assert!(matches!(
        attempt_craft(&recipe, &mut profs, &inv, 0.0),
        CraftResult::SkillTooLow
    ));
}

#[test]
fn craft_missing_materials() {
    let recipe = copper_bar_recipe();
    let mut profs = mining_profs(10);
    let inv = |id: u32| if id == 2770 { 1 } else { 0 }; // only 1, need 2
    let result = attempt_craft(&recipe, &mut profs, &inv, 0.0);
    match result {
        CraftResult::MissingMaterials { missing } => {
            assert_eq!(missing.len(), 1);
            assert_eq!(missing[0].item_id, 2770);
            assert_eq!(missing[0].count, 1); // need 1 more
        }
        other => panic!("expected missing, got {other:?}"),
    }
}

#[test]
fn craft_skill_up() {
    let recipe = copper_bar_recipe();
    let mut profs = mining_profs(5);
    let inv = |_: u32| 99;
    let result = attempt_craft(&recipe, &mut profs, &inv, 0.0); // roll=0 → skill up
    match result {
        CraftResult::Success { skilled_up, .. } => assert!(skilled_up),
        other => panic!("expected success, got {other:?}"),
    }
    assert_eq!(profs.get(Profession::Mining).unwrap().current, 6);
}

#[test]
fn check_materials_all_present() {
    let recipe = copper_bar_recipe();
    let inv = |id: u32| if id == 2770 { 5 } else { 0 };
    let missing = check_materials(&recipe, &inv);
    assert!(missing.is_empty());
}

#[test]
fn check_materials_partial() {
    let recipe = Recipe {
        id: 1,
        profession: Profession::Blacksmithing,
        required_skill: 1,
        trivial_skill: 50,
        materials: vec![
            CraftMaterial {
                item_id: 100,
                count: 5,
            },
            CraftMaterial {
                item_id: 200,
                count: 3,
            },
        ],
        output_item_id: 300,
        output_count: 1,
        source: None,
    };
    let inv = |id: u32| match id {
        100 => 2,
        200 => 3,
        _ => 0,
    };
    let missing = check_materials(&recipe, &inv);
    assert_eq!(missing.len(), 1);
    assert_eq!(
        missing[0],
        CraftMaterial {
            item_id: 100,
            count: 3
        }
    );
}

// --- Profession specialization tests ---

fn bs_profs(skill: u16) -> PlayerProfessions {
    let mut profs = PlayerProfessions::default();
    profs.learn(Profession::Blacksmithing).unwrap();
    let s = profs.get_mut(Profession::Blacksmithing).unwrap();
    s.current = skill;
    s.max = 300;
    profs
}

#[test]
fn specs_for_blacksmithing() {
    let specs = specs_for_profession(Profession::Blacksmithing);
    assert_eq!(specs.len(), 2);
}

#[test]
fn choose_spec() {
    let profs = bs_profs(250);
    let mut chosen = ChosenSpecs::default();
    assert!(chosen.choose(1, &profs).is_ok()); // Weaponsmith
    assert!(chosen.has_spec(1));
    assert_eq!(
        chosen
            .spec_for_profession(Profession::Blacksmithing)
            .unwrap()
            .name,
        "Weaponsmith"
    );
}

#[test]
fn choose_spec_skill_too_low() {
    let profs = bs_profs(100); // need 200
    let mut chosen = ChosenSpecs::default();
    assert_eq!(chosen.choose(1, &profs), Err(SpecError::SkillTooLow));
}

#[test]
fn choose_spec_already_specialized() {
    let profs = bs_profs(250);
    let mut chosen = ChosenSpecs::default();
    chosen.choose(1, &profs).unwrap(); // Weaponsmith
    assert_eq!(chosen.choose(2, &profs), Err(SpecError::AlreadySpecialized));
}

#[test]
fn drop_and_respec() {
    let profs = bs_profs(250);
    let mut chosen = ChosenSpecs::default();
    chosen.choose(1, &profs).unwrap();
    chosen.drop_spec(Profession::Blacksmithing);
    assert!(!chosen.has_spec(1));
    assert!(chosen.choose(2, &profs).is_ok()); // Armorsmith now
}

#[test]
fn multiple_profession_specs() {
    let mut profs = bs_profs(250);
    profs.learn(Profession::Alchemy).unwrap();
    let s = profs.get_mut(Profession::Alchemy).unwrap();
    s.current = 350;
    s.max = 450;

    let mut chosen = ChosenSpecs::default();
    chosen.choose(1, &profs).unwrap(); // Weaponsmith
    chosen.choose(6, &profs).unwrap(); // Elixir Master
    assert_eq!(chosen.spec_ids.len(), 2);
}

#[test]
fn total_spec_count() {
    assert_eq!(PROFESSION_SPECS.len(), 13);
}

// --- Recipe source tests ---

#[test]
fn learn_recipe() {
    let mut known = KnownRecipes::default();
    assert!(known.learn(100));
    assert!(known.knows(100));
    assert!(!known.learn(100)); // duplicate
}

#[test]
fn trainer_recipes_filtered_by_skill() {
    let recipes = vec![
        Recipe {
            id: 1,
            profession: Profession::Mining,
            required_skill: 1,
            trivial_skill: 25,
            materials: vec![],
            output_item_id: 100,
            output_count: 1,
            source: Some(RecipeSource::Trainer),
        },
        Recipe {
            id: 2,
            profession: Profession::Mining,
            required_skill: 50,
            trivial_skill: 75,
            materials: vec![],
            output_item_id: 200,
            output_count: 1,
            source: Some(RecipeSource::Trainer),
        },
        Recipe {
            id: 3,
            profession: Profession::Mining,
            required_skill: 1,
            trivial_skill: 25,
            materials: vec![],
            output_item_id: 300,
            output_count: 1,
            source: Some(RecipeSource::Item { item_id: 999 }),
        },
    ];
    let available = KnownRecipes::available_from_trainer(&recipes, 30);
    assert_eq!(available.len(), 1); // only recipe 1 (trainer, skill ≤ 30)
    assert_eq!(available[0].id, 1);
}

#[test]
fn discovery_roll() {
    let discoverable = vec![Recipe {
        id: 50,
        profession: Profession::Alchemy,
        required_skill: 300,
        trivial_skill: 350,
        materials: vec![],
        output_item_id: 500,
        output_count: 1,
        source: Some(RecipeSource::Discovery { chance: 100 }),
    }];
    let mut known = KnownRecipes::default();
    // roll=50 < chance=100 → discovered
    let found = known.roll_discovery(&discoverable, 50);
    assert_eq!(found, Some(50));
    assert!(known.knows(50));
    // Can't discover again
    let found2 = known.roll_discovery(&discoverable, 50);
    assert_eq!(found2, None);
}

#[test]
fn discovery_high_roll_fails() {
    let discoverable = vec![Recipe {
        id: 50,
        profession: Profession::Alchemy,
        required_skill: 300,
        trivial_skill: 350,
        materials: vec![],
        output_item_id: 500,
        output_count: 1,
        source: Some(RecipeSource::Discovery { chance: 100 }),
    }];
    let mut known = KnownRecipes::default();
    let found = known.roll_discovery(&discoverable, 200); // > 100
    assert_eq!(found, None);
}
