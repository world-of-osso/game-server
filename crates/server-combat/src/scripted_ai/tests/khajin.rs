//! Lua2777a9d Khajin; canonical derived Normal placement, projected1/205.
use super::*;

const KHAJIN: u32 = 189727;
const MAP: u32 = 2527;
const BIT: u32 = 2;
const SHOCK: u32 = 385963;
const HAIL: u32 = 386757;

fn projected_khajin(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(KHAJIN, MAP, BIT, difficulty, None);
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
fn encounter_coverage_khajin_projected_native_damage_and_isolation() {
    for difficulty in [1, 205] {
        for spell in [SHOCK, HAIL] {
            let (mut h, boss, _) = projected_khajin(difficulty);
            h.engage(boss);
            let player = h.player();
            crate::spell_cast::creature_cast(h.app.world_mut(), boss, spell, Some(player), false)
                .expect("native Khajin damage admission");
            // Instant Shock completes before its scheduled6s request; Hail needs
            // its native7s cast. Do not count an automatic Shock as this manual one.
            h.advance(if spell == SHOCK { 0.1 } else { 8.0 });
            assert_eq!(completed_casts(&h, boss, spell), 1);
            assert!(
                h.inbox(0)
                    .log
                    .iter()
                    .any(|event| event.kind == CombatLogKind::Damage
                        && event.amount > 0
                        && event.spell_id == Some(spell)
                        && event.source == Some(boss.to_bits())
                        && event.target == Some(player.to_bits())),
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
fn encounter_coverage_khajin_projected_first_shock_source_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = projected_khajin(difficulty);
        h.engage(boss);
        h.advance(5.8);
        assert_eq!(completed_casts(&h, boss, SHOCK), 0);
        h.advance(0.3);
        assert_eq!(
            completed_casts(&h, boss, SHOCK),
            1,
            "6s instant Shock projected{difficulty}"
        );
    }
}

#[test]
fn encounter_coverage_khajin_projected_lanes_credit_and_corpse() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = projected_khajin(difficulty);
        h.engage(boss);
        h.advance(90.0);
        for (spell, count) in [(SHOCK, 5), (HAIL, 3), (386559, 3)] {
            assert_eq!(
                completed_casts(&h, boss, spell),
                count,
                "native serialized source lane{spell}"
            );
        }
        assert_eq!(
            completed_casts(&h, boss, 390111),
            0,
            "no Mythic-only Cyclone in1/205"
        );
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|event| event.encounter_id == 2617 && event.difficulty_id == difficulty)
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
                .any(|event| event.encounter_id == 2617 && event.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        let before = h.inbox(0).spell_gos.len();
        h.advance(50.0);
        assert_eq!(h.inbox(0).spell_gos.len(), before, "corpse silence");
    }
}

#[test]
fn encounter_coverage_khajin_projected_evade_rearms_first_shock() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = projected_khajin(difficulty);
        h.engage(boss);
        h.advance(7.0);
        assert_eq!(completed_casts(&h, boss, SHOCK), 1);
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
        assert_eq!(completed_casts(&h, boss, SHOCK), 1);
        h.advance(0.3);
        assert_eq!(completed_casts(&h, boss, SHOCK), 2);
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
    }
}
