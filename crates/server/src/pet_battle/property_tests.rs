use super::*;

#[test]
fn pet_battle_forced_swap_priority_cleanup_stays_on_original_pet() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [563, 110, 123]);
    b.teams[1].pets[0].abilities = [780, 110, 123];
    b.teams[1].pets[0].stats.speed = 15000;
    b.teams[1].pets[0]
        .states
        .insert(c.state_id("Stat_Speed").unwrap(), 10000);
    b.teams[0].pets[1].health = 2000;
    // Both pets have first-strike priority; enemy15000 speed breaks that tie.
    // End-round -10000 must restore the OLD caster, not subtract from incoming.
    let (b, _) = b
        .step(
            &c,
            [Action::Ability(563), Action::Ability(780)],
            &mut Fixed(500000),
        )
        .unwrap();
    assert_eq!(b.teams[0].active, 1);
    for pet in &b.teams[0].pets {
        assert_eq!(pet.state(c.state_id("Stat_Speed").unwrap()), 0);
    }
}

#[test]
fn pet_battle_50_moth_balls_state_gate_before_accuracy() {
    let c = catalog();
    let b = battle(&c, 7, 7, [507, 110, 123]);
    let mut rng = Fixed(400000);
    let mut rt = super::super::engine::Runtime {
        battle: b,
        catalog: &c,
        rng: &mut rng,
        events: Vec::new(),
        healing_target: None,
        used_ability: None,
    };
    let caster = PetRef { team: 0, slot: 0 };
    let target = PetRef { team: 1, slot: 0 };
    // Real Moth Balls507 effect1165:caster state127 required;50% hit;
    // aura154 reduces speed by25% for3 rounds. Gate must not consume a hit roll.
    let effect = &c.effects[&1165];
    assert!(
        !rt.execute_effect(caster, target, 507, effect, false)
            .unwrap()
    );
    rt.battle
        .pet_mut(caster)
        .states
        .insert(c.state_id("Condition_DidDamageThisRound").unwrap(), 1);
    assert!(
        rt.execute_effect(caster, target, 507, effect, false)
            .unwrap()
    );
    assert_eq!(
        rt.battle
            .pet(target)
            .state(c.state_id("Mod_SpeedPercent").unwrap()),
        -25
    );
}

#[test]
fn pet_battle_31_critter_rejects_direct_stun_state() {
    let c = catalog();
    let b = battle(&c, 4, 7, [292, 110, 123]);
    let mut rng = Fixed(500000);
    let mut rt = super::super::engine::Runtime {
        battle: b,
        catalog: &c,
        rng: &mut rng,
        events: Vec::new(),
        healing_target: None,
        used_ability: None,
    };
    let effect = c
        .ability_turns(292)
        .into_iter()
        .flat_map(|t| c.turn_effects(t.id))
        .find(|e| e.property == 31 && e.params[0] == 35)
        .unwrap();
    // GM Stun292 tries to set turnLock35=1; Critter passive42 makes CC immune.
    assert!(
        !rt.execute_effect(
            PetRef { team: 0, slot: 0 },
            PetRef { team: 1, slot: 0 },
            292,
            effect,
            false
        )
        .unwrap()
    );
    assert_eq!(rt.battle.teams[0].pets[0].state(35), 0);
}

#[test]
fn pet_battle_undetermined_kill_targets_are_rejected_before_rng() {
    let c = catalog();
    let b = battle(&c, 7, 7, [967, 110, 123]);
    // GM Kill All Enemy967 has3 identical135 rows. No guessed self-kill routing.
    let err = b
        .step(&c, [Action::Ability(967), Action::Pass], &mut NoDraw)
        .unwrap_err();
    assert!(err.contains("undetermined"), "{err}");
}

#[test]
fn pet_battle_decoy_refresh_restores_two_team_charges() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [334, 110, 123]);
    b.teams[0].pets[0].stats.speed = 200;
    b = b
        .step(
            &c,
            [Action::Ability(334), Action::Ability(110)],
            &mut Fixed(500000),
        )
        .unwrap()
        .0;
    b.teams[0].pets[0].cooldowns.clear();
    // Recast334 after one block refreshes back to exactly2 charges, not1 or3.
    b = use_ability(&b, &c, 334).0;
    for _ in 0..2 {
        let (next, e) = b
            .step(&c, [Action::Pass, Action::Ability(110)], &mut Fixed(500000))
            .unwrap();
        assert!(damage(&e).is_empty());
        b = next;
    }
    assert!(b.auras.is_empty());
    for pet in &b.teams[0].pets {
        assert_eq!(
            pet.state(c.state_id("Special_BlockedAttackCount").unwrap()),
            0
        );
    }
}

