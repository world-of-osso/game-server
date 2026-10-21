//! Current Retail LittleWigs2777a9d; outgoing combat only, source immunity retained.
use super::*;

const ANCIENT: u32 = 196482;
const MAP: u32 = 2526;
const BIT: u32 = 1;
const BARKBREAKER: u32 = 388544;

fn ancient_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(ANCIENT, MAP, BIT, difficulty, None);
    assert_eq!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0 & 0x300,
        0x300,
        "source PC/NPC immunity remains: unlock gap, not a native clear"
    );
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    let wdt = h.app.world().resource::<MapStore>().maps[&MAP].wdt_file_data_id;
    h.app.insert_resource(crate::ground::GroundMap::new(
        &crate::ground::ground_data_dir(),
        std::collections::HashMap::from([(MAP, wdt)]),
    ));
    (h, boss, map)
}

fn cast_count(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

#[test]
fn encounter_coverage_ancient_native_barkbreaker_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = ancient_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, BARKBREAKER, Some(player), false)
            .expect("ordinary native388544");
        h.advance(3.0);
        assert_eq!(cast_count(&h, boss, BARKBREAKER), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|e| e.kind == CombatLogKind::Damage
                    && e.amount > 0
                    && e.spell_id == Some(BARKBREAKER)
                    && e.source == Some(boss.to_bits())
                    && e.target == Some(player.to_bits())),
            "native Barkbreaker damage difficulty{difficulty}"
        );
        assert!(h.has_aura(player, BARKBREAKER));
        assert!(!h.has_aura(h.players[1].0, BARKBREAKER));
    }
}

#[test]
#[ignore = "RED: current Retail Ancient Lua-clock port absent"]
fn encounter_coverage_ancient_first_barkbreaker_uses_current_retail_clock() {
    let (mut h, boss, _) = ancient_copy(205);
    h.engage(boss);
    h.advance(12.0);
    assert_eq!(
        cast_count(&h, boss, BARKBREAKER),
        1,
        "current Retail9s initial Barkbreaker"
    );
}

#[test]
fn encounter_coverage_ancient_current_lanes_preserve_immunity_and_copy_isolation() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = ancient_copy(difficulty);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(130.0);
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        eprintln!("ANCIENT difficulty{difficulty} {attempts:?}");
        let lanes = [
            (BARKBREAKER, &[9.0, 37.0, 66.0, 94.0, 123.0][..]),
            (388796, &[18.0, 51.0, 84.0, 117.0][..]),
            (388623, &[30.0, 60.0, 90.0, 120.0][..]),
            (388923, &[54.0, 109.0][..]),
        ];
        assert_eq!(attempts.len(), 15, "only current Retail native lanes");
        for (spell, expected) in lanes {
            let actual: Vec<_> = attempts.iter().filter(|a| a.spell == spell).collect();
            assert_eq!(actual.len(), expected.len(), "ongoing lane{spell}");
            for (attempt, time) in actual.iter().zip(expected) {
                assert!(attempt.result.is_ok(), "ordinary admission {attempt:?}");
                assert!(
                    (attempt.time - *time).abs() <= 0.1,
                    "source duration order {attempt:?} vs{time}"
                );
            }
            assert_eq!(
                cast_count(&h, boss, spell),
                expected.len(),
                "real native completions{spell}"
            );
        }
        assert_eq!(
            h.app.world().get::<UnitFlags>(boss).unwrap().0 & 0x300,
            0x300
        );
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|e| e.kind == CombatLogKind::Damage
                    && e.amount > 0
                    && e.spell_id == Some(388923)),
            "native Burst Forth area damage"
        );
        assert!(
            h.inbox(1)
                .spell_gos
                .iter()
                .all(|g| g.caster != boss.to_bits())
        );
        assert!(h.inbox(1).starts.is_empty());
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0,
            "no fabricated kill/unlock credit"
        );
        assert!(
            h.creatures_of(196548).is_empty(),
            "dummy Branch Out has no invented summon"
        );
    }
}

#[test]
fn encounter_coverage_ancient_evade_rearms_without_unlock_or_credit() {
    let (mut h, boss, map) = ancient_copy(205);
    h.engage(boss);
    h.advance(12.0);
    assert_eq!(cast_count(&h, boss, BARKBREAKER), 1);
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, BIT),
        Some(EncounterState::NotStarted)
    );
    h.engage(boss);
    h.advance(8.8);
    assert_eq!(
        cast_count(&h, boss, BARKBREAKER),
        1,
        "no early reset Barkbreaker"
    );
    h.advance(3.0);
    assert_eq!(cast_count(&h, boss, BARKBREAKER), 2);
    assert_eq!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0 & 0x300,
        0x300
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
}
