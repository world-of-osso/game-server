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
fn encounter_coverage_electrocutioner_real_death_credit_without_resource_override() {
    let (mut h, boss, map) = bound_boss(ELECTROCUTIONER, MAP, BIT);
    h.engage(boss);
    h.advance(0.1);
    assert!(
        h.inbox(0)
            .starts
            .iter()
            .any(|start| start.encounter_id == ENCOUNTER)
    );
    assert!(h.inbox(1).starts.is_empty());
    let player = h.player();
    assert!(matches!(
        crate::spell_cast::creature_cast(h.app.world_mut(), boss, SHOCK, Some(player), false),
        Err(crate::spell_cast::CastFailure::NotEnoughPower(
            shared::components::PowerType::Mana
        ))
    ));
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, BIT),
        Some(EncounterState::Done)
    );
    assert_eq!(
        h.inbox(0)
            .ends
            .iter()
            .filter(|end| end.encounter_id == ENCOUNTER && end.success)
            .count(),
        1
    );
    assert!(h.inbox(1).ends.is_empty());
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1 << BIT
    );
    h.advance(1.0);
    assert_eq!(
        h.inbox(0)
            .ends
            .iter()
            .filter(|end| end.encounter_id == ENCOUNTER && end.success)
            .count(),
        1
    );
}

#[test]
#[ignore = "source-data: TDB class1 has no mana; TC SpellInfo.cpp:4043-4089 / Spell.cpp:7392-7430 reject Shock"]
fn encounter_coverage_electrocutioner_real_sai_spell_acceptance() {
    let (mut h, boss, _) = bound_boss(ELECTROCUTIONER, MAP, BIT);
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
}
