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
