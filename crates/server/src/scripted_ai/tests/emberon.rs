//! LittleWigs2777a9d Emberon.lua + native70448; no synthetic spell effects.
use super::*;

const EMBERON: u32 = 184422;
const MAP: u32 = 2451;
const BIT: u32 = 3;
const CLAP: u32 = 369061;
const EMBERS: u32 = 369110;
const PURGING: u32 = 368990;
const PURGING_DAMAGE: u32 = 369025;

fn emberon_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(EMBERON, MAP, BIT, difficulty, None);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

fn cast_count(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

fn damage_count(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .log
        .iter()
        .filter(|event| {
            event.kind == CombatLogKind::Damage
                && event.amount > 0
                && event.spell_id == Some(spell)
                && event.source == Some(boss.to_bits())
                && event.target == Some(h.player().to_bits())
        })
        .count()
}

#[test]
#[ignore = "encounter_coverage Emberon native qualification before source port"]
fn encounter_coverage_emberon_native_clap_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = emberon_copy(difficulty);
        h.engage(boss);
        h.advance(0.05);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, CLAP, Some(boss), false)
            .expect("ordinary native369061");
        h.advance(2.5);
        assert_eq!(cast_count(&h, boss, CLAP), 1);
        assert!(
            damage_count(&h, boss, CLAP) > 0,
            "native target104 cone difficulty{difficulty}"
        );
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|event| event.source != Some(boss.to_bits()))
        );
    }
}

#[test]
#[ignore = "encounter_coverage Emberon native qualification before source port"]
fn encounter_coverage_emberon_native_unstable_embers_aura_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = emberon_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        h.advance(0.05);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, EMBERS, Some(boss), false)
            .expect("ordinary native369110");
        h.advance(3.0);
        assert_eq!(cast_count(&h, boss, EMBERS), 1);
        assert!(
            h.has_aura(player, EMBERS),
            "native dummy aura difficulty{difficulty}"
        );
        assert!(!h.has_aura(h.players[1].0, EMBERS));
    }
}

#[test]
#[ignore = "encounter_coverage Emberon native qualification before source port"]
fn encounter_coverage_emberon_native_purging_periodic_and_indefinite_duration_qualify() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = emberon_copy(difficulty);
        h.engage(boss);
        h.advance(0.05);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, PURGING, Some(boss), false)
            .expect("ordinary native368990");
        h.advance(6.0);
        assert!(h.has_aura(boss, PURGING));
        assert!(
            h.app
                .world()
                .get::<shared::casting::CastState>(boss)
                .is_none_or(|cast| cast.spell_id != PURGING),
            "native indefinite Purging has no CastState; ordinary Clap may still cast"
        );
        let first = damage_count(&h, boss, PURGING_DAMAGE);
        assert!(
            first > 0,
            "native periodic child369025 difficulty{difficulty}"
        );
        h.advance(20.0);
        assert!(
            h.has_aura(boss, PURGING),
            "native DurationIndex21 is indefinite, not24.3s"
        );
        assert!(damage_count(&h, boss, PURGING_DAMAGE) > first);
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|event| event.source != Some(boss.to_bits()))
        );
    }
}

#[test]
#[ignore = "encounter_coverage Emberon first sourced request RED before combat implementation"]
fn encounter_coverage_emberon_first_clap_uses_current_lua_clock() {
    let (mut h, boss, _) = emberon_copy(205);
    h.engage(boss);
    h.advance(4.2);
    assert_eq!(cast_count(&h, boss, CLAP), 0, "no early Clap");
    h.advance(2.6);
    assert_eq!(
        cast_count(&h, boss, CLAP),
        1,
        "Lua4.5s request + native2s cast"
    );
    assert!(damage_count(&h, boss, CLAP) > 0);
}