#[test]
fn pet_battle_246_bend_time_clears_incoming_cooldowns_once() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [1905, 110, 123]);
    b.teams[0].pets[1].cooldowns.insert(123, 9);
    b.teams[0].pets[2].cooldowns.insert(123, 9);
    // Bend Time1905/1904 indices1..3 clear only the next incoming pet's cooldowns.
    // Accuracy0 and removal Chance0 here encode unconditional scripted operations.
    b = use_ability(&b, &c, 1905).0;
    let (b, _) = b
        .step(&c, [Action::Swap(1), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(
        b.teams[0].pets[1].cooldowns.get(&123).copied().unwrap_or(0),
        0
    );
    assert!(b.auras.is_empty());
    assert_eq!(b.teams[0].pets[2].cooldowns[&123], 7);
}

#[test]
fn pet_battle_85_tidal_wave_cleans_objects_and_attacks_all_slots() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [334, 419, 110]);
    b = use_ability(&b, &c, 334).0;
    assert_eq!(b.auras.len(), 1);
    // Tidal Wave419:three enemy-slot7-point attacks,7*6=42 each;
    // state69 reset removes team objects without damaging friendly wearers.
    let (b, e) = use_ability(&b, &c, 419);
    assert_eq!(damage(&e), vec![42, 42, 42]);
    assert!(b.auras.is_empty());
    for pet in &b.teams[0].pets {
        assert_eq!(pet.state(c.state_id("Mechanic_IsObject").unwrap()), 0);
    }
}

#[test]
fn pet_battle_111_49_failsafe_revives_twenty_percent() {
    let c = catalog();
    let mut b = battle(&c, 9, 7, [723, 110, 123]);
    b.teams[0].pets[0].health = 0;
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Is_Dead").unwrap(), 1);
    // Failsafe723:unused parameter20 on removal49 isn't a second hit check.
    // Resurrection111 restores20% of10000=2000, clears dead1 to0.
    let mut rng = Fixed(500000);
    let mut rt = super::super::engine::Runtime {
        battle: b,
        catalog: &c,
        rng: &mut rng,
        events: Vec::new(),
        healing_target: None,
        used_ability: None,
    };
    assert!(validate_ability(&c, 723).is_ok());
    rt.cast_turn(
        PetRef { team: 0, slot: 0 },
        PetRef { team: 1, slot: 0 },
        723,
        1,
        255,
    )
    .unwrap();
    assert_eq!(rt.battle.teams[0].pets[0].health, 2000);
    assert_eq!(
        rt.battle.teams[0].pets[0].state(c.state_id("Is_Dead").unwrap()),
        0
    );
}

#[test]
fn pet_battle_373_reconstruction_changes_three_ability_slots() {
    let c = catalog();
    let b = battle(&c, 7, 7, [110, 119, 123]);
    // Necrotic Reconstruction2262 explicitly writes slots0,1,2 to2306,256,2304.
    let mut rng = Fixed(500000);
    let mut rt = super::super::engine::Runtime {
        battle: b,
        catalog: &c,
        rng: &mut rng,
        events: Vec::new(),
        healing_target: None,
        used_ability: None,
    };
    rt.cast_turn(
        PetRef { team: 0, slot: 0 },
        PetRef { team: 1, slot: 0 },
        2262,
        0,
        0,
    )
    .unwrap();
    assert_eq!(rt.battle.teams[0].pets[0].abilities, [2306, 256, 2304]);
    assert_eq!(rt.battle.teams[0].pets[1].abilities, [110, 119, 123]);
}

#[test]
fn pet_battle_169_scorched_earth_ticks_both_fronts_elemental_immune() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [172, 110, 123]);
    // Scorched Earth171 tick4*(1+.05*100)=24, only two front pets each round.
    let (next, e) = use_ability(&b, &c, 172);
    assert!(damage(&e).ends_with(&[24, 24]));
    assert_eq!(next.teams[0].pets[1].health, 10000);
    b.teams[1].pets[0] = Pet::new(
        &c,
        39,
        b.teams[1].pets[0].stats,
        6,
        [110, 119, 123],
        States::new(),
    )
    .unwrap();
    let (next, _) = use_ability(&b, &c, 172);
    let (_, e) = next
        .step(&c, [Action::Pass; 2], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&e), vec![24]);
}

