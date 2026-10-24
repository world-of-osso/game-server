//! Actual native application state, not a ratio of merged amounts or later stats.
use super::*;
use crate::spell_cast::test_harness::{Harness, harness, spawn_npc, spawn_player};

const RAKE: u32 = 1822;
const BLEED: u32 = 155722;
const RANK: u32 = 231052;
const STEALTH: u32 = 5215;
const REJUVENATION: u32 = 774;
const SOUL_BUFF: u32 = 114108;

fn native_actor() -> Harness {
    harness(
        Vec::new(),
        &[RAKE, BLEED, RANK, STEALTH, REJUVENATION, SOUL_BUFF, 768],
        11,
    )
    .unwrap()
}

fn bindings() -> Bindings {
    Bindings {
        actions: [("rake".into(), RAKE), ("rejuvenation".into(), REJUVENATION)].into(),
        buffs: [
            ("rake".into(), BLEED),
            ("rejuvenation".into(), REJUVENATION),
        ]
        .into(),
        dot_persistent: ["rake", "rejuvenation"]
            .into_iter()
            .map(|name| {
                (
                    name.into(),
                    DotPersistent {
                        resolve: native_dot_spell,
                        effect: 0,
                    },
                )
            })
            .collect(),
        ..Default::default()
    }
}

fn apply(h: &mut Harness, caster: Entity, target: Entity, spell: u32) {
    crate::auras::apply_aura(
        h.app.world_mut(),
        &crate::auras::AuraApplication {
            caster,
            target,
            spell_id: spell,
            effect_mask: u32::MAX,
            source: crate::auras::AuraSource::Cast,
        },
    );
    assert!(
        h.app
            .world()
            .get::<crate::auras::ActiveAuras>(target)
            .unwrap()
            .iter()
            .any(|aura| aura.spell_id == spell && aura.caster == caster),
        "actual native spell{spell} must apply before testing a fact"
    );
}

fn empower_rake(h: &mut Harness) -> f64 {
    let actor = h.caster;
    for spell in [768, RANK, STEALTH] {
        apply(h, actor, actor, spell);
    }
    let spell = h
        .app
        .world()
        .resource::<crate::spell_info::SpellStore>()
        .get(RAKE)
        .unwrap();
    let percent = spell
        .effects
        .iter()
        .find(|effect| effect.index == 3)
        .unwrap()
        .base_points;
    assert!(
        percent > 0.0,
        "native Rake effect3 supplies the existing bonus"
    );
    1.0 + f64::from(percent) / 100.0
}

fn remove_rake_bonus(h: &mut Harness) {
    crate::auras::remove_auras(
        h.app.world_mut(),
        h.caster,
        crate::auras::RemoveReason::Cancelled,
        |aura| matches!(aura.spell_id, STEALTH | RANK),
    );
}

fn facts<'a>(h: &'a Harness, b: &'a Bindings, targets: &'a [Entity]) -> NativeFacts<'a> {
    NativeFacts::new(
        h.app.world(),
        h.caster,
        b,
        Encounter::Fixture {
            started_at: crate::spell_cast::now(h.app.world()),
            duration: 300.0,
            style: FightStyle::Patchwerk,
        },
        targets.len(),
    )
    .with_targets(targets.first().copied(), targets)
}

fn stored(h: &Harness, b: &Bindings, target: Entity, alias: &str) -> Result<f64, String> {
    facts(h, b, &[target]).value(&format!("dot.{alias}.pmultiplier"))
}

fn live(h: &Harness, b: &Bindings, alias: &str) -> Result<f64, String> {
    let action = Action {
        name: alias.into(),
        target: Some(h.npc.to_bits()),
        options: Default::default(),
    };
    facts(h, b, &[h.npc]).value_for(&action, "persistent_multiplier")
}

fn amount(h: &Harness, target: Entity, spell: u32) -> f32 {
    h.app
        .world()
        .get::<crate::auras::ActiveAuras>(target)
        .unwrap()
        .iter()
        .find(|aura| aura.spell_id == spell && aura.caster == h.caster)
        .unwrap()
        .effects
        .iter()
        .find(|effect| effect.index == 0)
        .unwrap()
        .amount
}

#[test]
#[ignore = "strict native persistent-factor RED before capture/facts implementation"]
fn native_persistent_factor_stored_rake_survives_qualifier_removal_and_later_ap() {
    let mut h = native_actor();
    let b = bindings();
    let expected = empower_rake(&mut h);
    let (actor, target) = (h.caster, h.npc);
    apply(&mut h, actor, target, BLEED);
    let applied_amount = amount(&h, target, BLEED);
    assert!(applied_amount > 0.0);
    assert!((stored(&h, &b, target, "rake").unwrap() - expected).abs() < 1e-6);
    remove_rake_bonus(&mut h);
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .attack_power = 500.0;
    h.advance(1);
    assert_eq!(amount(&h, target, BLEED), applied_amount);
    assert!((stored(&h, &b, target, "rake").unwrap() - expected).abs() < 1e-6);
    assert_eq!(live(&h, &b, "rake").unwrap(), 1.0);
}

