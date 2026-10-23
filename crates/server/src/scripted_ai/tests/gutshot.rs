//! Lua2777a9d Gutshot; canonical derived Normal placement, projected1/205.
use super::*;

const GUTSHOT: u32 = 186116;
const MAP: u32 = 2520;
const BIT: u32 = 0;
const SHOT: u32 = 384353;
const TRAP: u32 = 383979;

fn projected_gutshot(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(GUTSHOT, MAP, BIT, difficulty, None);
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
    for (player, _, _) in &h.players {
        *h.app.world_mut().get_mut::<Position>(*player).unwrap() = Position {
            x: position.x + 3.0,
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

#[test]
#[ignore = "Gutshot native child damage qualification"]
fn encounter_coverage_gutshot_projected_native_shot_damage_and_isolation() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = projected_gutshot(difficulty);
        h.engage(boss);
        let player = h.player();
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, SHOT, Some(player), false)
            .expect("native Gut Shot parent");
        h.advance(3.0);
        assert_eq!(completed_casts(&h, boss, SHOT), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.spell_id == Some(384343)
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())),
            "actual384353→384343 projected{difficulty}"
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
#[ignore = "RED before native Gutshot scripting"]
fn encounter_coverage_gutshot_projected_first_shot_source_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = projected_gutshot(difficulty);
        h.engage(boss);
        h.advance(11.8);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.3);
        assert_eq!(
            h.app
                .world()
                .get::<CastState>(boss)
                .map(|cast| cast.spell_id),
            Some(SHOT),
            "12s Gut Shot projected{difficulty}"
        );
    }
}

#[test]
#[ignore = "RED before native Gutshot scripting"]
fn encounter_coverage_gutshot_projected_lanes_credit_and_corpse() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = projected_gutshot(difficulty);
        h.engage(boss);
        h.advance(80.0);
        assert_eq!(
            completed_casts(&h, boss, SHOT),
            4,
            "12/30.2/48.4/66.6 requests;15.7s repeatafter2.5ssuccess"
        );
        assert_eq!(
            completed_casts(&h, boss, TRAP),
            5,
            "8.4/25.4/42.4/59.4/76.4 trap pre-casts"
        );
        assert_eq!(
            completed_casts(&h, boss, 384416),
            0,
            "non-NormalMeatToss absent in1/205"
        );
        assert_eq!(
            completed_casts(&h, boss, 384633),
            0,
            "no invented trappedHyenaMasterCall"
        );
        assert_eq!(
            completed_casts(&h, boss, 384827),
            0,
            "no fabricated100EnergyHyena controller"
        );
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|event| event.encounter_id == 2567 && event.difficulty_id == difficulty)
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
                .any(|event| event.encounter_id == 2567 && event.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        let before = h.inbox(0).spell_gos.len();
        h.advance(50.0);
        assert_eq!(h.inbox(0).spell_gos.len(), before, "corpse silence");
    }
}

#[test]
#[ignore = "RED before native Gutshot scripting"]
fn encounter_coverage_gutshot_projected_evade_rearms_first_shot() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = projected_gutshot(difficulty);
        h.engage(boss);
        h.advance(15.0);
        assert_eq!(completed_casts(&h, boss, SHOT), 1);
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
        h.advance(11.8);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.3);
        assert_eq!(
            h.app
                .world()
                .get::<CastState>(boss)
                .map(|cast| cast.spell_id),
            Some(SHOT)
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
    }
}

#[test]
#[ignore = "RED before native Gutshot scripting"]
fn encounter_coverage_gutshot_projected_interrupted_shot_has_no_success_repeat() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = projected_gutshot(difficulty);
        h.engage(boss);
        h.advance(12.2);
        assert_eq!(
            h.app
                .world()
                .get::<CastState>(boss)
                .map(|cast| cast.spell_id),
            Some(SHOT)
        );
        crate::spell_cast::cancel_cast(h.app.world_mut(), boss);
        h.advance(40.0);
        assert_eq!(
            completed_casts(&h, boss, SHOT),
            0,
            "cancellation must not arm native success-ownedrepeat"
        );
        assert!(
            completed_casts(&h, boss, TRAP) >= 2,
            "independent trap lane continues"
        );
    }
}
