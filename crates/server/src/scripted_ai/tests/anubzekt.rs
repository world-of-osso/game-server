//! LittleWigs2777a9d AraKara/Anubzekt.lua; actual source215405, native70448.
use super::*;

const ANUBZEKT: u32 = 215405;
const MAP: u32 = 2660;
const BIT: u32 = 1;
const IMPALE: u32 = 435012;
const INFESTATION: u32 = 433740;
const BURROW: u32 = 439506;
const EYE: u32 = 433766;

fn anubzekt_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(ANUBZEKT, MAP, BIT, difficulty, None);
    assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

fn completed_casts(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

#[test]
#[ignore = "native RED: source class4 Energy pool missing at ordinary spawn"]
fn encounter_coverage_creature_class4_native_energy_spawn_spend_and_regeneration() {
    use shared::components::PowerType;
    for difficulty in [1, 205] {
        let (mut h, boss, _) = anubzekt_copy(difficulty);
        let player = h.player();
        h.app.world_mut().get_mut::<Position>(player).unwrap().x += 1000.0;
        crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
        h.advance(5.0);
        assert!(
            !h.app.world().get::<CombatStatus>(boss).unwrap().0,
            "real evade/home must finish before idle outgoing qualification"
        );
        assert!(
            h.app.world().get::<Mana>(boss).is_none(),
            "source class4 is not Mana"
        );
        assert_eq!(
            crate::spell_cast::powers::power_value(h.app.world(), boss, PowerType::Energy),
            Some((100.0, 100.0)),
            "native ChrClasses4.DisplayPower3 and70448 NPC Energy initialization"
        );
        eprintln!(
            "ROOT before difficulty{difficulty} combat={:?} motion={:?} moving={:?} dormant={} state={:?}",
            h.app.world().get::<CombatStatus>(boss),
            h.app.world().get::<CreatureMotion>(boss),
            h.app.world().get::<crate::networking::Moving>(boss),
            h.app
                .world()
                .get::<crate::grid_activity::GridDormant>(boss)
                .is_some(),
            h.app.world().get::<crate::creature_ai::CreatureState>(boss)
        );
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, EYE, Some(boss), false)
            .expect("ordinary source100Energy Eye, no scripted power grant");
        eprintln!("ROOT admitted {:?}", h.app.world().get::<CastState>(boss));
        assert_eq!(
            crate::spell_cast::powers::power_value(h.app.world(), boss, PowerType::Energy),
            Some((100.0, 100.0)),
            "ordinary native cast charges power on completion, not admission"
        );
        h.advance(7.2);
        eprintln!(
            "ROOT after difficulty{difficulty} cast={:?} energy={:?} combat={:?} moving={:?} dormant={} gos={:?}",
            h.app.world().get::<CastState>(boss),
            crate::spell_cast::powers::power_value(h.app.world(), boss, PowerType::Energy),
            h.app.world().get::<CombatStatus>(boss),
            h.app.world().get::<crate::networking::Moving>(boss),
            h.app
                .world()
                .get::<crate::grid_activity::GridDormant>(boss)
                .is_some(),
            h.inbox(0).spell_gos
        );
        assert_eq!(completed_casts(&h, boss, EYE), 1);
        assert_eq!(
            crate::spell_cast::powers::power_value(h.app.world(), boss, PowerType::Energy),
            Some((0.0, 100.0))
        );
        h.advance(2.0);
        let energy = crate::spell_cast::powers::power_value(h.app.world(), boss, PowerType::Energy)
            .unwrap()
            .0;
        assert_eq!(
            energy, 0.0,
            "actual source unit_flags2 lacks regenerate-power0x800"
        );
        assert_eq!(completed_casts(&h, boss, EYE), 1);
        assert_eq!(h.app.world().get::<UnitFlags>(boss).unwrap().0, 64);
    }
}

#[test]
#[ignore = "native70448 qualification before local combat port"]
fn encounter_coverage_anubzekt_native_infestation_periodic_damage_qualifies() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = anubzekt_copy(difficulty);
        let player = h.player();
        h.engage(boss);
        h.advance(0.1);
        let before = completed_casts(&h, boss, INFESTATION);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, INFESTATION, Some(boss), false)
            .expect("ordinary native433740 area aura");
        h.advance(2.0);
        assert_eq!(completed_casts(&h, boss, INFESTATION), before + 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.spell_id == Some(INFESTATION)
                    && event.amount > 0
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())),
            "actual500ms periodic damage difficulty{difficulty}"
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
#[ignore = "native RED: current Lua4.6s Impale, no combat script yet"]
fn encounter_coverage_anubzekt_first_impale_uses_current_lua_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = anubzekt_copy(difficulty);
        h.engage(boss);
        h.advance(4.4);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.3);
        let cast = h
            .app
            .world()
            .get::<CastState>(boss)
            .expect("source4.6s ordinary Impale request");
        assert_eq!(cast.spell_id, IMPALE);
        assert_eq!(
            cast.duration, 4.0,
            "actual Normal/Follower native cast time"
        );
        h.advance(4.1);
        assert_eq!(completed_casts(&h, boss, IMPALE), 1);
    }
}