#[test]
#[ignore = "strict native persistent-factor RED before capture/facts implementation"]
fn native_persistent_factor_reapplication_captures_new_state_not_old_snapshot() {
    let mut h = native_actor();
    let b = bindings();
    let expected = empower_rake(&mut h);
    let (actor, target) = (h.caster, h.npc);
    apply(&mut h, actor, target, BLEED);
    let old_amount = amount(&h, target, BLEED);
    assert!((stored(&h, &b, target, "rake").unwrap() - expected).abs() < 1e-6);
    remove_rake_bonus(&mut h);
    h.app
        .world_mut()
        .get_mut::<crate::stats::CharacterStats>(actor)
        .unwrap()
        .attack_power = 500.0;
    apply(&mut h, actor, target, BLEED);
    assert_ne!(
        amount(&h, target, BLEED),
        old_amount,
        "new native AP really changes amount"
    );
    assert_eq!(stored(&h, &b, target, "rake").unwrap(), 1.0);
    assert_eq!(live(&h, &b, "rake").unwrap(), 1.0);
}

#[test]
#[ignore = "strict native persistent-factor RED before capture/facts implementation"]
fn native_persistent_factor_live_proposal_is_distinct_from_absent_or_stored_dot() {
    let mut h = native_actor();
    let b = bindings();
    let expected = empower_rake(&mut h);
    assert!(!crate::auras::has_aura(h.app.world(), h.npc, BLEED));
    assert!((live(&h, &b, "rake").unwrap() - expected).abs() < 1e-6);
    assert_eq!(stored(&h, &b, h.npc, "rake").unwrap(), 0.0);
    assert!(
        (facts(&h, &b, &[h.npc])
            .value("action.rake.persistent_multiplier")
            .unwrap()
            - expected)
            .abs()
            < 1e-6
    );
    assert!(
        facts(&h, &b, &[h.npc])
            .value("persistent_multiplier")
            .is_err(),
        "live query needs proposed action identity"
    );
    remove_rake_bonus(&mut h);
    assert_eq!(live(&h, &b, "rake").unwrap(), 1.0);
}

#[test]
#[ignore = "strict native persistent-factor RED before capture/facts implementation"]
fn native_persistent_factor_reads_owned_selected_target_not_another_casters_dot() {
    let mut h = native_actor();
    let b = bindings();
    let expected = empower_rake(&mut h);
    let (actor, primary) = (h.caster, h.npc);
    let secondary = spawn_npc(&mut h.app, 2.0);
    let foreign_only = spawn_npc(&mut h.app, 4.0);
    let foreign = spawn_player(&mut h.app, 11, 1.0);
    apply(&mut h, actor, primary, BLEED);
    remove_rake_bonus(&mut h);
    apply(&mut h, actor, secondary, BLEED);
    apply(&mut h, foreign, primary, BLEED);
    apply(&mut h, foreign, foreign_only, BLEED);
    let targets = [primary, secondary, foreign_only];
    let f = facts(&h, &b, &targets);
    assert!((f.value("dot.rake.pmultiplier").unwrap() - expected).abs() < 1e-6);
    for (target, expected) in [(secondary, 1.0), (foreign_only, 0.0)] {
        let action = Action {
            name: "rake".into(),
            target: Some(target.to_bits()),
            options: Default::default(),
        };
        assert_eq!(
            f.value_for(&action, "dot.rake.pmultiplier").unwrap(),
            expected
        );
    }
}

#[test]
#[ignore = "strict native persistent-factor RED before capture/facts implementation"]
fn native_persistent_factor_rake_qualifiers_do_not_empower_an_unrelated_native_hot() {
    let mut h = native_actor();
    let b = bindings();
    empower_rake(&mut h);
    let actor = h.caster;
    apply(&mut h, actor, actor, REJUVENATION);
    assert_eq!(stored(&h, &b, actor, "rejuvenation").unwrap(), 1.0);
    assert_eq!(live(&h, &b, "rejuvenation").unwrap(), 1.0);
}

#[test]
#[ignore = "strict native persistent-factor RED before capture/facts implementation"]
fn native_persistent_factor_existing_soul_hot_snapshot_is_not_feral_only() {
    let mut h = native_actor();
    let b = bindings();
    let actor = h.caster;
    apply(&mut h, actor, actor, SOUL_BUFF);
    let expected = 1.0 + f64::from(amount(&h, actor, SOUL_BUFF)) / 100.0;
    assert!(expected > 1.0);
    apply(&mut h, actor, actor, REJUVENATION);
    assert!((stored(&h, &b, actor, "rejuvenation").unwrap() - expected).abs() < 1e-6);
    crate::auras::remove_auras(
        h.app.world_mut(),
        actor,
        crate::auras::RemoveReason::Cancelled,
        |aura| aura.spell_id == SOUL_BUFF,
    );
    assert_eq!(live(&h, &b, "rejuvenation").unwrap(), 1.0);
    assert!((stored(&h, &b, actor, "rejuvenation").unwrap() - expected).abs() < 1e-6);
}

#[test]
#[ignore = "strict native persistent-factor RED before capture/facts implementation"]
fn native_persistent_factor_missing_registration_is_a_named_gap_not_a_fabricated_product() {
    let h = native_actor();
    let mut b = bindings();
    b.dot_persistent.clear();
    let error = live(&h, &b, "rake").unwrap_err();
    assert!(
        error.contains("NATIVE_PERSISTENT_FACTOR_UNAVAILABLE"),
        "{error}"
    );
    assert!(stored(&h, &b, h.npc, "not_registered").is_err());
}
