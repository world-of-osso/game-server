//! Native triggered sends: TC Spell.cpp3893/4779/8348; no invented visual rows.
use crate::spell_cast::test_harness::{Harness, harness};
use shared::protocol::CombatLogKind;

const BOAR_IMPACT: u32 = 471936;
const FROSTBOLT: u32 = 116;
const FROSTBOLT_DAMAGE: u32 = 228597;
const ARCANE_INTELLECT: u32 = 1459;
const MIND_BLAST: u32 = 8092;
const LIGHT_IN_THE_DARKNESS: u32 = 196415;

fn successes(h: &Harness, spell: u32) -> usize {
    h.inbox
        .log
        .iter()
        .filter(|event| event.spell_id == Some(spell) && event.kind == CombatLogKind::CastSuccess)
        .count()
}

fn sends(h: &Harness, spell: u32) -> usize {
    h.inbox
        .go
        .iter()
        .filter(|event| event.spell_id == spell)
        .count()
}

fn landed(h: &Harness, spell: u32) -> bool {
    h.inbox.log.iter().any(|event| {
        event.spell_id == Some(spell)
            && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
    })
}

#[test]
fn triggered_send_native_boar_visual_publishes_success_and_spell_go() {
    let _seed = crate::random::seed_scope(70_448);
    let mut h = harness(Vec::new(), &[BOAR_IMPACT], 3).expect("native Boar payload");
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, BOAR_IMPACT, Some(h.npc), 1);
    h.advance(6);
    assert!(
        landed(&h, BOAR_IMPACT),
        "real native payload must reach its impact boundary"
    );
    assert_eq!(
        successes(&h, BOAR_IMPACT),
        1,
        "native visual454344 requires CastSuccess"
    );
    assert_eq!(
        sends(&h, BOAR_IMPACT),
        1,
        "native visual454344 requires SpellGo"
    );
}

#[test]
fn triggered_send_native_zero_visual_payload_has_no_success_or_spell_go() {
    let _seed = crate::random::seed_scope(70_448);
    let mut h = harness(Vec::new(), &[FROSTBOLT_DAMAGE], 8).expect("native zero-visual payload");
    crate::spell_cast::cast_triggered(
        h.app.world_mut(),
        h.caster,
        FROSTBOLT_DAMAGE,
        Some(h.npc),
        1,
    );
    h.advance(6);
    assert!(
        landed(&h, FROSTBOLT_DAMAGE),
        "zero visual must not suppress the real payload"
    );
    assert_eq!(successes(&h, FROSTBOLT_DAMAGE), 0);
    assert_eq!(sends(&h, FROSTBOLT_DAMAGE), 0);
}

#[test]
fn triggered_send_native_bool_visual_publishes_once() {
    let _seed = crate::random::seed_scope(70_448);
    let mut h = harness(Vec::new(), &[BOAR_IMPACT], 3).expect("native Boar payload");
    crate::spell_cast::creature_cast(h.app.world_mut(), h.caster, BOAR_IMPACT, Some(h.npc), true)
        .expect("actual bool FULL_MASK admission");
    h.advance(6);
    assert!(landed(&h, BOAR_IMPACT));
    assert_eq!(
        successes(&h, BOAR_IMPACT),
        1,
        "existing bool path must not double-log"
    );
    assert_eq!(sends(&h, BOAR_IMPACT), 1);
}

#[test]
fn triggered_send_native_bool_zero_visual_has_no_success_or_spell_go() {
    let _seed = crate::random::seed_scope(70_448);
    let mut h = harness(Vec::new(), &[FROSTBOLT_DAMAGE], 8).expect("native zero-visual payload");
    crate::spell_cast::creature_cast(
        h.app.world_mut(),
        h.caster,
        FROSTBOLT_DAMAGE,
        Some(h.npc),
        true,
    )
    .expect("actual bool FULL_MASK admission");
    h.advance(6);
    assert!(landed(&h, FROSTBOLT_DAMAGE));
    assert_eq!(successes(&h, FROSTBOLT_DAMAGE), 0);
    assert_eq!(sends(&h, FROSTBOLT_DAMAGE), 0);
}

#[test]
fn triggered_send_native_conditional_visual_points_term_still_publishes() {
    let _seed = crate::random::seed_scope(70_448);
    let mut h =
        harness(Vec::new(), &[ARCANE_INTELLECT], 8).expect("native conditional visual spell");
    crate::spell_cast::cast_triggered(
        h.app.world_mut(),
        h.caster,
        ARCANE_INTELLECT,
        Some(h.caster),
        1,
    );
    h.advance(6);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        ARCANE_INTELLECT
    ));
    assert_eq!(
        successes(&h, ARCANE_INTELLECT),
        1,
        "native AuraPointsOnClient is independent of visual resolution"
    );
    assert_eq!(sends(&h, ARCANE_INTELLECT), 1);
}

#[test]
fn triggered_send_native_conditional_only_gap_does_not_invent_publication() {
    let _seed = crate::random::seed_scope(70_448);
    let mut h = harness(Vec::new(), &[MIND_BLAST], 5).expect("native strictly conditional visual");
    for _ in 0..2 {
        crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, MIND_BLAST, Some(h.npc), 1);
        h.advance(6);
    }
    let impacts = h
        .inbox
        .log
        .iter()
        .filter(|event| {
            event.spell_id == Some(MIND_BLAST)
                && matches!(event.kind, CombatLogKind::Damage | CombatLogKind::Miss(_))
        })
        .count();
    assert_eq!(
        impacts, 2,
        "unresolved publication does not disable the actual payload"
    );
    assert_eq!(successes(&h, MIND_BLAST), 0);
    assert_eq!(sends(&h, MIND_BLAST), 0);
}

#[test]
fn triggered_send_native_always_cast_log_overrides_conditional_visual_gap() {
    let _seed = crate::random::seed_scope(70_448);
    let mut h =
        harness(Vec::new(), &[LIGHT_IN_THE_DARKNESS], 5).expect("native AlwaysCastLog spell");
    crate::spell_cast::cast_triggered(
        h.app.world_mut(),
        h.caster,
        LIGHT_IN_THE_DARKNESS,
        Some(h.caster),
        1,
    );
    h.advance(6);
    assert!(crate::auras::has_aura(
        h.app.world(),
        h.caster,
        LIGHT_IN_THE_DARKNESS
    ));
    assert_eq!(successes(&h, LIGHT_IN_THE_DARKNESS), 1);
    assert_eq!(sends(&h, LIGHT_IN_THE_DARKNESS), 1);
}

#[test]
fn triggered_send_native_missile_publishes_once_before_impact() {
    let _seed = crate::random::seed_scope(70_448);
    let mut h = harness(Vec::new(), &[FROSTBOLT, FROSTBOLT_DAMAGE], 8).expect("native missile");
    crate::spell_cast::cast_triggered(h.app.world_mut(), h.caster, FROSTBOLT, Some(h.npc), 1);
    let mut observed_launch = false;
    for _ in 0..20 {
        h.advance(1);
        if sends(&h, FROSTBOLT) > 0 {
            assert!(
                !landed(&h, FROSTBOLT_DAMAGE),
                "launch delivery must precede native flight impact"
            );
            observed_launch = true;
            break;
        }
    }
    assert!(
        observed_launch,
        "native visual missile must publish at launch"
    );
    h.advance(30);
    assert!(landed(&h, FROSTBOLT_DAMAGE));
    assert_eq!(successes(&h, FROSTBOLT), 1);
    assert_eq!(
        sends(&h, FROSTBOLT),
        1,
        "landing must not duplicate launch publication"
    );
}
