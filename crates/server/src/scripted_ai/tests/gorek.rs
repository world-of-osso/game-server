//! Lua2777a9d Gorek; canonical derived Normal placement, projected1/205.
use super::*;

const GOREK: u32 = 189478;
const MAP: u32 = 2519;
const BIT: u32 = 2;
const MIGHT: u32 = 374635;
const HEATED: u32 = 374533;

fn projected_gorek(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(GOREK, MAP, BIT, difficulty, None);
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
fn encounter_coverage_gorek_projected_native_auras_and_isolation() {
    for difficulty in [1, 205] {
        for spell in [MIGHT, HEATED] {
            let (mut h, boss, _) = projected_gorek(difficulty);
            h.engage(boss);
            let player = h.player();
            let target = if spell == MIGHT { boss } else { player };
            crate::spell_cast::creature_cast(h.app.world_mut(), boss, spell, Some(target), false)
                .expect("native Gorek aura admission");
            h.advance(if spell == MIGHT { 0.1 } else { 1.2 });
            assert_eq!(completed_casts(&h, boss, spell), 1);
            assert!(
                h.has_aura(target, spell),
                "native aura{spell} projected{difficulty}"
            );
            assert!(!h.has_aura(h.players[1].0, spell));
        }
    }
}

#[test]
fn encounter_coverage_gorek_projected_first_might_source_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = projected_gorek(difficulty);
        h.engage(boss);
        h.advance(3.4);
        assert_eq!(completed_casts(&h, boss, MIGHT), 0);
        h.advance(0.3);
        assert_eq!(
            completed_casts(&h, boss, MIGHT),
            1,
            "3.6s instant Might projected{difficulty}"
        );
    }
}

#[test]
fn encounter_coverage_gorek_projected_lanes_credit_and_corpse() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = projected_gorek(difficulty);
        h.engage(boss);
        h.advance(92.0);
        for spell in [MIGHT, HEATED, 374842, 374969] {
            assert_eq!(
                completed_casts(&h, boss, spell),
                3,
                "three sourced cycles{spell}"
            );
        }
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|event| event.encounter_id == 2612 && event.difficulty_id == difficulty)
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
                .any(|event| event.encounter_id == 2612 && event.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        let before = h.inbox(0).spell_gos.len();
        h.advance(50.0);
        assert_eq!(h.inbox(0).spell_gos.len(), before, "corpse silence");
    }
}

#[test]
fn encounter_coverage_gorek_projected_evade_rearms_first_might() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = projected_gorek(difficulty);
        h.engage(boss);
        h.advance(4.0);
        assert_eq!(completed_casts(&h, boss, MIGHT), 1);
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
        h.advance(3.4);
        assert_eq!(completed_casts(&h, boss, MIGHT), 1);
        h.advance(0.3);
        assert_eq!(completed_casts(&h, boss, MIGHT), 2);
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
    }
}
