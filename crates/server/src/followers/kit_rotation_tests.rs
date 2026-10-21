//! SimC76f8118 actor-owned refreshable predicates, observed on native Flame Shock.
use super::kit::{Conditions, Entry, Slot};
use super::kit_tests::at_level;
use crate::auras::{ActiveAuras, aura_now};
use crate::spell_cast::creature_cast;
use bevy::prelude::*;

const SHUJA: u32 = 209053;
const FLAME_SHOCK: u32 = 470411;
const FLAME_SHOCK_DOT: u32 = 188389;

fn refresh_entry() -> Entry {
    Entry {
        spec_id: 262,
        spell_id: FLAME_SHOCK,
        min_level: 3,
        max_level: 68,
        slot: Slot::Attack,
        conditions: serde_json::from_str::<Conditions>(
            r#"{"target":"enemy","refreshable_aura":188389}"#,
        )
        .expect("kit must accept an actor-owned refreshable predicate"),
    }
}

#[test]
fn rotation_fidelity_262_refreshes_own_flame_shock_in_pandemic_window() {
    let (mut h, caster) = at_level(SHUJA, 32, |_| {});
    let target = h.npc;
    let entry = refresh_entry();
    assert!(super::kit_ai::conditions_met(
        h.app.world_mut(),
        caster,
        target,
        &[caster],
        &[],
        &entry,
    ));
    creature_cast(h.app.world_mut(), caster, FLAME_SHOCK, Some(target), false).unwrap();
    h.advance(1);
    let aura = h
        .app
        .world()
        .get::<ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == FLAME_SHOCK_DOT && aura.caster == caster)
        .expect("ordinary native Flame Shock must apply its own periodic aura")
        .clone();
    assert!(
        !super::kit_ai::conditions_met(h.app.world_mut(), caster, target, &[caster], &[], &entry,),
        "fresh own Flame Shock is not refreshable"
    );
    while aura.expires_at.unwrap() - aura_now(h.app.world())
        > f64::from(aura.duration_ms) / 1000.0 * 0.29
    {
        h.advance(1);
    }
    assert!(crate::auras::has_aura(
        h.app.world(),
        target,
        FLAME_SHOCK_DOT
    ));
    assert!(
        super::kit_ai::conditions_met(h.app.world_mut(), caster, target, &[caster], &[], &entry,),
        "still-ticking own Flame Shock must be refreshable within 30 percent"
    );
}

#[test]
fn rotation_fidelity_262_foreign_flame_shock_is_not_own_refresh_state() {
    let (mut h, caster) = at_level(SHUJA, 32, |_| {});
    let other = crate::spell_cast::test_harness::spawn_npc(&mut h.app, 0.0);
    let target = h.npc;
    creature_cast(h.app.world_mut(), other, FLAME_SHOCK, Some(target), true).unwrap();
    h.advance(1);
    assert!(
        h.app
            .world()
            .get::<ActiveAuras>(target)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == FLAME_SHOCK_DOT && aura.caster == other)
    );
    assert!(
        super::kit_ai::conditions_met(
            h.app.world_mut(),
            caster,
            target,
            &[caster],
            &[],
            &refresh_entry(),
        ),
        "another caster's Flame Shock must not block this actor's application"
    );
}
