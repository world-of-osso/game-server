//! Lua2777a9d stage1-only partial; explicitly projected Normal1/Follower205.
use super::*;

const IRIDEUS: u32 = 189719;
const MAP: u32 = 2527;
const BIT: u32 = 0;
const FIST: u32 = 384524;
const SURGE: u32 = 384014;
const OVERLOAD: u32 = 389179;
const VOLLEY: u32 = 384351;

fn projected_irideus(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(IRIDEUS, MAP, BIT, difficulty, None);
    let maps = h.app.world().resource::<MapStore>().clone();
    let other_map = h
        .app
        .world_mut()
        .resource_mut::<crate::maps::InstanceManager>()
        .create_map(
            &maps,
            MAP,
            crate::maps::instances::InstanceEntrant {
                character_id: 2002,
                leader: 2002,
                difficulty_id: difficulty,
                ..Default::default()
            },
            crate::maps::instances::Now {
                elapsed: 0.0,
                unix: 0,
            },
        )
        .expect("actual separate Halls copy")
        .map;
    assert_ne!(map, other_map);
    let peer = h.players[1].0;
    h.app.world_mut().entity_mut(peer).insert(other_map);
    let position = *h.app.world().get::<Position>(boss).unwrap();
    let facing = h.app.world().get::<Rotation>(boss).unwrap().z;
    for (player, _, _) in &h.players {
        *h.app.world_mut().get_mut::<Position>(*player).unwrap() = Position {
            x: position.x + 3.0 * facing.cos(),
            y: position.y + 3.0 * facing.sin(),
            ..position
        };
    }
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

fn assert_casting(h: &Harness, boss: Entity, spell: u32) {
    assert_eq!(
        h.app
            .world()
            .get::<CastState>(boss)
            .map(|cast| cast.spell_id),
        Some(spell)
    );
}

#[test]
fn encounter_coverage_irideus_projected_native_fist_and_surge_damage() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = projected_irideus(difficulty);
        h.engage(boss);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, FIST, None, false)
            .expect("native cone Fist");
        h.advance(2.2);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.spell_id == Some(FIST)
                    && event.source == Some(boss.to_bits())),
            "nativeFistdamage projected{difficulty}"
        );
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, SURGE, Some(boss), false)
            .expect("native self Surge");
        h.advance(3.2);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.spell_id == Some(384015)
                    && event.source == Some(boss.to_bits())),
            "nativeSurgeperiodic384015damage projected{difficulty}"
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
fn encounter_coverage_irideus_projected_first_four_lane_clocks_and_overload_aura() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = projected_irideus(difficulty);
        h.engage(boss);
        h.advance(5.8);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.5);
        assert_casting(&h, boss, FIST);
        h.advance(4.5);
        assert_casting(&h, boss, SURGE);
        h.advance(12.6);
        assert_casting(&h, boss, FIST);
        h.advance(2.3);
        assert_casting(&h, boss, OVERLOAD);
        h.advance(1.7);
        assert!(
            h.has_aura(h.player(), OVERLOAD),
            "actual native six-second Overload aura projected{difficulty}"
        );
        h.advance(2.1);
        assert_casting(&h, boss, VOLLEY);
        h.advance(4.0);
        for spell in [FIST, SURGE, OVERLOAD, VOLLEY] {
            assert!(
                completed_casts(&h, boss, spell) >= 1,
                "actual native first lane {spell}"
            );
        }
    }
}