#[test]
fn pet_battle_170_solar_beam_sunlight_conditional_bonus() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [753, 110, 123]);
    // Solar Beam753:40*6=240, additional20*6=120 if state61 Sunlight.
    assert_eq!(damage(&use_ability(&b, &c, 753).1), vec![240]);
    for team in &mut b.teams {
        for pet in &mut team.pets {
            pet.states
                .insert(c.state_id("Weather_Sunlight").unwrap(), 1);
        }
    }
    assert_eq!(damage(&use_ability(&b, &c, 753).1), vec![240, 120]);
}

#[test]
fn pet_battle_171_photosynthesis_sunlight_doubles_tick_not_team_pad() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [268, 110, 123]);
    b.teams[0].pets[0].health = 5000;
    b.teams[0].pets[1].health = 5000;
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Weather_Sunlight").unwrap(), 1);
    // Photosynthesis267 normal6*6=36 +Sunlight-only6*6=36 ->72 each round.
    b = use_ability(&b, &c, 268).0;
    assert_eq!(b.teams[0].pets[0].health, 5072);
    let (b, _) = b
        .step(&c, [Action::Swap(1), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(b.teams[0].pets[0].health, 5144);
    assert_eq!(b.teams[0].pets[1].health, 5000);
}

#[test]
fn pet_battle_28_decoy_two_team_blocks_persist_through_swap() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [334, 110, 123]);
    b.teams[0].pets[0].stats.speed = 200;
    // Decoy334/333 has state74=2: absorbs exactly two direct attacks across swaps.
    let (b, e) = b
        .step(
            &c,
            [Action::Ability(334), Action::Ability(110)],
            &mut Fixed(500000),
        )
        .unwrap();
    assert!(damage(&e).is_empty());
    let (b, e) = b
        .step(
            &c,
            [Action::Swap(1), Action::Ability(110)],
            &mut Fixed(500000),
        )
        .unwrap();
    assert!(damage(&e).is_empty());
    assert!(b.auras.is_empty());
    let (b, e) = b
        .step(&c, [Action::Pass, Action::Ability(110)], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&e), vec![120]);
    assert_eq!(b.teams[0].pets[1].health, 9880);
}

#[test]
fn pet_battle_136_sear_magic_cleans_pet_auras_not_weather() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [194, 763, 256]);
    b = use_ability(&b, &c, 256).0;
    b = use_ability(&b, &c, 194).0;
    assert_eq!(
        b.teams[0].pets[0].state(c.state_id("Mod_SpeedPercent").unwrap()),
        100
    );
    // Sear Magic763 removes all personal buffs/debuffs, not the global Darkness.
    b = use_ability(&b, &c, 763).0;
    assert_eq!(
        b.teams[0].pets[0].state(c.state_id("Mod_SpeedPercent").unwrap()),
        0
    );
    assert_eq!(
        b.teams[0].pets[0].state(c.state_id("Weather_Darkness").unwrap()),
        1
    );
}

#[test]
fn pet_battle_139_145_force_health_swap_cancels_queued_ability() {
    let c = catalog();
    for (id, slot) in [(780, 1), (457, 2)] {
        let mut b = battle(&c, 7, 7, [id, 110, 123]);
        b.teams[0].pets[0].stats.speed = 200;
        b.teams[1].pets[1].health = 2000;
        b.teams[1].pets[2].health = 8000;
        // Death Grip780 chooses lowest2000; Sweep457 highest8000 bench pet.
        // The old active pet's queued Bite must not execute on the incoming pet.
        let (b, e) = b
            .step(
                &c,
                [Action::Ability(id), Action::Ability(110)],
                &mut Fixed(500000),
            )
            .unwrap();
        assert_eq!(b.teams[1].active, slot);
        assert_eq!(b.teams[0].pets[0].health, 10000);
        assert_eq!(
            e.iter().filter(|e| matches!(e, Event::Used { .. })).count(),
            1
        );
    }
}

#[test]
fn pet_battle_68_135_explode_bypasses_failsafe() {
    let c = catalog();
    let b = battle(&c, 9, 0, [282, 110, 123]);
    // Explode282 deals40% of caster max10000=4000, then kills itself.
    // Description explicitly prevents Failsafe; no Mechanical20% resurrection.
    let (b, e) = use_ability(&b, &c, 282);
    assert_eq!(b.teams[1].pets[0].health, 6000);
    assert_eq!(b.teams[0].pets[0].health, 0);
    assert!(!e.iter().any(|e| matches!(e, Event::Revived(_))));
}

