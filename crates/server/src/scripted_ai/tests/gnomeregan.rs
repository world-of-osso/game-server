//! Installed Gnomeregan SAI and pinned TC identities; controlled, not a native clear.
use super::*;

const ELECTROCUTIONER: u32 = 6235;
const MAP: u32 = 90;
const BIT: u32 = 3;
const ENCOUNTER: u32 = 380;
const SHOCK: u32 = 15605;
const MEGAVOLT: u32 = 11082;
const CHAIN_BOLT: u32 = 11085;

#[test]
fn encounter_coverage_thermaplugg_real_spells_and_isolated_credit() {
    const THERMAPLUGG: u32 = 7800;
    const BIT: u32 = 5;
    const ENCOUNTER: u32 = 382;
    let (mut h, boss, map) = bound_boss(THERMAPLUGG, MAP, BIT);
    let player = h.player();
    // Controlled survival only; preserve authored spells/resources/timers and summon behavior.
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 1_000_000.0,
        max: 1_000_000.0,
    });
    h.engage(boss);
    h.advance(14.5);
    assert_eq!(h.casts(0, boss, 10101), 0);
    assert_eq!(h.casts(0, boss, 11130), 0);
    h.advance(1.0);
    assert_eq!(h.casts(0, boss, 10101), 1);
    h.advance(24.0);
    assert_eq!(h.casts(0, boss, 11130), 0);
    h.advance(1.0);
    assert_eq!(h.casts(0, boss, 11130), 1);
    h.advance(65.0);
    for spell in [10101, 11130] {
        assert!(h.casts(0, boss, spell) >= 2, "missing repeat {spell}");
        assert_eq!(h.casts(1, boss, spell), 0);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.spell_id == Some(spell)
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0),
            "missing real damage {spell}"
        );
    }
    assert!(
        h.inbox(0)
            .starts
            .iter()
            .any(|start| start.encounter_id == ENCOUNTER)
    );
    assert!(h.inbox(1).starts.is_empty());
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, BIT),
        Some(EncounterState::Done)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1 << BIT
    );
    assert_eq!(
        h.inbox(0)
            .ends
            .iter()
            .filter(|end| end.encounter_id == ENCOUNTER && end.success)
            .count(),
        1
    );
    assert!(h.inbox(1).ends.is_empty());
}

#[test]
fn encounter_coverage_thermaplugg_normal_linked_walking_bomb_pair() {
    const THERMAPLUGG: u32 = 7800;
    const WALKING_BOMB: u32 = 7915;
    let (mut h, boss, map) = bound_boss(THERMAPLUGG, MAP, 5);
    let player = h.player();
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 1_000_000.0,
        max: 1_000_000.0,
    });
    let speech = h.app.world().resource::<CreatureTexts>().groups[&THERMAPLUGG][&2][0]
        .male_text
        .clone();
    h.engage(boss);
    h.advance(29.9);
    assert!(h.creatures_of(WALKING_BOMB).is_empty());
    h.advance(0.1);
    let bombs = h.creatures_of(WALKING_BOMB);
    assert_eq!(bombs.len(), 2, "real linked Normal-only pair must spawn");
    for (x, y, z) in [(-521.934, 693.13, -327.005), (-535.318, 649.987, -326.494)] {
        assert!(
            bombs.iter().any(|bomb| {
                let pos = h.app.world().get::<Position>(*bomb).unwrap();
                Vec3::new(pos.x - x, pos.y - z, pos.z + y).length() < 0.01
            }),
            "missing authored bomb position {x}/{y}/{z}"
        );
    }
    for bomb in bombs {
        assert_eq!(h.app.world().get::<InMap>(bomb), Some(&map));
    }
    assert!(h.inbox(0).chat.iter().any(|line| line.content == speech));
    assert!(h.inbox(1).chat.is_empty());
    assert!(
        h.inbox(0).ends.is_empty(),
        "bomb pair must not complete encounter"
    );
    assert!(h.inbox(1).ends.is_empty());
}