#[test]
#[ignore = "encounter_coverage Emberon Normal24.3s lifecycle RED before combat implementation"]
fn encounter_coverage_emberon_normal_phase_cap_resumed_clocks_and_copy_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = emberon_copy(difficulty);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .insert_resource(super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(62.4);
        assert!(
            h.has_aura(boss, PURGING),
            "first sourced channel still active before62.8"
        );
        assert!(damage_count(&h, boss, PURGING_DAMAGE) > 0);
        h.advance(0.8);
        assert!(
            !h.has_aura(boss, PURGING),
            "EMBERON_NORMAL_PURGING_24P3S_LUA ends aura"
        );
        assert!(
            h.app
                .world()
                .get::<shared::casting::CastState>(boss)
                .is_none()
        );
        let periodic = damage_count(&h, boss, PURGING_DAMAGE);
        h.advance(35.0);
        assert_eq!(
            damage_count(&h, boss, PURGING_DAMAGE),
            periodic,
            "no channel ticks after cap"
        );
        let attempts = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        eprintln!("EMBERON difficulty{difficulty} {attempts:?}");
        let lanes = [
            (CLAP, &[4.5, 27.5, 68.2, 91.2][..]),
            (EMBERS, &[12.1, 23.8, 35.5, 64.0, 75.7, 87.4][..]),
            (PURGING, &[38.5][..]),
        ];
        assert_eq!(
            attempts.len(),
            11,
            "only source-counted current Normal lanes"
        );
        for (spell, times) in lanes {
            let actual: Vec<_> = attempts
                .iter()
                .filter(|attempt| attempt.spell == spell)
                .collect();
            assert_eq!(actual.len(), times.len(), "source lane{spell}");
            for (attempt, time) in actual.iter().zip(times) {
                assert!(
                    attempt.result.is_ok(),
                    "ordinary native admission {attempt:?}"
                );
                assert!(
                    (attempt.time - time).abs() <= 0.15,
                    "source clock {attempt:?} expected{time}"
                );
            }
        }
        assert!(damage_count(&h, boss, CLAP) > 0);
        assert!(h.has_aura(boss, 369101), "actual template addon retained");
        assert!(h.inbox(1).starts.is_empty());
        assert!(
            h.inbox(1)
                .spell_gos
                .iter()
                .all(|go| go.caster != boss.to_bits())
        );
        h.advance(9.0);
        let later = &h
            .app
            .world()
            .resource::<super::cast_admission::CastAdmissions>()
            .entries;
        let purging: Vec<_> = later
            .iter()
            .filter(|attempt| attempt.spell == PURGING)
            .collect();
        assert_eq!(purging.len(), 2, "next source37.7s phase request");
        assert!(purging[1].result.is_ok());
        assert!(
            (purging[1].time - 100.5).abs() <= 0.2,
            "native cast occupancy {purging:?}"
        );
        assert!(h.has_aura(boss, PURGING), "second native phase active");
        h.hit_pct(boss, 200.0);
        h.advance(0.1);
        assert!(!h.has_aura(boss, PURGING), "death ends second phase");
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
        assert!(h.inbox(1).ends.is_empty());
        let before = h.inbox(0).spell_gos.len();
        h.advance(30.0);
        assert_eq!(
            h.inbox(0).spell_gos.len(),
            before,
            "corpse casts stay silent"
        );
    }
}

#[test]
#[ignore = "encounter_coverage Emberon below10pct source condition RED before combat implementation"]
fn encounter_coverage_emberon_below_ten_percent_disables_purging_and_keeps_normal_lanes() {
    let (mut h, boss, _) = emberon_copy(205);
    h.engage(boss);
    h.hit_pct(boss, 91.0);
    h.advance(80.0);
    assert_eq!(
        cast_count(&h, boss, PURGING),
        0,
        "Lua UNIT_HEALTH below10% disables futurePurging"
    );
    assert_eq!(
        cast_count(&h, boss, CLAP),
        4,
        "below10% removes Clap2-count limit"
    );
    assert_eq!(
        cast_count(&h, boss, EMBERS),
        6,
        "below10% removes Embers3-count limit"
    );
    assert!(!h.has_aura(boss, PURGING));
}

#[test]
#[ignore = "encounter_coverage Emberon channel reset RED before combat implementation"]
fn encounter_coverage_emberon_evade_during_channel_cleans_phase_and_rearms_without_credit() {
    let (mut h, boss, map) = emberon_copy(205);
    h.engage(boss);
    h.advance(45.0);
    assert!(h.has_aura(boss, PURGING));
    crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
    assert!(!h.has_aura(boss, PURGING));
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, BIT),
        Some(EncounterState::NotStarted)
    );
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        0
    );
    let claps = cast_count(&h, boss, CLAP);
    h.engage(boss);
    h.advance(4.2);
    assert_eq!(cast_count(&h, boss, CLAP), claps, "no early resetClap");
    h.advance(2.6);
    assert_eq!(
        cast_count(&h, boss, CLAP),
        claps + 1,
        "initial4.5s clock rearmed"
    );
    h.advance(35.0);
    assert!(h.has_aura(boss, PURGING), "fresh reset phase reached");
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert!(!h.has_aura(boss, PURGING), "death stops native channel");
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, BIT),
        Some(EncounterState::Done)
    );
    let periodic = damage_count(&h, boss, PURGING_DAMAGE);
    h.advance(30.0);
    assert_eq!(
        damage_count(&h, boss, PURGING_DAMAGE),
        periodic,
        "no corpse channel ticks"
    );
}