#[test]
fn pet_battle_157_belly_slide_blizzard_only_priority() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [529, 110, 123]);
    b.teams[0].pets[0].stats.speed = 1;
    // Belly Slide529 state58 gate: no Blizzard -> slow; Blizzard1 -> goes first.
    assert_eq!(use_ability(&b, &c, 529).1[0], Event::Order([1, 0]));
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Weather_Blizzard").unwrap(), 1);
    assert!(use_ability(&b, &c, 529).1.contains(&Event::Order([0, 1])));
}

#[test]
fn pet_battle_104_devour_only_heals_after_kill() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [538, 110, 123]);
    b.teams[0].pets[0].health = 6000;
    // Devour538 direct15*6=90. Kill-only heal30*6=180; living target ->0.
    assert_eq!(use_ability(&b, &c, 538).0.teams[0].pets[0].health, 6000);
    b.teams[1].pets[0].health = 80;
    assert_eq!(use_ability(&b, &c, 538).0.teams[0].pets[0].health, 6180);
}

#[test]
fn pet_battle_63_tranquility_heals_swapped_pet_using_source_power() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [254, 110, 123]);
    b.teams[0].pets[0].health = 5000;
    b.teams[0].pets[1].health = 5000;
    b.teams[0].pets[1].stats.power = 0;
    b = use_ability(&b, &c, 254).0;
    // Tranquility255 tick10*(1+.05*source100)=60, not incoming power0's10.
    let (b, _) = b
        .step(&c, [Action::Swap(1), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(b.teams[0].pets[0].health, 5060);
    assert_eq!(b.teams[0].pets[1].health, 5060);
}

#[test]
fn pet_battle_vicious_fang_miss_does_not_ramp_and_damage_flags_reset() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [156, 110, 123]);
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Stat_Accuracy").unwrap(), -100);
    b = use_ability(&b, &c, 156).0;
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Stat_Accuracy").unwrap(), 0);
    // Vicious Fang156 explicitly increases each time it HITS:miss leaves15*6=90.
    let (b, e) = use_ability(&b, &c, 156);
    assert_eq!(damage(&e), vec![90]);
    assert_eq!(
        b.teams[1].pets[0].state(c.state_id("Condition_WasDamagedThisTurn").unwrap()),
        1
    );
    let (b, _) = b.step(&c, [Action::Pass; 2], &mut Fixed(500000)).unwrap();
    assert_eq!(
        b.teams[1].pets[0].state(c.state_id("Condition_WasDamagedThisTurn").unwrap()),
        0
    );
}

fn use_ability(b: &Battle, c: &Catalog, id: u32) -> (Battle, Vec<Event>) {
    b.step(c, [Action::Ability(id), Action::Pass], &mut Fixed(500000))
        .unwrap()
}

#[test]
fn pet_battle_80_weather_all_slots_replace_expire_elemental() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [256, 595, 123]);
    b.teams[1].pets[2] = Pet::new(
        &c,
        39,
        b.teams[1].pets[2].stats,
        6,
        [110, 119, 123],
        States::new(),
    )
    .unwrap();
    let (b, _) = use_ability(&b, &c, 256);
    // Darkness257: -50% healing received, -10 accuracy, blind1 on all slots.
    for team in 0..2 {
        for slot in 0..3 {
            let pet = &b.teams[team].pets[slot];
            let elemental = team == 1 && slot == 2;
            assert_eq!(pet.state(c.state_id("Weather_Darkness").unwrap()), 1);
            assert_eq!(
                pet.state(c.state_id("Mod_HealingTakenPercent").unwrap()),
                if elemental { 0 } else { -50 }
            );
            assert_eq!(
                pet.state(c.state_id("Mechanic_IsBlind").unwrap()),
                if elemental { 0 } else { 1 }
            );
        }
    }
    let (mut b, _) = use_ability(&b, &c, 595);
    // Moonlight replaces rather than stacks with Darkness: +25% received,
    // +10% Magic damage; flags persist on bench pets through swaps.
    assert!(b.auras.iter().all(|a| a.ability != 257));
    let (next, _) = b
        .step(&c, [Action::Swap(1), Action::Pass], &mut Fixed(500000))
        .unwrap();
    b = next;
    for team in &b.teams {
        for pet in &team.pets {
            assert_eq!(pet.state(c.state_id("Weather_Darkness").unwrap()), 0);
            assert_eq!(
                pet.state(c.state_id("Mod_HealingTakenPercent").unwrap()),
                25
            );
        }
    }
    for _ in 0..8 {
        b = b.step(&c, [Action::Pass; 2], &mut Fixed(500000)).unwrap().0;
    }
    assert!(b.auras.is_empty());
    for team in &b.teams {
        for pet in &team.pets {
            assert_eq!(pet.state(c.state_id("Mod_HealingTakenPercent").unwrap()), 0);
            assert_eq!(pet.state(c.state_id("Mechanic_IsBlind").unwrap()), 0);
        }
    }
}

