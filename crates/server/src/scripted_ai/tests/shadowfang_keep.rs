//! Source Silverlaine spell/lifecycle boundary; not a complete fight or native clear.
use super::*;

const SILVERLAINE: u32 = 3887;
const SHADOWFANG_MAP: u32 = 33;
const NORMAL_DIFFICULTY: u32 = 1;
const SILVERLAINE_ENCOUNTER: u32 = 1070;
const VEIL_OF_SHADOW: u32 = 7068;
const SILVERLAINE_BIT: u32 = 1;
const HEALING_POTION: u32 = 8141;
const HEAL_INPUT: f32 = 1000.0;

/// A controlled noncritical heal at the shared healing-modifier boundary.
fn apply_potion_heal(h: &mut Harness, target: Entity) -> f32 {
    let before = h.health(target).current;
    let spell = h
        .app
        .world()
        .resource::<SpellStore>()
        .get(HEALING_POTION)
        .unwrap()
        .clone();
    let context = EffectCtx {
        caster: target,
        spell: &spell,
        depth: 0,
        dest: None,
        chain_multiplier: 1.0,
    };
    crate::spell_cast::effects::apply_heal(
        h.app.world_mut(),
        &context,
        target,
        HEAL_INPUT,
        false,
        false,
    );
    h.health(target).current - before
}

#[test]
fn encounter_coverage_silverlaine_real_source_cast_repeat_and_death_credit() {
    let (mut h, boss, map) = encounter_fixture(
        SILVERLAINE,
        SHADOWFANG_MAP,
        SILVERLAINE_BIT,
        NORMAL_DIFFICULTY,
        None,
        EncounterState::NotStarted,
        false,
    );
    let boss = boss.unwrap();
    let player = h.player();
    // Isolate the source clock from player survival; actor, spell, timer and chance stay real.
    h.app.world_mut().entity_mut(player).insert(Health {
        current: 900_000.0,
        max: 1_000_000.0,
    });
    assert_eq!(apply_potion_heal(&mut h, player), HEAL_INPUT);
    h.engage(boss);
    h.advance(0.8);
    assert_eq!(h.casts(0, boss, VEIL_OF_SHADOW), 0);
    assert!(!h.has_aura(player, VEIL_OF_SHADOW));
    h.advance(3.0);
    assert_eq!(h.casts(0, boss, VEIL_OF_SHADOW), 1);
    assert!(h.has_aura(player, VEIL_OF_SHADOW));
    // Actual7068 effect0 MOD_HEALING_PCT118/-50, TC Unit::SpellHealingBonusTaken.
    assert_eq!(apply_potion_heal(&mut h, player), HEAL_INPUT * 0.5);
    assert!(!h.has_aura(h.players[1].0, VEIL_OF_SHADOW));
    h.advance(36.0);
    assert_eq!(h.casts(0, boss, VEIL_OF_SHADOW), 2);
    assert_eq!(h.casts(1, boss, VEIL_OF_SHADOW), 0);
    h.hit_pct(boss, 200.0);
    h.advance(0.1);
    assert_eq!(
        instance_script::get_boss_state(h.app.world_mut(), map, SILVERLAINE_BIT),
        Some(EncounterState::Done)
    );
    assert!(
        h.inbox(0)
            .ends
            .iter()
            .any(|end| end.encounter_id == SILVERLAINE_ENCOUNTER && end.success)
    );
    assert!(h.inbox(1).ends.is_empty());
    assert_eq!(
        h.app
            .world()
            .resource::<crate::maps::InstanceManager>()
            .completed_encounters(map),
        1 << SILVERLAINE_BIT
    );
}
