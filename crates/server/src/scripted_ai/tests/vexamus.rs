//! Projected Normal1/Follower205 actors from the canonical derived Vexamus placement.
//! LittleWigs2777a9d current Retail timeline: first Expulsion5s, not legacy12.1s.
use super::*;

const VEXAMUS: u32 = 194181;
const MAP: u32 = 2526;
const EXPULSION: u32 = 385958;

fn projected_vexamus(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(VEXAMUS, MAP, 0, difficulty, None);
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

#[test]
#[ignore = "RED before native Vexamus scripting"]
fn encounter_coverage_vexamus_projected_first_expulsion_current_alert_clock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = projected_vexamus(difficulty);
        h.engage(boss);
        h.advance(4.8);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.3);
        assert_eq!(
            h.app
                .world()
                .get::<CastState>(boss)
                .map(|cast| cast.spell_id),
            Some(EXPULSION),
            "projected difficulty{difficulty}: current Retail5s Expulsion"
        );
    }
}

fn completed_casts(h: &Harness, boss: Entity, spell: u32) -> usize {
    h.inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == spell)
        .count()
}

#[test]
#[ignore = "RED before Vexamus Bomb lane and ongoing encounter lifecycle"]
fn encounter_coverage_vexamus_projected_ongoing_lanes_credit_and_corpse() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = projected_vexamus(difficulty);
        h.engage(boss);
        h.advance(18.0);
        assert_eq!(
            completed_casts(&h, boss, 386173),
            1,
            "current Retail15s Bomb request"
        );
        h.advance(64.0);
        assert_eq!(
            completed_casts(&h, boss, EXPULSION),
            5,
            "5/23/41/59/77 requests + native4s casts"
        );
        assert_eq!(
            completed_casts(&h, boss, 386173),
            4,
            "15/33/51/69 requests + native2.5s casts"
        );
        assert_eq!(
            completed_casts(&h, boss, 388537),
            0,
            "no fabricated full-energy controller"
        );
        assert!(
            h.inbox(0)
                .starts
                .iter()
                .any(|event| event.encounter_id == 2562 && event.difficulty_id == difficulty)
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
            instance_script::get_boss_state(h.app.world_mut(), map, 0),
            Some(EncounterState::Done)
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            1
        );
        assert!(
            h.inbox(0)
                .ends
                .iter()
                .any(|event| event.encounter_id == 2562 && event.success)
        );
        assert!(h.inbox(1).ends.is_empty());
        let before = h.inbox(0).spell_gos.len();
        h.advance(25.0);
        assert_eq!(h.inbox(0).spell_gos.len(), before, "corpse silence");
    }
}

#[test]
#[ignore = "Vexamus reset proof pending scripting"]
fn encounter_coverage_vexamus_projected_evade_rearms_first_expulsion() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = projected_vexamus(difficulty);
        h.engage(boss);
        h.advance(10.0);
        assert_eq!(completed_casts(&h, boss, EXPULSION), 1);
        crate::creature_aggro::enter_evade_mode(h.app.world_mut(), boss);
        assert_eq!(
            instance_script::get_boss_state(h.app.world_mut(), map, 0),
            Some(EncounterState::NotStarted)
        );
        h.engage(boss);
        h.advance(4.8);
        assert!(h.app.world().get::<CastState>(boss).is_none());
        h.advance(0.3);
        assert_eq!(
            h.app
                .world()
                .get::<CastState>(boss)
                .map(|cast| cast.spell_id),
            Some(EXPULSION)
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
        assert!(h.inbox(0).ends.iter().all(|event| !event.success));
    }
}

#[test]
#[ignore = "native Vexamus input qualification"]
fn encounter_coverage_vexamus_projected_native_expulsion_damage_and_isolation() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = projected_vexamus(difficulty);
        let player = h.player();
        h.engage(boss);
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, EXPULSION, Some(player), false)
            .expect("native cone104 Expulsion");
        h.advance(4.0);
        assert!(
            h.inbox(0).log.iter().any(|event| {
                event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.spell_id == Some(EXPULSION)
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
            }),
            "native Expulsion projected difficulty{difficulty}"
        );
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|event| event.source != Some(boss.to_bits()))
        );
    }
}