#[test]
fn pet_battle_24_sunlight_enemy_slots_and_80_max_health() {
    let c = catalog();
    let b = battle(&c, 7, 7, [404, 123, 110]);
    // Sunlight404 has three15-point enemy-slot attacks:15*6=90 each.
    // Aura403 gives +50% maximum HP=15000, +25% healing done to all6 pets.
    let (b, e) = use_ability(&b, &c, 404);
    assert_eq!(damage(&e), vec![90, 90, 90]);
    for pet in &b.teams[1].pets {
        assert_eq!(pet.health, 9910);
    }
    for team in &b.teams {
        for pet in &team.pets {
            assert_eq!(pet.stats.health, 15000);
        }
    }
    let mut b = b;
    b.teams[0].pets[0].health = 5000;
    let (mut b, _) = use_ability(&b, &c, 123);
    // Healing Wave123 base30*6=180; Sunlight180*1.25=225.
    assert_eq!(b.teams[0].pets[0].health, 5225);
    for _ in 0..8 {
        b = b.step(&c, [Action::Pass; 2], &mut Fixed(500000)).unwrap().0;
    }
    for team in &b.teams {
        for pet in &team.pets {
            assert_eq!(pet.stats.health, 10000);
        }
    }
}

#[test]
fn pet_battle_141_arcane_storm_splits_pool_and_weather_bonus() {
    let c = catalog();
    let b = battle(&c, 7, 7, [589, 110, 123]);
    // Arcane Storm589 pool24*6=144/3 living enemies=48 each.
    let (mut b, e) = use_ability(&b, &c, 589);
    assert_eq!(damage(&e), vec![48, 48, 48]);
    b.teams[0].pets[0].cooldowns.clear();
    // Arcane Winds state54 supplies bonus4: (24+4)*6/3=56 each.
    let (_, e) = use_ability(&b, &c, 589);
    assert_eq!(damage(&e), vec![56, 56, 56]);
    b.teams[1].pets[2].health = 0;
    // Split among TWO living pets: (24+4)*6/2=84 each (dead pet gets none).
    let (_, e) = use_ability(&b, &c, 589);
    assert_eq!(damage(&e), vec![84, 84]);
}

#[test]
fn pet_battle_29_takedown_adds_only_when_target_stunned() {
    let c = catalog();
    let mut b = battle(&c, 7, 0, [221, 110, 123]);
    // Takedown221: first18*6=108; conditional second18*6=108 only if state22.
    assert_eq!(damage(&use_ability(&b, &c, 221).1), vec![108]);
    b.teams[1].pets[0]
        .states
        .insert(c.state_id("Mechanic_IsStunned").unwrap(), 1);
    assert_eq!(damage(&use_ability(&b, &c, 221).1), vec![108, 108]);
}

#[test]
fn pet_battle_62_trample_percent_has_no_power_scaling() {
    let c = catalog();
    let b = battle(&c, 7, 7, [377, 110, 123]);
    // Trample377:10*6=60 ordinary damage +10% of target max10000=1000.
    assert_eq!(damage(&use_ability(&b, &c, 377).1), vec![60, 1000]);
}

#[test]
fn pet_battle_178_bash_resilient_reports_immune() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [348, 110, 123]);
    b.teams[0].pets[0].stats.speed = 200;
    b.teams[1].pets[0]
        .states
        .insert(c.state_id("resilient").unwrap(), 1);
    // Bash348 params state149,value1 blocks the1-round stun and reports immune.
    let (next, e) = use_ability(&b, &c, 348);
    assert!(next.auras.is_empty());
    assert!(e.iter().any(|e| matches!(e, Event::Immune { .. })));
    b.teams[1].pets[0].states.clear();
    let (_, e) = b
        .step(
            &c,
            [Action::Ability(348), Action::Ability(110)],
            &mut Fixed(500000),
        )
        .unwrap();
    assert!(e.contains(&Event::Skipped(PetRef { team: 1, slot: 0 })));
}

