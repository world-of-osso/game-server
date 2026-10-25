//! Lua2777a9d Granyth on canonical derived placements; projected Normal1/Follower205.
use super::*;

const GRANYTH: u32 = 186616;
const MAP: u32 = 2516;
const BIT: u32 = 3;
const SHARDS: u32 = 388817;
const STOMP: u32 = 385916;

fn projected_granyth(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(GRANYTH, MAP, BIT, difficulty, None);
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
        .expect("actual separate Nokhud copy")
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
fn encounter_coverage_granyth_projected_native_damage_and_isolation() {
    for difficulty in [1, 205] {
        for spell in [SHARDS, STOMP] {
            let (mut h, boss, _) = projected_granyth(difficulty);
            h.engage(boss);
            crate::spell_cast::creature_cast(h.app.world_mut(), boss, spell, Some(boss), false)
                .expect("native Granyth area cast");
            h.advance(5.0);
            assert_eq!(completed_casts(&h, boss, spell), 1);
            assert!(
                h.inbox(0).log.iter().any(|event| {
                    event.kind == CombatLogKind::Damage
                        && event.amount > 0
                        && event.spell_id == Some(spell)
                        && event.source == Some(boss.to_bits())
                        && event.target == Some(h.player().to_bits())
                }),
                "native{spell} projected{difficulty}"
            );
            assert!(
                h.inbox(1)
                    .log
                    .iter()
                    .all(|event| event.source != Some(boss.to_bits()))
            );
        }
    }
}

#[test]
fn encounter_coverage_granyth_projected_first_shards_source_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = projected_granyth(difficulty);
        h.engage(boss);
        h.advance(10.4);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.3);
        assert_eq!(
            h.app
                .world()
                .get::<CastState>(boss)
                .map(|cast| cast.spell_id),
            Some(SHARDS),
            "first10.6s Shards projected{difficulty}"
        );
    }
}

#[test]
fn encounter_coverage_granyth_projected_finite_lanes_credit_and_corpse() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = projected_granyth(difficulty);
        h.engage(boss);
        h.advance(40.0);
        assert_eq!(
            completed_casts(&h, boss, SHARDS),
            2,
            "10.6/23.9 sourced Shards only"
        );
        assert_eq!(
            completed_casts(&h, boss, STOMP),
            1,
            "15.5 sourced Stomp only"
        );
        assert_eq!(
            completed_casts(&h, boss, 388283),
            0,
            "no fabricated full-energy Eruption"
        );
        for spell in [386320, 386747, 386748] {
            assert_eq!(
                completed_casts(&h, boss, spell),
                0,
                "no Mythic saboteurs in1/205"
            );
        }
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|event| event.encounter_id == 2637 && event.difficulty_id == difficulty)
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
                .any(|event| event.encounter_id == 2637 && event.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        let before = h.inbox(0).spell_gos.len();
        h.advance(35.0);
        assert_eq!(h.inbox(0).spell_gos.len(), before);
    }
}

#[test]
fn encounter_coverage_granyth_projected_evade_rearms_first_shards() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = projected_granyth(difficulty);
        h.engage(boss);
        h.advance(14.0);
        assert_eq!(completed_casts(&h, boss, SHARDS), 1);
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
        h.advance(10.4);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.3);
        assert_eq!(
            h.app
                .world()
                .get::<CastState>(boss)
                .map(|cast| cast.spell_id),
            Some(SHARDS)
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