#[test]
#[ignore = "native ongoing source lanes and isolated lifecycle before combat port"]
fn encounter_coverage_anubzekt_ongoing_lanes_native_damage_and_copy_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = anubzekt_copy(difficulty);
        let initial_infestations = completed_casts(&h, boss, INFESTATION);
        assert!(
            h.has_aura(h.player(), INFESTATION),
            "source immediate-on-pull aura is already real before fixture observer"
        );
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
        eprintln!("ANUBZEKT difficulty{difficulty} {attempts:?}");
        let lanes = [
            (
                IMPALE,
                &[
                    4.55, 13.05, 21.85, 27.65, 42.55, 51.05, 59.55, 68.05, 76.55, 85.35, 91.15,
                    99.65, 108.15, 116.95, 122.75,
                ][..],
            ),
            (
                INFESTATION,
                &[
                    10.1, 21.05, 38.95, 47.45, 55.95, 64.45, 72.95, 84.55, 95.15, 103.65, 116.15,
                    126.75,
                ][..],
            ),
            (BURROW, &[17.05, 80.55, 112.15][..]),
            (EYE, &[31.65, 112.15][..]),
        ];
        assert_eq!(
            attempts.len(),
            32,
            "all sourced requests, including ordinary second-Eye rejection"
        );
        for (spell, times) in lanes {
            let lane: Vec<_> = attempts.iter().filter(|a| a.spell == spell).collect();
            assert_eq!(lane.len(), times.len(), "source lane{spell}: {lane:?}");
            for (index, (attempt, expected)) in lane.iter().zip(times).enumerate() {
                assert!(
                    (attempt.time - expected).abs() <= 0.15,
                    "source countdown/native occupancy {attempt:?}, expected{expected}"
                );
                if spell == EYE && index == 1 {
                    assert!(
                        matches!(
                            attempt.result,
                            Err(crate::spell_cast::CastFailure::NotEnoughPower(
                                shared::components::PowerType::Energy
                            ))
                        ),
                        "source actor has no automatic regeneration; no second energy grant: {attempt:?}"
                    );
                } else {
                    assert!(
                        attempt.result.is_ok(),
                        "ordinary native admission {attempt:?}"
                    );
                }
            }
            let accepted = lane.iter().filter(|a| a.result.is_ok()).count();
            let initial = if spell == INFESTATION {
                initial_infestations
            } else {
                0
            };
            assert_eq!(completed_casts(&h, boss, spell), accepted + initial);
        }
        assert!(
            attempts
                .iter()
                .all(|a| [IMPALE, INFESTATION, BURROW, EYE].contains(&a.spell)),
            "no invented child forwarding or adds"
        );
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.spell_id == Some(INFESTATION)
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(h.player().to_bits()))
        );
        assert!(h.inbox(1).spell_gos.is_empty());
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
        let player = h.player();
        crate::combat::kill_unit(h.app.world_mut(), player, boss);
        h.advance(0.1);
        assert_eq!(
            get_boss_state(h.app.world_mut(), map, BIT),
            Some(EncounterState::Done)
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1 << BIT
        );
        assert!(
            h.inbox(0)
                .ends
                .iter()
                .any(|end| end.encounter_id == 2906 && end.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        let casts = h.inbox(0).spell_gos.len();
        h.advance(20.0);
        assert_eq!(h.inbox(0).spell_gos.len(), casts, "corpse cannot cast");
    }
}

#[test]
#[ignore = "native source reset/rearm before combat port"]
fn encounter_coverage_anubzekt_evade_rearms_initial_combo_without_credit() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = anubzekt_copy(difficulty);
        h.engage(boss);
        h.advance(45.0);
        assert!(completed_casts(&h, boss, EYE) > 0);
        let player = h.player();
        h.app.world_mut().get_mut::<Position>(player).unwrap().x += 1000.0;
        crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
        h.advance(5.0);
        assert_eq!(
            get_boss_state(h.app.world_mut(), map, BIT),
            Some(EncounterState::NotStarted)
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
        let casts = h.inbox(0).spell_gos.len();
        h.advance(40.0);
        assert_eq!(
            h.inbox(0).spell_gos.len(),
            casts,
            "evade retires all old lanes"
        );
        let mut near = *h.app.world().get::<Position>(boss).unwrap();
        near.x += 3.0;
        h.app.world_mut().entity_mut(player).insert(near);
        let before = completed_casts(&h, boss, IMPALE);
        h.engage(boss);
        h.advance(4.4);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.3);
        assert_eq!(
            h.app.world().get::<CastState>(boss).unwrap().spell_id,
            IMPALE
        );
        h.advance(4.1);
        assert_eq!(completed_casts(&h, boss, IMPALE), before + 1);
    }
}