#[test]
fn pet_battle_384_385_tail_sweep_selects_first_or_last() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [122, 110, 123]);
    // Tail Sweep122:25*6=150 when user acts last;18*6=108 otherwise.
    b.teams[0].pets[0].stats.speed = 1;
    assert_eq!(damage(&use_ability(&b, &c, 122).1), vec![150]);
    b.teams[0].pets[0].stats.speed = 200;
    assert_eq!(damage(&use_ability(&b, &c, 122).1), vec![108]);
}

#[test]
fn pet_battle_death_coil_158_stops_failed_chain_32_heals_damage() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [121, 110, 123]);
    b.teams[0].pets[0].health = 5000;
    // Death Coil121:14*(1+.05*100)=84 neutral Undead damage;
    // property32 restores50% of the last hit =42, without scaling power again.
    let (next, e) = b
        .step(&c, [Action::Ability(121), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert_eq!(damage(&e), vec![84]);
    assert_eq!(next.teams[0].pets[0].health, 5042);
    b.teams[0].pets[0]
        .states
        .insert(c.state_id("Stat_Accuracy").unwrap(), -100);
    let (next, e) = b
        .step(&c, [Action::Ability(121), Action::Pass], &mut Fixed(500000))
        .unwrap();
    assert!(damage(&e).is_empty());
    assert_eq!(next.teams[0].pets[0].health, 5000);
    assert!(!e.iter().any(|e| matches!(e, Event::Heal { .. })));
}

#[test]
fn pet_battle_quick_attack_79_116_priority_restores_speed() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [563, 110, 123]);
    b.teams[0].pets[0].stats.speed = 1;
    b.teams[1].pets[0].stats.speed = 15000;
    // Quick Attack563 adds10000 to raw state20 to establish priority; removes10000
    // after the round.15*6=90 damage even though base speed1 is below15000.
    let (next, e) = b
        .step(
            &c,
            [Action::Ability(563), Action::Ability(110)],
            &mut Fixed(500000),
        )
        .unwrap();
    assert_eq!(e[0], Event::Order([0, 1]));
    assert_eq!(damage(&e), vec![90, 120]);
    let (_, e) = next
        .step(&c, [Action::Pass; 2], &mut Fixed(500000))
        .unwrap();
    assert_eq!(e[0], Event::Order([1, 0]));
}

#[test]
fn pet_battle_frog_kiss_27_ramps_177_blocks_resilient() {
    let c = catalog();
    let mut b = battle(&c, 7, 7, [233, 110, 123]);
    b.teams[1].pets[0]
        .states
        .insert(c.state_id("resilient").unwrap(), 1);
    // Frog Kiss233: base10 +10/use, capped30; power100 ->60,120,180,180.
    // Resilient149=1 must stop only the stun branch, not the preceding attack.
    for expected in [60, 120, 180, 180] {
        let (next, e) = b
            .step(&c, [Action::Ability(233), Action::Pass], &mut Fixed(500000))
            .unwrap();
        assert_eq!(damage(&e), vec![expected]);
        assert!(next.auras.is_empty());
        b = next;
    }
}

#[test]
fn pet_battle_22_dummy_points_are_tooltip_metadata() {
    let c = catalog();
    let b = battle(&c, 7, 7, [577, 110, 123]);
    // Healthy577 has dummy Points5 (tooltip), not a five-point heal/damage.
    // The dummy itself does not apply the aura's maxHealthBonus state.
    // Isolate it from aura admission, which requires separate stat handling.
    let mut rng = Fixed(500000);
    let mut rt = super::super::engine::Runtime {
        battle: b,
        catalog: &c,
        rng: &mut rng,
        events: Vec::new(),
        healing_target: None,
        used_ability: None,
    };
    rt.cast_turn(
        PetRef { team: 0, slot: 0 },
        PetRef { team: 1, slot: 0 },
        577,
        1,
        255,
    )
    .unwrap();
    assert!(damage(&rt.events).is_empty());
    assert_eq!(rt.battle.teams[0].pets[0].health, 10000);
}