#[test]
fn encounter_coverage_pummeler_source_clock_damage_and_death_credit() {
    const PUMMELER: u32 = 6229;
    const PUMMELER_BIT: u32 = 4;
    const PUMMELER_ENCOUNTER: u32 = 381;
    const ARCING_SMASH: u32 = 8374;
    const TRAMPLE: u32 = 5568;
    const CROWD_PUMMEL: u32 = 10887;
    let spells = [ARCING_SMASH, TRAMPLE, CROWD_PUMMEL];
    let (mut h, boss, map) = bound_boss(PUMMELER, MAP, PUMMELER_BIT);
    let player = h.player();
    // Survival-only staging: actor, cast flags, resources and SAI chance/timers stay authored.
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 1_000_000.0,
        max: 1_000_000.0,
    });
    h.engage(boss);
    let mut cursor = 0;
    let mut casts = Vec::new();
    for tick in 1..=2200 {
        h.advance(0.05);
        let elapsed = tick as f32 * 0.05;
        let log = &h.inbox(0).log;
        casts.extend(log[cursor..].iter().filter_map(|event| {
            let spell = event.spell_id?;
            (event.source == Some(boss.to_bits())
                && event.kind == CombatLogKind::CastSuccess
                && spells.contains(&spell))
            .then_some((spell, elapsed))
        }));
        cursor = log.len();
    }
    eprintln!("PUMMELER actual successful casts at simulation seconds: {casts:?}");
    for (spell, first, repeat_min, repeat_max) in [
        (ARCING_SMASH, 15.0, 15.0, 20.0),
        (TRAMPLE, 25.0, 25.0, 35.0),
        (CROWD_PUMMEL, 40.0, 35.0, 60.0),
    ] {
        let times: Vec<_> = casts
            .iter()
            .filter(|(id, _)| *id == spell)
            .map(|(_, time)| *time)
            .collect();
        assert!(times.len() >= 2, "missing first/repeat {spell}: {times:?}");
        // Fixed50ms update: one scheduling and one observation tick of tolerance.
        assert!(
            (first - 0.1..=first + 0.1).contains(&times[0]),
            "first {spell}: {times:?}"
        );
        for pair in times.windows(2) {
            let gap = pair[1] - pair[0];
            assert!(
                (repeat_min - 0.1..=repeat_max + 0.1).contains(&gap),
                "repeat {spell}: {times:?}"
            );
        }
        let damage: Vec<_> = h
            .inbox(0)
            .log
            .iter()
            .filter(|event| {
                event.source == Some(boss.to_bits())
                    && event.spell_id == Some(spell)
                    && event.kind == CombatLogKind::Damage
                    && event.amount > 0
            })
            .collect();
        assert!(!damage.is_empty(), "no real damage {spell}");
        assert!(
            damage
                .iter()
                .all(|event| event.target == Some(player.to_bits()))
        );
        assert_eq!(h.casts(1, boss, spell), 0);
    }
    assert!(
        h.inbox(0)
            .starts
            .iter()
            .any(|start| start.encounter_id == PUMMELER_ENCOUNTER)
    );
    assert!(h.inbox(1).starts.is_empty());
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, PUMMELER_BIT),
        Some(EncounterState::Done)
    );
    assert_eq!(
        h.inbox(0)
            .ends
            .iter()
            .filter(|end| end.encounter_id == PUMMELER_ENCOUNTER && end.success)
            .count(),
        1
    );
    assert!(h.inbox(1).ends.is_empty());
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1 << PUMMELER_BIT
    );
    let count_before = casts.len();
    h.advance(2.0);
    let count_after = spells
        .iter()
        .map(|spell| h.casts(0, boss, *spell))
        .sum::<usize>();
    assert_eq!(count_after, count_before, "dead boss must not keep casting");
}

#[test]
fn encounter_coverage_electrocutioner_real_death_credit_without_resource_override() {
    let (mut h, boss, map) = bound_boss(ELECTROCUTIONER, MAP, BIT);
    h.engage(boss);
    h.advance(0.1);
    assert!(
        h.inbox(0)
            .starts
            .iter()
            .any(|start| start.encounter_id == ENCOUNTER)
    );
    assert!(h.inbox(1).starts.is_empty());
    let player = h.player();
    assert!(matches!(
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, SHOCK, Some(player), false),
        Err(crate::spell_cast::CastFailure::NotEnoughPower(
            shared::components::PowerType::Mana
        ))
    ));
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, BIT),
        Some(EncounterState::Done)
    );
    assert_eq!(
        h.inbox(0)
            .ends
            .iter()
            .filter(|end| end.encounter_id == ENCOUNTER && end.success)
            .count(),
        1
    );
    assert!(h.inbox(1).ends.is_empty());
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1 << BIT
    );
    h.advance(1.0);
    assert_eq!(
        h.inbox(0)
            .ends
            .iter()
            .filter(|end| end.encounter_id == ENCOUNTER && end.success)
            .count(),
        1
    );
}

#[test]
#[ignore = "source-data: TDB class1 has no mana; TC SpellInfo.cpp:4043-4089 / Spell.cpp:7392-7430 reject Shock"]
fn encounter_coverage_electrocutioner_real_sai_spell_acceptance() {
    let (mut h, boss, _) = bound_boss(ELECTROCUTIONER, MAP, BIT);
    let player = h.player();
    // Survival-only staging leaves the authored actor, spells, timers and cast flags intact.
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 1_000_000.0,
        max: 1_000_000.0,
    });
    h.engage(boss);
    h.advance(14.5);
    for spell in [SHOCK, MEGAVOLT, CHAIN_BOLT] {
        assert_eq!(h.casts(0, boss, spell), 0, "early cast {spell}");
    }
    h.advance(4.0);
    assert!(h.casts(0, boss, SHOCK) >= 1);
    assert_eq!(h.casts(0, boss, MEGAVOLT), 0);
    assert_eq!(h.casts(0, boss, CHAIN_BOLT), 0);
    h.advance(11.5);
    assert!(h.casts(0, boss, SHOCK) >= 2);
    assert!(h.casts(0, boss, MEGAVOLT) >= 1);
    assert_eq!(h.casts(0, boss, CHAIN_BOLT), 0);
    h.advance(15.0);
    for spell in [SHOCK, MEGAVOLT, CHAIN_BOLT] {
        assert!(h.casts(0, boss, spell) >= 2, "missing repeat {spell}");
        assert_eq!(h.casts(1, boss, spell), 0);
        let damage: Vec<_> = h
            .inbox(0)
            .log
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.source == Some(boss.to_bits())
                    && event.spell_id == Some(spell)
            })
            .collect();
        assert!(!damage.is_empty(), "no real damage {spell}");
        assert!(
            damage
                .iter()
                .all(|event| { event.target == Some(player.to_bits()) && event.amount > 0 })
        );
    }
}