#[test]
fn encounter_coverage_irideus_projected_native_cooldown_rejections_and_generic_credit_copy_and_corpse()
 {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = projected_irideus(difficulty);
        let origin = h.app.world().resource::<Time>().elapsed_secs_f64();
        h.app
            .world_mut()
            .insert_resource(super::super::cast_admission::CastAdmissions {
                actor: boss,
                origin,
                entries: Vec::new(),
            });
        h.engage(boss);
        h.advance(90.0);
        assert!(
            h.health(h.player()).current > 0.0,
            "actual victim stays alive"
        );
        assert_eq!(
            [FIST, SURGE, OVERLOAD, VOLLEY].map(|spell| completed_casts(&h, boss, spell)),
            [5, 1, 3, 1],
            "native category cooldowns reject Surge/Volley repeats, not Fist/Overload"
        );
        let admissions = &h
            .app
            .world()
            .resource::<super::super::cast_admission::CastAdmissions>()
            .entries;
        for (spell, source_due) in [(SURGE, 38.5), (VOLLEY, 60.7)] {
            let attempts: Vec<_> = admissions
                .iter()
                .filter(|entry| entry.spell == spell)
                .collect();
            assert_eq!(
                attempts.len(),
                2,
                "one accepted start, one rejection, no fabricated retry"
            );
            assert!(attempts[0].result.is_ok());
            assert!(matches!(
                &attempts[1].result,
                Err(crate::spell_cast::CastFailure::OnCooldown)
            ));
            assert!(attempts[1].on_cooldown);
            assert!(
                (attempts[1].time - source_due).abs() <= 0.1,
                "source request deadline survives native rejection"
            );
        }
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|event| event.encounter_id == 2615 && event.difficulty_id == difficulty)
        );
        assert!(h.inbox(1).starts.is_empty());
        assert!(
            h.inbox(1)
                .spell_gos
                .iter()
                .all(|go| go.caster != boss.to_bits())
        );
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
        assert!(
            h.inbox(0)
                .ends
                .iter()
                .any(|event| event.encounter_id == 2615 && event.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        let before = h.inbox(0).spell_gos.len();
        h.advance(50.0);
        assert_eq!(h.inbox(0).spell_gos.len(), before, "corpse silence");
    }
}

#[test]
fn encounter_coverage_irideus_projected_fifteen_percent_stops_without_fabricated_stage2() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = projected_irideus(difficulty);
        h.engage(boss);
        h.advance(8.5);
        assert_eq!(completed_casts(&h, boss, FIST), 1);
        h.hit_pct(boss, 85.5);
        let before = [FIST, SURGE, OVERLOAD, VOLLEY].map(|spell| completed_casts(&h, boss, spell));
        h.advance(90.0);
        assert_eq!(
            [FIST, SURGE, OVERLOAD, VOLLEY].map(|spell| completed_casts(&h, boss, spell)),
            before,
            "stage1 stops below15% until missing sourceposition/barriercontroller"
        );
        for spell in [383840, 389446] {
            assert_eq!(
                completed_casts(&h, boss, spell),
                0,
                "no invented stage2 actions"
            );
        }
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, BIT),
            Some(EncounterState::InProgress)
        );
        assert!(h.inbox(0).ends.is_empty());
    }
}

#[test]
fn encounter_coverage_irideus_projected_native_home_reset_rearms_first_fist() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = projected_irideus(difficulty);
        h.engage(boss);
        h.advance(8.5);
        assert_eq!(completed_casts(&h, boss, FIST), 1);
        crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, BIT),
            Some(EncounterState::NotStarted)
        );
        let player = h.player();
        h.app.world_mut().get_mut::<Position>(player).unwrap().x += 500.0;
        for _ in 0..100 {
            if h.app.world().get::<crate::creature_ai::CreatureState>(boss)
                != Some(&crate::creature_ai::CreatureState::Returning)
            {
                break;
            }
            h.advance(0.1);
        }
        assert_ne!(
            h.app.world().get::<crate::creature_ai::CreatureState>(boss),
            Some(&crate::creature_ai::CreatureState::Returning)
        );
        let position = *h.app.world().get::<Position>(boss).unwrap();
        *h.app.world_mut().get_mut::<Position>(player).unwrap() = Position {
            x: position.x + 3.0,
            ..position
        };
        h.engage(boss);
        h.advance(5.8);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.5);
        assert_casting(&h, boss, FIST);
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
    }
}
