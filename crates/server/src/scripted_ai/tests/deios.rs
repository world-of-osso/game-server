//! LittleWigs2777a9d ChronoLordDeios.lua; native70448, actual source immunity.
use super::*;

const DEIOS: u32 = 184125;
const MAP: u32 = 2451;
const BIT: u32 = 4;
const WING: u32 = 376049;
const SOURCE_FLAGS: u32 = 320;

fn deios_copy(difficulty: u32) -> (Harness, Entity, InMap) {
    let (mut h, boss, map) = bound_boss_at_difficulty(DEIOS, MAP, BIT, difficulty, None);
    assert_eq!(
        h.app.world().get::<UnitFlags>(boss).unwrap().0,
        SOURCE_FLAGS
    );
    let mut visibility = crate::interest::NetworkVisibility::default();
    visibility.gain_visibility(h.players[0].1);
    h.app.world_mut().entity_mut(boss).insert(visibility);
    (h, boss, map)
}

#[test]
fn encounter_coverage_deios_native_wing_damage_qualifies_without_unlock() {
    for difficulty in [1, 205] {
        let (mut h, boss, _) = deios_copy(difficulty);
        let player = h.player();
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, WING, Some(boss), false)
            .expect("ordinary native376049; no immunity mutation");
        h.advance(3.0);
        assert!(
            h.inbox(0).log.iter().any(|event| {
                event.kind == CombatLogKind::Damage
                    && event.amount > 0
                    && event.spell_id == Some(WING)
                    && event.source == Some(boss.to_bits())
                    && event.target == Some(player.to_bits())
            }),
            "native outgoing damage difficulty{difficulty}, not engagement proof"
        );
        assert!(
            h.inbox(1)
                .log
                .iter()
                .all(|event| event.source != Some(boss.to_bits()))
        );
        assert_eq!(
            h.app.world().get::<UnitFlags>(boss).unwrap().0,
            SOURCE_FLAGS
        );
    }
}

#[test]
fn encounter_coverage_deios_source_immunity_blocks_ordinary_threat_ingress() {
    for difficulty in [1, 205] {
        let (mut h, boss, map) = deios_copy(difficulty);
        assert!(h.app.world().get::<ThreatTable>(boss).is_none());
        h.engage(boss);
        h.advance(10.0);
        assert!(
            h.app.world().get::<ThreatTable>(boss).is_none(),
            "native can_engage rejects no-threat Passive actor"
        );
        assert!(h.inbox(0).starts.is_empty());
        assert!(
            h.inbox(0)
                .spell_gos
                .iter()
                .all(|go| go.caster != boss.to_bits())
        );
        assert_eq!(
            h.app
                .world()
                .resource::<crate::maps::InstanceManager>()
                .completed_encounters(map),
            0
        );
        assert_eq!(
            h.app.world().get::<UnitFlags>(boss).unwrap().0,
            SOURCE_FLAGS
        );
    }
}

#[test]
#[ignore = "DEIOS_PC_IMMUNITY_INGRESS_UNSOURCED: no artificial engaged-state fixture"]
fn encounter_coverage_deios_first_wing_uses_current_lua_clock() {
    let (mut h, boss, _) = deios_copy(205);
    h.engage(boss);
    h.advance(9.0);
    let wings = h
        .inbox(0)
        .spell_gos
        .iter()
        .filter(|go| go.caster == boss.to_bits() && go.spell_id == WING)
        .count();
    assert_eq!(
        wings, 1,
        "current Lua6.3s first Wing + native2s cast; source ingress remains blocked"
    );
}
