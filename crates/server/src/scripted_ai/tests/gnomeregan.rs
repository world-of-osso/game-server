//! Installed Electrocutioner SAI and pinned TC instance identity; controlled, not a native clear.
use super::*;

const ELECTROCUTIONER: u32 = 6235;
const MAP: u32 = 90;
const BIT: u32 = 3;
const ENCOUNTER: u32 = 380;
const SHOCK: u32 = 15605;
const MEGAVOLT: u32 = 11082;
const CHAIN_BOLT: u32 = 11085;

#[test]
fn encounter_coverage_electrocutioner_real_sai_spells_and_copy_isolated_credit() {
    let (mut h, boss, map) = bound_boss(ELECTROCUTIONER, MAP, BIT);
    let player = h.player();
    // Survival-only staging leaves the authored actor, spells, timers and cast flags intact.
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 1_000_000.0,
        max: 1_000_000.0,
    });
    h.engage(boss);
    h.advance(14.5);
    for spell in [SHOCK, MEGAVOLT, CHAIN_BOLT] {
        assert_eq!(h.casts(0, boss, spell), 0, "early cast {spell}");
    }
    h.advance(4.0);
    assert!(h.casts(0, boss, SHOCK) >= 1);
    assert_eq!(h.casts(0, boss, MEGAVOLT), 0);
    assert_eq!(h.casts(0, boss, CHAIN_BOLT), 0);
    h.advance(11.5);
    assert!(h.casts(0, boss, SHOCK) >= 2);
    assert!(h.casts(0, boss, MEGAVOLT) >= 1);
    assert_eq!(h.casts(0, boss, CHAIN_BOLT), 0);
    h.advance(15.0);
    for spell in [SHOCK, MEGAVOLT, CHAIN_BOLT] {
        assert!(h.casts(0, boss, spell) >= 2, "missing repeat {spell}");
        assert_eq!(h.casts(1, boss, spell), 0);
        let damage: Vec<_> = h
            .inbox(0)
            .log
            .iter()
            .filter(|event| {
                event.kind == CombatLogKind::Damage
                    && event.source == Some(boss.to_bits())
                    && event.spell_id == Some(spell)
            })
            .collect();
        assert!(!damage.is_empty(), "no real damage {spell}");
        assert!(
            damage
                .iter()
                .all(|event| { event.target == Some(player.to_bits()) && event.amount > 0 })
        );
    }
    assert!(h.inbox(1).starts.is_empty());
    assert!(
        h.inbox(0)
            .starts
            .iter()
            .any(|start| start.encounter_id == ENCOUNTER)
    );
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, BIT),
        Some(EncounterState::Done)
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|end| { end.encounter_id == ENCOUNTER && end.success })
    );
    assert!(h.inbox(1).ends.is_empty());
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1 << BIT
    );
}
