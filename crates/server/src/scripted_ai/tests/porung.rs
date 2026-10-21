//! TDB1210 Porung20923 SmartAI; TC a352b1fa header identity; native70448 bit0.
use super::*;

const PORUNG: u32 = 20923;
const SHATTERED_HALLS: u32 = 540;
const PORUNG_BIT: u32 = 0;

fn porung_copy(difficulty: u32) -> (Harness, InMap) {
    let (mut h, _, map) = encounter_fixture(
        PORUNG,
        SHATTERED_HALLS,
        PORUNG_BIT,
        difficulty,
        None,
        EncounterState::NotStarted,
        true,
    );
    let directory = crate::ground::ground_data_dir();
    let wdt = h.app.world().resource::<MapStore>().maps[&SHATTERED_HALLS].wdt_file_data_id;
    h.app.insert_resource(crate::ground::GroundMap::new(
        &directory,
        std::collections::HashMap::from([(SHATTERED_HALLS, wdt)]),
    ));
    h.app.insert_resource(
        crate::line_of_sight::LineOfSight::load(&directory.join("los"))
            .expect("authentic eye tables"),
    );
    (h, map)
}

fn listen_and_approach(h: &mut Harness, boss: Entity) {
    // HARNESS SETUP: proximity begins after the authentic Heroic copy is ready.
    let position = *h.app.world().get::<Position>(boss).unwrap();
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: position.x + 3.0,
        ..position
    });
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
}

#[test]
#[ignore = "RED: Porung primary20923/1935bit0 not published"]
fn encounter_coverage_porung_real_heroic_engage_starts_native_encounter() {
    let (mut h, map) = porung_copy(2);
    let bosses = h.creatures_of(PORUNG);
    assert_eq!(bosses.len(), 1, "source Heroic-only spawn34038");
    let boss = bosses[0];
    listen_and_approach(&mut h, boss);
    h.engage(boss);
    h.advance(0.2);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, PORUNG_BIT),
        Some(EncounterState::InProgress),
        "real SmartAI engage must drive1935bit0"
    );
    assert!(
        h.inbox(0)
            .starts
            .iter()
            .any(|start| start.encounter_id == 1935 && start.difficulty_id == 2)
    );
    assert!(h.inbox(1).starts.is_empty());
}

fn observe_requests(h: &mut Harness, boss: Entity) {
    let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
    h.app
        .insert_resource(crate::scripted_ai::tests::cast_admission::CastAdmissions {
            actor: boss,
            origin,
            entries: Vec::new(),
        });
}

fn cleave_times(h: &Harness) -> Vec<f32> {
    h.app
        .world()
        .resource::<crate::scripted_ai::tests::cast_admission::CastAdmissions>()
        .entries
        .iter()
        .filter(|entry| entry.spell == 15496)
        .map(|entry| entry.time)
        .collect()
}

#[test]
fn encounter_coverage_porung_authentic_inputs_and_normal_absence_qualify() {
    for difficulty in [1, 2] {
        let (mut h, _) = porung_copy(difficulty);
        assert_eq!(
            h.creatures_of(PORUNG).len(),
            usize::from(difficulty == 2),
            "actual spawn34038/2 only"
        );
        let maps = h.app.world().resource::<MapStore>();
        for (entry, bit) in [(20923, 0), (16807, 1), (16809, 2), (16808, 3)] {
            assert_eq!(
                maps.raid_creature_encounters.get(&(540, entry)),
                Some(&bit),
                "all four authenticated primaries preserved"
            );
        }
        assert!(
            !maps.raid_creature_encounters.contains_key(&(540, 17461)),
            "same-display guard is not Porung"
        );
        let events = &h
            .app
            .world()
            .resource::<crate::scripted_ai::smart_ai::SmartScripts>()
            .0[&PORUNG];
        assert_eq!(
            events.len(),
            1,
            "one actual TDB rule, no hand-written rotation"
        );
        let event = &events[0];
        assert_eq!(event.event_type, 0);
        assert_eq!(event.chance, 100);
        assert_eq!(
            event.flags, 0,
            "NOT_REPEATABLE0x1 is absent despite source comment"
        );
        assert_eq!(&event.event_params[..4], &[1900, 8000, 9000, 22000]);
        assert_eq!(
            (event.action_type, event.action_params[0], event.target_type),
            (11, 15496, 2)
        );
        assert!(h.app.world().resource::<SpellStore>().get(15496).is_some());
    }
}

