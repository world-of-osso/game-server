//! Lua2777a9d Decaystrike-only partial; explicitly projected Normal1/Follower205.
use super::*;

const WRATHEYE: u32 = 186121;
const MAP: u32 = 2520;
const BIT: u32 = 2;
const DECAYSTRIKE: u32 = 373912;

fn projected_wratheye(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(WRATHEYE, MAP, BIT, difficulty, None);
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
        .expect("actual separate Brackenhide copy")
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

fn completed_strikes(h: &Harness, boss: Entity) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == DECAYSTRIKE)
        .count()
}

#[test]
fn encounter_coverage_wratheye_projected_native_strike_damage_aura_and_isolation() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = projected_wratheye(difficulty);
        h.engage(boss);
        let player = h.player();
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, DECAYSTRIKE, Some(player), false)
            .expect("native Decaystrike parent");
        h.advance(3.2);
        assert_eq!(completed_strikes(&h, boss), 1);
        assert!(
            h.inbox(0)
                .log
                .iter()
                .any(|event| event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.spell_id == Some(373915)
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())),
            "actual373912→373915 damage projected{difficulty}"
        );
        assert!(
            h.has_aura(player, 373917),
            "native periodic373917 aura projected{difficulty}"
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
fn encounter_coverage_wratheye_projected_first_and_alternating_strike_clocks() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = projected_wratheye(difficulty);
        h.engage(boss);
        h.advance(10.3);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.5);
        assert_eq!(
            h.app
                .world()
                .get::<CastState>(boss)
                .map(|cast| cast.spell_id),
            Some(DECAYSTRIKE),
            "10.6s actualcast not373917 warning"
        );
        h.advance(18.8);
        assert_eq!(completed_strikes(&h, boss), 1);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.6);
        assert_eq!(
            h.app
                .world()
                .get::<CastState>(boss)
                .map(|cast| cast.spell_id),
            Some(DECAYSTRIKE),
            "second30s start"
        );
        h.advance(24.7);
        assert_eq!(completed_strikes(&h, boss), 2);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.9);
        assert_eq!(
            h.app
                .world()
                .get::<CastState>(boss)
                .map(|cast| cast.spell_id),
            Some(DECAYSTRIKE),
            "third55.5s start: alternating25.5s gap"
        );
        h.advance(21.8);
        assert_eq!(
            completed_strikes(&h, boss),
            4,
            "10.6/30/55.5/74.9 starts plus native2s cast"
        );
    }
}

#[test]
fn encounter_coverage_wratheye_projected_generic_lethal_credit_copy_and_corpse() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = projected_wratheye(difficulty);
        h.engage(boss);
        h.advance(13.5);
        assert_eq!(completed_strikes(&h, boss), 1);
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|event| event.encounter_id == 2569 && event.difficulty_id == difficulty)
        );
        assert!(h.inbox(1).starts.is_empty());
        assert!(
            h.inbox(1)
                .spell_gos
                .iter()
                .all(|go| go.caster != boss.to_bits())
        );
        for spell in [373942, 373944, 376170, 373960] {
            assert!(
                h.inbox(0)
                    .spell_gos
                    .iter()
                    .all(|go| go.caster != boss.to_bits() || go.spell_id != spell),
                "no fabricated excluded controller{spell}"
            );
        }
        // Generic lethal credit only. Source 5%-health finish remains unimplemented.
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
                .any(|event| event.encounter_id == 2569 && event.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        let before = h.inbox(0).spell_gos.len();
        h.advance(50.0);
        assert_eq!(h.inbox(0).spell_gos.len(), before, "corpse silence");
    }
}

#[test]
fn encounter_coverage_wratheye_projected_native_home_reset_rearms_first_strike() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = projected_wratheye(difficulty);
        h.engage(boss);
        h.advance(13.5);
        assert_eq!(completed_strikes(&h, boss), 1);
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
        h.advance(10.3);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.5);
        assert_eq!(
            h.app
                .world()
                .get::<CastState>(boss)
                .map(|cast| cast.spell_id),
            Some(DECAYSTRIKE)
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