#[test]
#[ignore = "pending native SmartAI cleave clock and damage proof"]
fn encounter_coverage_porung_source_cleave_repeats_at_victim_and_really_damages() {
    let (mut h, _) = porung_copy(2);
    let boss = h.creatures_of(PORUNG)[0];
    listen_and_approach(&mut h, boss);
    observe_requests(&mut h, boss);
    h.engage(boss);
    h.advance(60.0);
    let times = cleave_times(&h);
    eprintln!("PORUNG actual source cleave requests={times:?}");
    assert!(
        times.len() >= 3,
        "nonzero native repeat fields, not the stale No Repeat comment: {times:?}"
    );
    assert!(
        times[0] >= 1.89 && times[0] <= 8.15,
        "source initial1900–8000: {times:?}"
    );
    assert!(
        times
            .windows(2)
            .all(|pair| pair[1] - pair[0] >= 8.99 && pair[1] - pair[0] <= 22.1),
        "source repeat9000–22000: {times:?}"
    );
    let admissions = &h
        .app
        .world()
        .resource::<crate::scripted_ai::tests::cast_admission::CastAdmissions>()
        .entries;
    assert!(
        admissions.iter().all(|entry| entry.result.is_ok()),
        "ordinary native15496 admission"
    );
    let player = h.player();
    let gos = spell_gos(&h, boss, 15496);
    assert_eq!(
        gos.len(),
        times.len(),
        "each admitted instant cleave launches"
    );
    assert!(
        gos.iter().all(|go| go.target == Some(player.to_bits())),
        "source TARGET_VICTIM2"
    );
    assert!(
        h.inbox(0)
            .log
            .iter()
            .any(|event| event.source == Some(boss.to_bits())
                && event.target == Some(player.to_bits())
                && event.spell_id == Some(15496)
                && event.kind == CombatLogKind::Damage
                && event.amount > 0),
        "real native cleave damage"
    );
    assert!(h.inbox(1).spell_gos.is_empty());
}

#[test]
#[ignore = "pending1935bit0 real death credit and corpse silence proof"]
fn encounter_coverage_porung_death_completes_only_heroic_bit_zero_then_stops_casting() {
    let (mut h, map) = porung_copy(2);
    let boss = h.creatures_of(PORUNG)[0];
    listen_and_approach(&mut h, boss);
    h.engage(boss);
    h.advance(8.2);
    assert!(
        !spell_gos(&h, boss, 15496).is_empty(),
        "source cleave before death"
    );
    h.hit_pct(boss, 200.0);
    h.advance(0.2);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, PORUNG_BIT),
        Some(EncounterState::Done)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1,
        "bit0 only"
    );
    assert_eq!(
        h.inbox(0)
            .ends
            .iter()
            .filter(|end| end.encounter_id == 1935 && end.difficulty_id == 2 && end.success)
            .count(),
        1
    );
    for bit in [1, 2, 3] {
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, bit),
            Some(EncounterState::NotStarted)
        );
    }
    let casts = spell_gos(&h, boss, 15496).len();
    h.advance(25.0);
    assert_eq!(
        spell_gos(&h, boss, 15496).len(),
        casts,
        "dead SmartAI cannot repeat"
    );
    assert!(h.inbox(1).ends.is_empty());
}

#[test]
#[ignore = "pending same-display nonprimary exclusion proof"]
fn encounter_coverage_porung_shared_display_guard_never_completes_porung() {
    let (mut h, map) = porung_copy(2);
    let guards = h.creatures_of(17461);
    assert!(!guards.is_empty(), "actual same-display guard population");
    h.hit_pct(guards[0], 200.0);
    h.advance(0.2);
    assert_eq!(h.health(guards[0]).current, 0.0, "real17461 death");
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, PORUNG_BIT),
        Some(EncounterState::NotStarted)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
    assert!(h.inbox(0).ends.iter().all(|end| end.encounter_id != 1935));
}

#[test]
#[ignore = "pending native SmartAI Reset rearm proof"]
fn encounter_coverage_porung_real_evade_rearms_source_initial_cleave_timer() {
    let (mut h, map) = porung_copy(2);
    let boss = h.creatures_of(PORUNG)[0];
    listen_and_approach(&mut h, boss);
    observe_requests(&mut h, boss);
    h.engage(boss);
    h.advance(8.2);
    assert_eq!(
        cleave_times(&h).len(),
        1,
        "one first cleave, no repeat before10.9s"
    );
    let player = h.player();
    let origin = *h.app.world().get::<Position>(boss).unwrap();
    h.app.world_mut().entity_mut(player).insert(Position {
        x: origin.x + 500.0,
        ..origin
    });
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    for _ in 0..400 {
        h.advance(0.05);
        if h.app.world().get::<crate::creature_ai::CreatureState>(boss)
            != Some(&crate::creature_ai::CreatureState::Returning)
        {
            break;
        }
    }
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, PORUNG_BIT),
        Some(EncounterState::NotStarted)
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|end| end.encounter_id == 1935 && !end.success)
    );
    listen_and_approach(&mut h, boss);
    observe_requests(&mut h, boss);
    h.engage(boss);
    h.advance(8.2);
    let times = cleave_times(&h);
    assert_eq!(
        times.len(),
        1,
        "native Reset reinitializes source initial timer: {times:?}"
    );
    assert!(times[0] >= 1.89 && times[0] <= 8.15);
    assert_eq!(
        h.inbox(0)
            .starts
            .iter()
            .filter(|start| start.encounter_id == 1935)
            .count(),
        2
    );
}
